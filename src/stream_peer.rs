use std::sync::Arc;


use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::api::API;

use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;

use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;

use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;





pub struct StreamPeer {
    api: Arc<API>,
    config: RTCConfiguration,
}

impl StreamPeer {
    pub async fn new(_offer: RTCSessionDescription) -> StreamPeer {
        let mut m = MediaEngine::default();
        m.register_default_codecs().unwrap();
    

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m).unwrap();

        

        let _video_track = Arc::new(TrackLocalStaticRTP::new(
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

        let api = Arc::new(
            APIBuilder::new()
                .with_media_engine(m)
                .with_interceptor_registry(registry)
                .build(),
        );

        // Create a new RTCPeerConnection

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

        StreamPeer { api, config }
    }

    // pub async fn offer(&self, offer: RTCSessionDescription) -> Result<String> {
    //     let peer_connection = Arc::new(self.api.new_peer_connection(config).await.unwrap());

    //     peer_connection.on_ice_connection_state_change(Box::new(
    //         move |connection_state: RTCIceConnectionState| {
    //             println!("Connection State has changed {}", connection_state);
    //             if connection_state == RTCIceConnectionState::Failed {
    //                 println!("Connection to peer failed!");
    //             }
    //             Box::pin(async {})
    //         },
    //     ));

    //     // Set the handler for Peer connection state
    //     // This will notify you when the peer has connected/disconnected
    //     peer_connection.on_peer_connection_state_change(Box::new(
    //         move |s: RTCPeerConnectionState| {
    //             println!("Peer Connection State has changed: {}", s);

    //             if s == RTCPeerConnectionState::Failed {
    //                 // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
    //                 // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
    //                 // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
    //                 println!("Peer Connection has gone to failed exiting: Done forwarding");
    //             }

    //             Box::pin(async {})
    //         },
    //     ));

    //     // Set the remote SessionDescription
    //     peer_connection.set_remote_description(offer).await?;

    //     // Create an answer
    //     let answer = peer_connection.create_answer(None).await?;

    //     // Create channel that is blocked until ICE Gathering is complete
    //     let mut gather_complete = peer_connection.gathering_complete_promise().await;

    //     // Sets the LocalDescription, and starts our UDP listeners
    //     peer_connection.set_local_description(answer).await?;

    //     // Block until ICE Gathering is complete, disabling trickle ICE
    //     // we do this because we only can exchange one signaling message
    //     // in a production application you should exchange ICE Candidates via OnICECandidate
    //     let _ = gather_complete.recv().await;

    //     // Output the answer in base64 so we can paste it in browser
    //     let local_desc = peer_connection.local_description().await.unwrap();

    //     let json_str = serde_json::to_string(&local_desc)?;

    //     Ok(json_str)
    // }

    // pub fn initialize() {}
    // pub fn feed(&self, data: Vec<u8>) {}

    // pub fn is_closed(&self) -> bool {
    //     false
    // }

    // pub async fn add_stream(stream: &RtpStream) {
    //     let track = stream.new_track().await;

    //     RtpStream {
    //         ffmpeg: None,
    //         delay: false,
    //         track,
    //     }
    // }
}
