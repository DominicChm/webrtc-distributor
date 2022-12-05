/*
    Manages peer connections
*/

use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};
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

pub struct Client {
    tracks: HashMap<TrackDef, BufferedTrack>,
    peer_connection: Arc<RTCPeerConnection>,
    assert_peer_status: Arc<tokio::sync::watch::Sender<bool>>,
    peer_status: tokio::sync::watch::Receiver<bool>,
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
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

        let (assert_peer_status, peer_status) = tokio::sync::watch::channel(false);
        let assert_peer_status = Arc::new(assert_peer_status);
        Client {
            tracks: HashMap::new(),
            peer_connection,
            peer_status,
            assert_peer_status,
        }
    }

    // TODO: Error handling
    pub async fn offer(&self, offer: RTCSessionDescription) -> RTCSessionDescription {
        self.peer_connection
            .on_ice_connection_state_change(Box::new(
                move |connection_state: RTCIceConnectionState| {
                    println!("Connection State has changed {}", connection_state);
                    if connection_state == RTCIceConnectionState::Failed {
                        println!("Connection to peer failed!");
                    }
                    Box::pin(async {})
                },
            ));

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        let a = self.assert_peer_status.clone();
        self.peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                println!("Peer Connection State has changed: {}", s);

                if s == RTCPeerConnectionState::Failed {
                    println!("Peer Connection has gone to failed exiting: Done forwarding");
                }

                if s == RTCPeerConnectionState::Connected {
                    a.send(true).unwrap();
                }

                Box::pin(async {})
            }));

        // Set the remote SessionDescription
        self.peer_connection
            .set_remote_description(offer)
            .await
            .expect("OFFER ERROR: set_remote_description");

        // Create an answer
        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .expect("OFFER ERROR: create_answer");

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // Sets the LocalDescription, and starts our UDP listeners
        self.peer_connection
            .set_local_description(answer)
            .await
            .expect("OFFER ERROR: set_local_description");

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate

        // ^^ Comment from webrtc-rs example. How you're actually supposed to ICE gather IDK
        // atm. So this only works on local networks for now.
        let _ = gather_complete.recv().await;

        self.peer_connection.local_description().await.unwrap()
    }

    /**
     * Connects this client with the passed stream.
     * Both the video and audio tracks, if applicable, are added.
     */
    pub async fn add_stream(&mut self, stream: Arc<Stream>) {
        println!("Adding stream");

        // while self.peer_status.borrow().to_owned() == false {
        //     println!("Delaying stream addition for peer connection.");
        //     self.peer_status.changed().await.unwrap();
        // }

        if let Some(rtp_track) = &stream.video {
            println!("Adding buffered track");
            let buffered_track = rtp_track.get_buffered_track();
            let rtp_sender = self
                .peer_connection
                .add_track(buffered_track.track.clone())
                .await
                .expect("Failed to add track");

            tokio::spawn(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
                Result::<()>::Ok(())
            });

            self.tracks
                .insert(rtp_track.track_def.clone(), buffered_track);
        }
    }

    /**
     * Links the track with the passed index to the passed RTP stream.
     * Switches with fast-forwarding, allowing seamless switches.
     */
    pub fn remove_stream(&mut self, stream: Stream) {
        todo!("Implement");
    }

    /**
     * Cleans up after a webrtc client has disconnected.
     * Takes ownership of self so no futher calls are possible.
     */
    pub fn discard(self) {
        todo!("Implement");
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
