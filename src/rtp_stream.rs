use anyhow::Result;
use rtp_rs::RtpReader;
use socket2::{Domain, Protocol, Socket, Type};
use std::fmt::format;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
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

use crate::sdp::Sdp;
use crate::track_feeder::{self, TrackFeeder};

use super::ffmpeg::FFmpeg;
use super::stream_peer::StreamPeer;

pub struct RtpStream {
    ffmpeg: Option<FFmpeg>,
    delay: bool,
    ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
    feeders: Arc<Mutex<Vec<TrackFeeder>>>,
}

impl RtpStream {
    pub fn new(sdp: Sdp) -> RtpStream {
        // Distribute to feeders
        let ff_packets: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let feeders: Arc<Mutex<Vec<TrackFeeder>>> = Arc::new(Mutex::new(Vec::new()));

        //RtpStream::task(ff_packets.clone(), feeders.clone(), sdp.streams[0].port);

        RtpStream {
            ffmpeg: None,
            delay: false,
            ff_packets,
            feeders,
        }
    }

    fn task(
        ff_packets: Arc<Mutex<Vec<Vec<u8>>>>,
        feeders: Arc<Mutex<Vec<TrackFeeder>>>,
        ffmpeg_port: u16,
    ) {
        tokio::spawn(async move {
            // Open socket

            // TODO: Need seperate task to rx SDP packets.
            // 1. RX SDP on multicast
            // 2. Hash incoming SDP (port+ip)
            // 3. Init new rtp_stream if new stream

            // RTP_stream
            // 1. Init with raw SDP
            // 2. Parse protocol, port, address
            // 3. Connect to RTP port with reuseaddr enabled
            // 4. Init FFProbe instance
            //  a. Create UDP socket
            //  b. Start ffprobe on that socket
            //  c. Pass SDP through SAP
            // 5. Start parsing FFProbe, and storing RTP packets.

            // let sdp_socket = UdpSocket::bind("0.0.0.0:9875").unwrap();
            // sdp_socket.set_broadcast(true).unwrap();
            // sdp_socket.set_multicast_ttl_v4(128).unwrap();
            // sdp_socket.join_multicast_v4(&Ipv4Addr::new(224,2,127,254),&Ipv4Addr::UNSPECIFIED).unwrap();


            println!("Starting socket");
            let mut buf = vec![0u8; 1600];
            // while let packet = sdp_socket.recv_from(&mut buf).await {
            //     match packet {
            //         Ok((n, _)) => {
            //             println!("{}", String::from_utf8(buf[24..n].to_vec()).unwrap());
            //         }
            //         Err(e) => {
            //             println!("{:?}", e);
            //         }
            //     }
                // Store packet in fast forward buffer
                // ff_packets.lock().unwrap().push(inbound_rtp_packet.clone());

                // Clear discarded feeders
                // feeders.lock().unwrap().retain(|f| f.is_active());

                // Push packet to active feeders.
                //for feeder in feeders.lock().unwrap().iter() {
                //    feeder.push(inbound_rtp_packet.clone());
                //}

                /* Testing code */
                //let header = RtpReader::new(&inbound_rtp_packet).unwrap();
                //let mut pld = header.payload().to_vec();
                //pld.truncate(5);
                //println!("source: type: {}, pld: {:?}", header.payload_type(), pld);

                //todo!("Delete old RTP packets from fast forward buffer");

                //println!()
            //}
            println!("FELL THROUGH LOOP");
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

