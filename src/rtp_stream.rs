use anyhow::Result;
use rtp_rs::RtpReader;
use socket2::{Domain, SockAddr, Socket, Type};
use std::fmt::format;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::NamedTempFile;
use tinytemplate::TinyTemplate;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;
use webrtc::api::media_engine::MIME_TYPE_VP8;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocalWriter;

use crate::track_feeder::{self, TrackFeeder};

use super::ffmpeg::FFmpeg;
use super::stream_peer::StreamPeer;

pub struct RtpStreamController {}

impl RtpStreamController {}

#[derive(serde::Serialize)]
struct SdpContext {
    port: u16,
    protocol: String,
}

static SDP_TEMPLATE: &str = r#"SDP TEMPLATE {port} {protocol}"#;
pub struct RtpStream {
    ffmpeg: Option<FFmpeg>,
    delay: bool,
    ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
    feeders: Arc<Mutex<Vec<TrackFeeder>>>,
    sdp_file: NamedTempFile,
}

impl RtpStream {
    pub fn new(port: u16) -> RtpStream {
        // Distribute to feeders
        let ff_packets: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let feeders: Arc<Mutex<Vec<TrackFeeder>>> = Arc::new(Mutex::new(Vec::new()));

        let ffprobe_port = get_free_port();
        println!("FFprobe UDP socket: {}", ffprobe_port);

        let sdp_file = generate_sdp_file(5000, "VP8".to_string()).unwrap();
        //.into_temp_path().to_str().unwrap().to_string()
        println!("SDP: {}", sdp_file.path().to_string_lossy());
        RtpStream::task(ff_packets.clone(), feeders.clone(), port, ffprobe_port);

        RtpStream {
            ffmpeg: None,
            delay: false,
            ff_packets,
            feeders,
            sdp_file,
        }
    }

    fn task(
        ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
        feeders: Arc<Mutex<Vec<TrackFeeder>>>,
        ffmpeg_port: u16,
        ffprobe_port: u16,
    ) {
        tokio::spawn(async move {
            // Open socket
            let sock = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], ffmpeg_port)));
            let sock = sock.await.expect("Couldn't open UDP socket!");

            // Create a socket to handle communication with an FFProbe instance
            // FFProbe is used to enable fast forwarding by finding keyframes.
            let ffprobe_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            ffprobe_sock
                .connect(SocketAddr::from(([127, 0, 0, 1], ffprobe_port)))
                .await
                .unwrap();

            // TODO: Write SDP

            // TODO: Start ffprobe reader task.

            let mut inbound_rtp_packet = vec![0u8; 1600]; // UDP MTU

            // Read packet continuously
            while let Ok((n, _)) = sock.recv_from(&mut inbound_rtp_packet).await {
                // Store packet in fast forward buffer
                ff_packets.lock().unwrap().push(inbound_rtp_packet.clone());
                ffprobe_sock.send(&inbound_rtp_packet).await;

                // Clear discarded feeders
                feeders.lock().unwrap().retain(|f| f.is_active());

                // Push packet to active feeders.
                for feeder in feeders.lock().unwrap().iter() {
                    feeder.push(inbound_rtp_packet.clone());
                }

                /** Testing code */
                let header = RtpReader::new(&inbound_rtp_packet).unwrap();
                let mut pld = header.payload().to_vec();
                pld.truncate(5);
                println!("type: {}, pld: {:?}", header.payload_type(), pld);


                //todo!("Delete old RTP packets from fast forward buffer");
            }
        });
    }

    pub fn new_with_ffmpeg() -> RtpStream {
        todo!("Implement internal ffmpeg constructor");
    }

    pub fn setup_feeder(&self, track: Arc<TrackLocalStaticRTP>) -> Arc<TrackFeeder> {
        let f = Arc::new(TrackFeeder::new(
            self.ff_packets.lock().unwrap().clone(),
            track,
        ));

        // Create new feeder

        // Add to feeder array for future pushes
        f
    }
}

fn get_free_port() -> u16 {
    let sock = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    sock.local_addr().unwrap().port()
}

// ./ffprobe lel.sdp -protocol_whitelist rtp,file,udp -show_frames

fn generate_sdp_file(port: u16, protocol: String) -> Result<NamedTempFile> {
    // Setup SDP file template
    let mut tt = TinyTemplate::new();
    tt.add_template("sdp", SDP_TEMPLATE)?;

    let context = SdpContext { port, protocol };

    let sdp_content = tt.render("sdp", &context)?;

    // Write the template to a temp file, return the path.
    let mut temp_file = tempfile::Builder::new()
        .prefix("rtp-sdp")
        .suffix(".sdp")
        .rand_bytes(5)
        .tempfile_in("")?;

    temp_file
        .as_file_mut()
        .write_all(sdp_content.as_bytes())
        .unwrap();

    Ok(temp_file)
}
