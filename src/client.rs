use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::{watch, Mutex, RwLock};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::{ice_connection_state::RTCIceConnectionState, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    rtp_transceiver::rtp_sender::RTCRtpSender,
};

use crate::{buffered_track::BufferedTrack, stream_manager::Stream, StreamDef};
struct TrackedStream {
    stream: Arc<Stream>,
    sender: Arc<RTCRtpSender>,
    buffer: Arc<BufferedTrack>,
}
pub struct Client {
    streams: Arc<RwLock<HashMap<StreamDef, TrackedStream>>>,
    peer_connection: RTCPeerConnection,
    watch_peer_status: watch::Receiver<RTCPeerConnectionState>,
    watch_failed: watch::Receiver<bool>,
    signalling: Mutex<()>,
}

impl Client {
    // TODO: Error handling
    pub async fn new() -> Result<Client> {
        // webrtc-rs boilerplate. See their examples for more info
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        let api = Arc::new(
            APIBuilder::new()
                .with_media_engine(m)
                .with_interceptor_registry(registry)
                .build(),
        );

        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                // urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create this client's peer connection
        let peer_connection = api.new_peer_connection(config).await?;

        let (watch_tx, watch_peer_status) = watch::channel(RTCPeerConnectionState::Unspecified);

        // Register handlers
        peer_connection.on_ice_connection_state_change(Box::new(
            move |connection_state: RTCIceConnectionState| {
                println!("ICE CONN STATE: {}", connection_state);
                Box::pin(async {})
            },
        ));

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected

        peer_connection.on_peer_connection_state_change(Box::new(
            move |s: RTCPeerConnectionState| {
                println!("Peer connection state: {}", s);
                watch_tx.send(s).expect("connection state send err");
                Box::pin(async {})
            },
        ));
        let (watch_failed_tx, watch_failed) = watch::channel(false);

        let c = Client {
            streams: Arc::new(RwLock::new(HashMap::new())),
            peer_connection,
            watch_peer_status,
            watch_failed,
            signalling: Mutex::new(()),
        };

        c.task_track_controller(watch_failed_tx);

        Ok(c)
    }

    // Task for asynchronously controller internal track buffers
    // based on connection state
    pub fn task_track_controller(&self, watch_failed: watch::Sender<bool>) {
        let mut ps = self.watch_peer_status.clone();
        let streams = Arc::downgrade(&self.streams);

        tokio::spawn(async move {
            loop {
                ps.changed().await?;
                let state = ps.borrow().clone();

                print!("CONNECTION STATE CHANGE: {:?}", state);
                match state {
                    RTCPeerConnectionState::Connected => {
                        let streams_arc = streams
                            .upgrade()
                            .ok_or(anyhow::Error::msg("Error upgrading streams"))?;
                        let streams_lock = streams_arc.read().await;
                        println!(" - resuming {} streams", streams_lock.len());
                        for s in streams_lock.values() {
                            s.buffer.resume().await;
                        }
                    }
                    RTCPeerConnectionState::Disconnected => {
                        println!(" - cleaning up.");
                        watch_failed.send(true).ok();
                        break;
                    }
                    s => println!("{}", s),
                }
            }
            anyhow::Ok::<()>(())
        });
    }

    pub async fn signal(&self, offer: RTCSessionDescription) -> Result<RTCSessionDescription> {
        // Holding this mutex will prevent multiple signals from happening simultaneously
        let _sig_lock = self.signalling.lock().await;

        // Set the remote SessionDescription
        self.peer_connection.set_remote_description(offer).await?;

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // Create an answer
        let answer = self.peer_connection.create_answer(None).await?;

        // Sets the LocalDescription, and starts our UDP listeners
        self.peer_connection.set_local_description(answer).await?;

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate

        // ^^ Comment from webrtc-rs example. How you're actually supposed to ICE gather IDK
        // atm. So this only works on local networks for now.
        let _ = gather_complete.recv().await;

        let r = self
            .peer_connection
            .local_description()
            .await
            .ok_or(anyhow::Error::msg("local description generation failed"))?;

        Ok(r)
    }

    /**
     * Connects this client with the passed stream.
     * Both the video and audio tracks, if applicable, are added.
     */
    pub async fn add_stream(&self, stream: Arc<Stream>) {
        println!("Adding stream");

        if let Some(ref rtp_track) = stream.video {
            println!("Creating video track");

            let buffered_track = Arc::new(BufferedTrack::new(rtp_track.clone()));

            let rtp_sender = self
                .peer_connection
                .add_track(buffered_track.track.clone())
                .await
                .expect("Failed to add track");

            let i_sender = rtp_sender.clone();

            // Read RTCP packets. Can't do anything with them ATM.
            tokio::spawn(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while i_sender.read(&mut rtcp_buf).await.is_ok() {}
            });

            // Add tracked stream
            let t = TrackedStream {
                buffer: buffered_track.clone(),
                sender: rtp_sender.clone(),
                stream: stream.clone(),
            };

            let mut s = self.streams.write().await;

            // If the client is already connected, immidiately resume the track
            if *self.watch_peer_status.borrow() == RTCPeerConnectionState::Connected {
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    buffered_track.resume().await;
                });
            }

            s.insert(stream.def.clone(), t);

            //dbg!(s.keys());
        }
    }

    /**
     * Links the track with the passed index to the passed RTP stream.
     * Switches with fast-forwarding, allowing seamless switches.
     */
    pub async fn remove_stream(&self, stream: Arc<Stream>) -> Result<()> {
        let mut s = self.streams.write().await;
        let tracked_stream = s
            .get(&stream.def)
            .ok_or(anyhow::Error::msg("Couldn't find stream to remove"))?;

        self.peer_connection
            .remove_track(&tracked_stream.sender)
            .await?;

        s.remove(&stream.def);
        Ok(())
    }

    /**
     * Cleans up after a webrtc client has disconnected.
     * Takes ownership of self so no futher calls are possible.
     */
    pub async fn discard(&self) {
        self.peer_connection.close().await.unwrap();
    }

    pub fn watch_fail(&self) -> watch::Receiver<bool> {
        self.watch_failed.clone()
    }

    pub async fn stream_ids(&self) -> HashSet<String> {
        self.streams
            .read()
            .await
            .keys()
            .clone()
            .into_iter()
            .map(|s| s.id.clone())
            .collect()
    }
}

/*
let t1 = new RtpStream(5000);

let client = Client::new(offer)
println(client.offer_response().await)
client.tracks(3); //allocate 3 tracks

client.set_track_stream(3, t1);
client.remove_track(3);

client.tracks(5)


*/
