use std::sync::Arc;

use anyhow::Result;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::Error;

pub struct WebrtcSink {
    peer_connection: Arc<RTCPeerConnection>
}

impl WebrtcSink {
    pub async fn new(offer: RTCSessionDescription) -> WebrtcSink {
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

        let video_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                ..Default::default()
            },
            format!("video_{}", "kek"),
            "test".to_string(), // Make the stream id the passed in string
        ));

        // Prepare the configuration
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create a new RTCPeerConnection
        let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

        // Add all tracks to the peer connection
        // for track in &req.state().video_tracks {
        //     // Add the track to the peerconnection...
        //     let rtp_sender = peer_connection
        //         .add_track(Arc::clone(track) as Arc<dyn TrackLocal + Send + Sync>)
        //         .await?;

        //     // ...and consume all RTCP packets so we don't overflow the RX buffer (I think....)
        //     tokio::spawn(async move {
        //         let mut rtcp_buf = vec![0u8; 1500];
        //         while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        //         Result::<()>::Ok(())
        //     });
        // }

        // Set the handler for ICE connection state
        // This will notify you when the peer has connected/disconnected
        peer_connection.on_ice_connection_state_change(Box::new(
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
        peer_connection.on_peer_connection_state_change(Box::new(
            move |s: RTCPeerConnectionState| {
                println!("Peer Connection State has changed: {}", s);

                if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    println!("Peer Connection has gone to failed exiting: Done forwarding");
                }

                Box::pin(async {})
            },
        ));

        WebrtcSink {
            peer_connection
        }
    }

    pub async fn offer(&self, offer: RTCSessionDescription) -> Result<String> {
        // Set the remote SessionDescription
        self.peer_connection.set_remote_description(offer).await?;

        // Create an answer
        let answer = self.peer_connection.create_answer(None).await?;

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // Sets the LocalDescription, and starts our UDP listeners
        self.peer_connection.set_local_description(answer).await?;

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate
        let _ = gather_complete.recv().await;

        // Output the answer in base64 so we can paste it in browser
        let local_desc = self.peer_connection.local_description().await.unwrap();

        let json_str = serde_json::to_string(&local_desc)?;

        Ok(json_str)
    }

    pub fn initialize() {}
    pub fn feed(&self, data: Vec<u8>) {}

    pub fn is_closed(&self) -> bool {
        false
    }
}
