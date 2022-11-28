use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::UdpSocket;
use webrtc::track::track_local::TrackLocalWriter;

use webrtc::api::media_engine::MIME_TYPE_VP8;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

use crate::track_feeder::{self, TrackFeeder};

use super::ffmpeg::FFmpeg;
use super::stream_peer::StreamPeer;

pub struct RtpStreamController {}

impl RtpStreamController {}

pub struct RtpStream {
    ffmpeg: Option<FFmpeg>,
    delay: bool,
    ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
    feeders: Arc<Mutex<Vec<TrackFeeder>>>,
}

impl RtpStream {
    pub fn new(port: u16) -> RtpStream {
        // Distribute to feeders
        let packets: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let feeders: Arc<Mutex<Vec<TrackFeeder>>> = Arc::new(Mutex::new(Vec::new()));

        let p_i = packets.clone();
        let f_i = feeders.clone();
        tokio::spawn(async move {
            // Open socket
            let sock = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port)));
            let sock = sock.await.expect("Couldn't open UDP socket!");
            let mut inbound_rtp_packet = vec![0u8; 1600]; // UDP MTU

            // Read packet continuously
            while let Ok((n, _)) = sock.recv_from(&mut inbound_rtp_packet).await {
                // Store packet
                p_i.lock().unwrap().push(inbound_rtp_packet.clone());

                // Clear discarded feeders
                f_i.lock().unwrap().retain(|f| f.is_active());

                // Push packet to non-discarded feeders.
                for feeder in f_i.lock().unwrap().iter() {
                    feeder.push(inbound_rtp_packet.clone());
                }

                todo!("Delete old RTP packets from fast forward buffer");
            }
        });

        RtpStream {
            ffmpeg: None,
            delay: false,
            ff_packets: packets,
            feeders,
        }
    }

    pub fn new_with_ffmpeg() -> RtpStream {
        todo!("Implement internal ffmpeg constructor");
    }

    pub fn setup_feeder(&self, track: Arc<TrackLocalStaticRTP>) -> Arc<Mutex<TrackFeeder>> {
        let f = Arc::new(TrackFeeder::new(
            self.ff_packets.lock().unwrap().clone(),
            track,
        ));
        // Create new feeder

        // Add to feeder array for future pushes
        f
    }
}
