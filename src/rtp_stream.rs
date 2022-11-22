use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::UdpSocket;
use webrtc::track::track_local::TrackLocalWriter;

use webrtc::api::media_engine::MIME_TYPE_VP8;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

use super::ffmpeg::FFmpeg;
use super::webrtc_sink::WebrtcSink;

pub struct RtpStreamController {}

impl RtpStreamController {}

pub struct RtpStream {
    ffmpeg: Option<FFmpeg>,
    delay: bool,
    track: Arc<TrackLocalStaticRTP>,
}

impl RtpStream {
    pub fn new(port: u16) -> RtpStream {
        let sock = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port)));

        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                ..Default::default()
            },
            format!("video_{}", "kek"),
            "test".to_string(), // Make the stream id the passed in string
        ));

        let track_handle = track.clone();

        tokio::spawn(async move {
            let sock = sock.await.expect("Couldn't open UDP socket!");
            let mut inbound_rtp_packet = vec![0u8; 1600]; // UDP MTU
            while let Ok((n, _)) = sock.recv_from(&mut inbound_rtp_packet).await {
                track_handle
                    .write(&inbound_rtp_packet[..n])
                    .await
                    .expect("Failed to write to track!");
            }
        });

        RtpStream {
            ffmpeg: None,
            delay: false,
            track,
        }
    }

    pub fn new_with_ffmpeg() -> RtpStream {
        todo!("Implement internal ffmpeg constructor");
    }

    pub fn bind(&mut self, sink: Arc<Mutex<WebrtcSink>>) {

    }
}
