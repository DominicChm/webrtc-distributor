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
}

impl RtpStream {
    pub fn new(port: u16) -> RtpStream {
        // Distribute to feeders
        let ff_packets: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let feeders: Arc<Mutex<Vec<TrackFeeder>>> = Arc::new(Mutex::new(Vec::new()));

        RtpStream::task(ff_packets.clone(), feeders.clone(), port);

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
            let mcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(224,2,127,254)), 9875);
            let sdp_socket = join_multicast(&mcast_addr).unwrap().unwrap();
            let sdp_socket = UdpSocket::from_std(sdp_socket).unwrap();
            

            // let sdp_socket = UdpSocket::bind("0.0.0.0:9875").unwrap();
            // sdp_socket.set_broadcast(true).unwrap();
            // sdp_socket.set_multicast_ttl_v4(128).unwrap();
            // sdp_socket.join_multicast_v4(&Ipv4Addr::new(224,2,127,254),&Ipv4Addr::UNSPECIFIED).unwrap();

            
            println!("Starting socket");
            let mut buf = vec![0u8; 1600];
            while let packet = sdp_socket.recv_from(&mut buf).await {
                match packet {
                    Ok((n, _)) => {
                        println!("{}", String::from_utf8(buf[24..n].to_vec()).unwrap());
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
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
            }
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

#[cfg(unix)]
fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    socket.bind(&socket2::SockAddr::from(*addr))
}

#[cfg(windows)]
fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    use std::net::Ipv6Addr;

    let addr = match *addr {
        SocketAddr::V4(addr) => SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), addr.port()),
        SocketAddr::V6(addr) => {
            SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(), addr.port())
        }
    };
    socket.bind(&socket2::SockAddr::from(addr))
}

/// Returns a socket joined to the multicast address
fn join_multicast(multicast_addr: &SocketAddr) -> Result<Option<std::net::UdpSocket>, io::Error> {
    let ip_addr = multicast_addr.ip();
    if !ip_addr.is_multicast() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("expected multicast address for binding: {}", ip_addr),
        ));
    }

    let socket = match ip_addr {
        IpAddr::V4(ref mdns_v4) => {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv4 dgram socket");
            socket
                .join_multicast_v4(mdns_v4, &Ipv4Addr::new(0, 0, 0, 0))
                .expect("join_multicast_v4");
            socket
        }
        IpAddr::V6(ref mdns_v6) => {
            let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv6 dgram socket");

            socket.set_only_v6(true)?;
            socket
                .join_multicast_v6(mdns_v6, 0)
                .expect("join_multicast_v6");
            socket
        }
    };

    socket.set_nonblocking(true).expect("nonblocking Error");
    socket.set_reuse_address(true).expect("reuse addr Error");
    #[cfg(unix)] // this is currently restricted to Unix's in socket2
    socket.set_reuse_port(true).expect("reuse port Error");
    bind_multicast(&socket, &multicast_addr).expect("bind Error");

    let udp: std::net::UdpSocket = socket.into();
    Ok(Some(udp))
}
