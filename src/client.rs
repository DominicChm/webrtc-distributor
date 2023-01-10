use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
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

use crate::{
    buffered_track::BufferedTrack,
    stream_manager::{Stream, StreamManager},
};
struct TrackedStream {
    stream: Arc<Stream>,
    sender: Arc<RTCRtpSender>,
    buffer: Arc<BufferedTrack>,
}
pub struct Client {
    streams: Arc<RwLock<HashMap<String, TrackedStream>>>,
    stream_manager: Arc<StreamManager>,
    peer_connection: RTCPeerConnection,
    on_peer_status: watch::Receiver<RTCPeerConnectionState>,
    on_connection_failed: watch::Sender<bool>,

    /// Used to prevent simultaneous signalling. Used in "signalling" function
    signalling: Mutex<()>,
}

impl Client {
    // TODO: Error handling
    pub async fn new(stream_manager: Arc<StreamManager>) -> Result<Arc<Client>> {
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

        // // Register handlers
        // peer_connection.on_ice_connection_state_change(Box::new(
        //     move |connection_state: RTCIceConnectionState| {
        //         println!("ICE CONN STATE: {}", connection_state);
        //         Box::pin(async {})
        //     },
        // ));

        // Connect state change to a tokio watch. task_track_controller will handle side-effects.
        peer_connection.on_peer_connection_state_change(Box::new(
            move |s: RTCPeerConnectionState| {
                watch_tx.send(s).expect("connection state send err");
                Box::pin(async {})
            },
        ));
        let (watch_failed_tx, _) = watch::channel(false);

        let c = Arc::new(Client {
            streams: Arc::new(RwLock::new(HashMap::new())),
            peer_connection,
            on_peer_status: watch_peer_status,
            stream_manager,
            on_connection_failed: watch_failed_tx,
            signalling: Mutex::new(()),
        });

        Client::task_track_controller(Arc::downgrade(&c));

        Ok(c)
    }

    // Task for asynchronously controller internal track buffers
    // based on connection state
    pub fn task_track_controller(parent: Weak<Client>) {
        tokio::spawn(async move {
            let mut ps = parent.upgrade().unwrap().on_peer_status.clone();
            let streams = Arc::downgrade(&parent.upgrade().unwrap().streams);

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
                        for tracked_stream in streams_lock.values() {
                            tracked_stream.buffer.resync().await;
                        }
                    }
                    RTCPeerConnectionState::Disconnected => {
                        println!(" - cleaning up.");
                        parent.upgrade().unwrap().on_connection_failed.send(true).ok();
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

            let buffered_track = BufferedTrack::new(rtp_track.clone());

            let rtp_sender = self
                .peer_connection
                .add_track(buffered_track.rtc_track.clone())
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

            s.insert(stream.def.id.clone(), t);

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
            .get(&stream.def.id)
            .ok_or(anyhow::Error::msg("Couldn't find stream to remove"))?;

        self.peer_connection
            .remove_track(&tracked_stream.sender)
            .await?;

        s.remove(&stream.def.id);
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
        self.on_connection_failed.subscribe()
    }

    pub async fn stream_ids(&self) -> HashSet<String> {
        self.streams
            .read()
            .await
            .keys()
            .clone()
            .into_iter()
            .map(|s| s.clone())
            .collect()
    }

    pub async fn sync_active_streams(
        &self,
        stream_ids: Vec<String>,
    ) {
        let incoming_stream_set: HashSet<String> = HashSet::from_iter(stream_ids.into_iter());
        let current_stream_set: HashSet<String> = self.stream_ids().await;

        let added_stream_ids = incoming_stream_set.difference(&current_stream_set);
        dbg!(&added_stream_ids);
        for id in added_stream_ids {
            if let Some(s) = self.stream_manager.get_stream(id).await {
                println!("Sync: Adding stream {} to client", id);
                self.add_stream(s).await;
            }
        }

        let removed_stream_ids = current_stream_set.difference(&incoming_stream_set);
        dbg!(&removed_stream_ids);
        for id in removed_stream_ids {
            if let Some(s) = self.stream_manager.get_stream(id).await {
                println!("Sync: Removing stream {} from client", id);
                self.remove_stream(s).await;
            }
        }
    }

    /**
     * Re-starts the internal buffered track to force a GOP re-sync on the client.
     */
    pub async fn resync_rtp_streams(&self, stream_ids: Vec<String>) {
        for id in stream_ids {
            self.resync_stream(id).await;
        }
    }

    /**
     * Re-starts the internal buffered track to force a GOP re-sync on the client.
     */
    pub async fn resync_stream(&self, stream_id: String) {
        // Only perform a re-sync if connected.
        if self.peer_connection.connection_state() == RTCPeerConnectionState::Connected {
            let streams = self.streams.read().await;
            streams.get(&stream_id).unwrap().buffer.resync().await;
        }
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
