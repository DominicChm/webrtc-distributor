use anyhow::Result;
use bytes::Buf;
use rtp_rs::RtpReader;
use socket2::{Domain, Protocol, Socket, Type};
use std::fmt::format;
use std::future;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread::{self, yield_now};
use tempfile::{tempdir, NamedTempFile};
use tinytemplate::TinyTemplate;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UdpSocket;
use tokio::process::{ChildStdout, Command};
use tokio::select;
use tokio::time::{sleep, Sleep};
use webrtc::api::media_engine::MIME_TYPE_VP8;
use webrtc::interceptor::twcc::receiver::Receiver;
use webrtc::rtcp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::sdp::SessionDescription;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::util::Unmarshal;

use crate::net_util::listen_udp;
use crate::track_feeder::{self, TrackFeeder};

use super::ffmpeg::FFmpeg;
use super::stream_peer::StreamPeer;

pub struct RtpStream {
    delay: bool,
    ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
    feeders: Arc<Mutex<Vec<TrackFeeder>>>,
}

impl RtpStream {
    pub fn new(sdp: String) -> RtpStream {
        // Distribute to feeders
        let ff_packets: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let feeders: Arc<Mutex<Vec<TrackFeeder>>> = Arc::new(Mutex::new(Vec::new()));

        RtpStream::task(ff_packets.clone(), feeders.clone(), sdp);

        RtpStream {
            delay: false,
            ff_packets,
            feeders,
        }
    }

    fn task(
        ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
        feeders: Arc<Mutex<Vec<TrackFeeder>>>,
        sdp: String,
    ) {
        tokio::spawn(async move {
            // Open socket

            // Initialize the ffmpeg remuxer.
            // remuxer_sdp.media_descriptions[0]

            //RtpStream::init_ffprobe(sdp).await;
            // RTP_stream
            // 1. Init with raw SDP
            // 2. Parse protocol, port, address
            // 3. Connect to RTP port with reuseaddr enabled
            // 4. Init FFProbe instance
            //  a. Create UDP socket
            //  b. Start ffprobe on that socket
            //  c. Pass SDP through SAP
            // 5. Start parsing FFProbe, and storing RTP packets.

            // Init ffprobe once we're bound with a REUSEADDR port.

            let (remuxer_addr, mut ffprobe, codec) = tokio::join!(
                RtpStream::init_remuxer(sdp),
                RtpStream::ffprobe_frames(),
                RtpStream::ffprobe_codec()
            );

            println!("FOUND CODEC: {}", codec);
            //let remuxer_addr = "239.7.69.7:5002".parse::<SocketAddr>().unwrap();
            println!("Binding to remuxer at {}", remuxer_addr);
            let remuxer_sock = listen_udp(&remuxer_addr).unwrap();
            let remuxer_sock = UdpSocket::from_std(remuxer_sock).unwrap();

            let mut buf = vec![0u8; 1600];
            let mut probe_line = String::new();
            loop {
                select! {
                    biased;

                    n = ffprobe.read_line(&mut probe_line) => {
                        println!("FRAME: {}", probe_line);
                    }
                    Ok((n, origin)) = remuxer_sock.recv_from(&mut buf) => {
                        let mut trimmed = buf[..n].to_vec();
                        let mut b: &[u8] = &trimmed;

                        let pkt = webrtc::rtp::packet::Packet::unmarshal(&mut b).unwrap();

                        // Store packet in fast forward buffer
                        ff_packets.lock().unwrap().push(trimmed.clone());

                        // Clear discarded feeders
                        feeders.lock().unwrap().retain(|f| f.is_active());

                        // Push packet to active feeders.
                        for feeder in feeders.lock().unwrap().iter() {
                            feeder.push(trimmed.clone());
                        }

                    },

                }
                //let packet_descriptor = ffprobe.

                /* Testing code */
                //let header = RtpReader::new(&inbound_rtp_packet).unwrap();
                //let mut pld = header.payload().to_vec();
                //pld.truncate(5);
                //println!("source: type: {}, pld: {:?}", header.payload_type(), pld);

                //todo!("Delete old RTP packets from fast forward buffer");
            }
            //println!("FELL THROUGH LOOP");
        });
    }

    // ./ffprobe -f sap sap://224.2.127.254 -show_frames -show_entries frame=key_frame -print_format csv -loglevel panic
    pub async fn ffprobe_frames() -> BufReader<ChildStdout> {
        println!("Initializing ffprobe");

        let proc = Command::new("./ffprobe")
            .args(["-probesize", "32"])
            .args(["-loglevel", "fatal"])
            .args(["-f", "sdp", "./extstream"])
            .args(["-protocol_whitelist", "rtp,file,udp"])
            .args(["-show_entries", "frame=pkt_size,key_frame"])
            .args(["-print_format", "csv"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("ffprobe start failure");

        BufReader::new(proc.stdout.unwrap())
    }

    pub async fn ffprobe_codec() -> String {
        println!("Running ffprobe for stream");

        let out = Command::new("./ffprobe")
            .args(["-probesize", "32"])
            .args(["-loglevel", "panic"])
            .args(["-f", "sdp", "./extstream"])
            .args(["-protocol_whitelist", "rtp,file,udp"])
            .args(["-show_entries", "stream=codec_name"])
            .args(["-print_format", "csv"])
            .output()
            .await
            .expect("ffprobe start failure")
            .stdout;

        // Convert CSV output into a raw name
        String::from_utf8(out)
            .expect("Error parsing ffprobe output??")
            .rsplit_once(",")
            .expect("Unable to get split...")
            .1
            .to_string()
    }

    // ./ffmpeg -f sdp -protocol_whitelist rtp,file,udp -i lel.sdp
    // ./ffmpeg -re -i sap:// -vcodec copy -f rtp rtp://127.0.0.1:9553?pkt_size=1316
    // https://stackoverflow.com/questions/16658873/how-to-minimize-the-delay-in-a-live-streaming-with-ffmpeg
    // https://trac.ffmpeg.org/wiki/StreamingGuide#Latency
    // https://stackoverflow.com/questions/60462840/ffmpeg-delay-in-decoding-h264
    // https://stackoverflow.com/questions/65102404/how-to-transfer-video-as-rtp-stream-with-least-delay-by-ffmpeg
    // https://trac.ffmpeg.org/ticket/3354

    // Note: This remux introduces a frame of latency. Nothing you can do about it as far as I can tell.
    // It's a tradeoff between user-friendliness, and absolute performance.
    // It would be possible to avoid the remux if users could mux directly to
    // RTP with a 1200 packet size, but using SAP doesn't pass through RTP settings like packet_size.
    // Not using SAP would require manually moving an SDP from client to server, which isn't tenable.
    pub async fn init_remuxer(sdp: String) -> SocketAddr {
        println!("Initializing remuxer");

        tokio::fs::remove_file("./extstream").await.ok();

        let addr = SocketAddr::new(
            IpAddr::from(Ipv4Addr::LOCALHOST),
            7690, //portpicker::pick_unused_port().unwrap(),
        );

        let mut ext_sdp = File::create("./extstream").await.expect("file create");
        ext_sdp.write_all(&sdp.as_bytes()).await.expect("Write SDP");
        drop(ext_sdp); // close external sdp

        Command::new("./ffmpeg")
            // General Settings
            .args(["-probesize", "32"])
            .args(["-loglevel", "fatal"])
            .args(["-threads", "1"])
            .args(["-fflags", "+flush_packets+nobuffer+discardcorrupt"])
            .args(["-avioflags", "direct"])
            .args(["-flags", "low_delay"])
            // Input
            .args(["-protocol_whitelist", "rtp,file,udp"])
            .args(["-f", "sdp"])
            .args(["-i", "extstream"])
            // Output
            .args(["-vcodec", "copy"])
            .args(["-f", "rtp"])
            .args(["-packetsize", "1200"])
            .args(["-flags", "low_delay"])
            .args(["-fflags", "+flush_packets"])
            .arg(format!("rtp://{}", addr))
            .spawn()
            .expect("remuxer start failure");

        addr
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

// https://stackoverflow.com/questions/1957427/detect-mpeg4-h264-i-frame-idr-in-rtp-stream
// https://stackoverflow.com/questions/38795036/detect-vp8-key-framei-frame-in-rtp-stream

// https://github.com/bluejekyll/multicast-example/blob/c3ef3be23e6cf0a9c30900ef40d14b52ccf93efe/src/lib.rs#L45

/*           let sdp_socket = UdpSocket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
           let sdp_multicast_ip = Ipv4Addr::new(224, 2, 127, 254);

           sdp_socket.join_multicast_v4(&sdp_multicast_ip, &Ipv4Addr::new(0, 0, 0, 0));
           let bindaddr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 0);

           sdp_socket.set_reuse_address(true);
           sdp_socket.bind(&SockAddr::from(bindaddr));
           // TODO: Start ffprobe reader task.

           let mut buf = std::mem::MaybeUninit::new(vec![0u8; 1600]); // UDP MTU
           tokio::net::UdpSocket::from(sdp_socket.std);
           // Read packet continuously

*/

async fn block_for_file(file: PathBuf) -> File {
    while let Err(_) = tokio::fs::File::open(&file).await {
        tokio::task::yield_now().await;
    }
    tokio::fs::File::open(file).await.unwrap()
}

// ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libx264 -b:v 200k -cpu-used 5 -g 3 -f sap -packet_size 1000 sap://239.7.69.7:5002
