/*
    Manages peer connections
*/

use anyhow::Result;
use bytes::buf;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::{
    select,
    sync::{broadcast, watch, Mutex, RwLock},
};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_VP8},
        APIBuilder, API,
    },
    ice_transport::{ice_connection_state::RTCIceConnectionState, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    rtp_transceiver::rtp_sender::RTCRtpSender,
};

use crate::{buffered_track::BufferedTrack, stream_manager::Stream, StreamDef, TrackDef};

#[derive(Clone)]
struct TrackedStream {
    stream: Arc<Stream>,
    sender: Arc<RTCRtpSender>,
    buffer: Arc<BufferedTrack>,
}
pub struct Client {
    streams: Arc<RwLock<HashMap<StreamDef, TrackedStream>>>,
    peer_connection: Arc<RTCPeerConnection>,
    watch_peer_status: watch::Receiver<RTCIceConnectionState>,
    watch_failed: watch::Receiver<bool>,
    offer_response: RwLock<Option<RTCSessionDescription>>,
}

impl Client {
    // TODO: Error handling
    pub async fn new() -> Client {
        // webrtc-rs boilerplate. See their examples for more info
        let mut m = MediaEngine::default();
        m.register_default_codecs().unwrap();

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m).unwrap();

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
        let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

        let (watch_tx, watch_peer_status) = watch::channel(RTCIceConnectionState::Unspecified);

        // Register handlers
        peer_connection.on_ice_connection_state_change(Box::new(
            move |connection_state: RTCIceConnectionState| {
                watch_tx.send(connection_state).unwrap();
                Box::pin(async {})
            },
        ));

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected

        // peer_connection.on_peer_connection_state_change(Box::new(
        //     move |s: RTCPeerConnectionState| {
        //         println!("Peer connection state: {}", s);
        //         Box::pin(async {})
        //     },
        // ));
        let (watch_failed_tx, watch_failed) = watch::channel(false);

        let c = Client {
            streams: Arc::new(RwLock::new(HashMap::new())),
            peer_connection,
            watch_peer_status,
            watch_failed,
            offer_response: RwLock::new(None),
        };

        c.task_track_controller(watch_failed_tx);

        c
    }

    // Task for asynchronously controller internal track buffers
    // based on connection state
    pub fn task_track_controller(&self, watch_failed: watch::Sender<bool>) {
        let mut ps = self.watch_peer_status.clone();
        let streams = self.streams.clone();

        tokio::spawn(async move {
            loop {
                ps.changed().await.unwrap();
                let state = ps.borrow().clone();
                print!("CONNECTION STATE CHANGE: {:?}", state);
                match state {
                    RTCIceConnectionState::Connected => {
                        let streams_lock = streams.read().await;
                        println!(" - resuming {} streams", streams_lock.len());
                        for s in streams_lock.values() {
                            s.buffer.resume().await;
                        }
                    }
                    RTCIceConnectionState::Disconnected => {
                        let streams_lock = streams.read().await;
                        println!(" - pausing {} streams", streams_lock.len());

                        for s in streams_lock.values() {
                            s.buffer.pause().await;
                        }
                    }
                    RTCIceConnectionState::Failed => {
                        println!(" - cleaning up.");
                        watch_failed.send(true);
                        break;
                    }
                    s => println!(""),
                }
            }
        });
    }
    // TODO: Error handling
    pub async fn signal(&self, offer: RTCSessionDescription) -> RTCSessionDescription {
        //println!("Signalling");

        // Set the remote SessionDescription
        self.peer_connection
            .set_remote_description(offer)
            .await
            .expect("SIGNAL ERROR: set_remote_description");

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // Create an answer
        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .expect("SIGNAL ERROR: create_answer");

        // Sets the LocalDescription, and starts our UDP listeners
        self.peer_connection
            .set_local_description(answer)
            .await
            .expect("SIGNAL ERROR: set_local_description");

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate

        // ^^ Comment from webrtc-rs example. How you're actually supposed to ICE gather IDK
        // atm. So this only works on local networks for now.
        let _ = gather_complete.recv().await;

        let r = self.peer_connection.local_description().await.unwrap();

        r
    }

    /**
     * Connects this client with the passed stream.
     * Both the video and audio tracks, if applicable, are added.
     */
    pub async fn add_stream(&self, stream: Arc<Stream>) {
        println!("Adding stream");

        //TODO: Check if stream already exists.

        if let Some(ref rtp_track) = stream.video {
            println!("Creating video track");

            let buffered_track = Arc::new(BufferedTrack::new(rtp_track.clone()));
            let rtp_sender = self
                .peer_connection
                .add_track(buffered_track.track.clone())
                .await
                .expect("Failed to add track");

            let i_sender = rtp_sender.clone();
            tokio::spawn(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while let Ok((_, _)) = i_sender.read(&mut rtcp_buf).await {}
                println!("RTCP RXer FAILED");
                Result::<()>::Ok(())
            });

            // Add tracked stream
            let t = TrackedStream {
                buffer: buffered_track.clone(),
                sender: rtp_sender.clone(),
                stream: stream.clone(),
            };

            let mut s = self.streams.write().await;
            s.insert(stream.def.clone(), t);
        }
    }

    /**
     * Links the track with the passed index to the passed RTP stream.
     * Switches with fast-forwarding, allowing seamless switches.
     */
    pub fn remove_stream(&mut self, stream: StreamDef) {
        todo!("Implement");
    }

    /**
     * Cleans up after a webrtc client has disconnected.
     * Takes ownership of self so no futher calls are possible.
     */
    pub fn discard(self) {
        todo!("Implement");
    }

    pub fn watch_fail(&self) -> watch::Receiver<bool> {
        self.watch_failed.clone()
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
