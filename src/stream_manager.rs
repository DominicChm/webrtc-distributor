use lazy_static::lazy_static;
use regex::Regex;
use std::hash::Hash;
use std::{
    collections::{BTreeMap, HashMap},
    io::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    string::ParseError,
};

use crate::{
    net_util::{join_multicast, listen_udp},
    rtp_stream::RtpStream,
    sdp::Sdp,
};

#[derive(Clone)]
pub struct StreamManager {
    streams: HashMap<Sdp, u128>,
}
impl StreamManager {
    pub fn new(mcast_ip: IpAddr, mcast_port: u16) {
        let mcast_addr = SocketAddr::new(mcast_ip, mcast_port);
        let sap_socket = listen_udp(&mcast_addr).expect("open SDP socket");
        let sap_socket = tokio::net::UdpSocket::from_std(sap_socket).expect("convert std to tokio");
        tokio::spawn(async move {
            let mut streams: HashMap<Sdp, RtpStream> = HashMap::new();
            // Open socket

            // TODO: Need seperate task to rx SDP packets.
            // 1. RX SDP on multicast
            // 2. Hash incoming SDP (port+ip)
            // 3. Init new rtp_stream if new stream

            println!("Listening for SAP announcements on {}", mcast_addr);
            let mut buf = vec![0u8; 1600];
            while let packet = sap_socket.recv_from(&mut buf).await {
                match packet {
                    Ok((n, _)) => {
                        let str = String::from_utf8(buf[24..n].to_vec()).expect("sdp to string");
                        let sdp = crate::sdp::parse_sdp(str);
                        
                        if !streams.contains_key(&sdp) {
                            println!("Received unique SDP: {:?}", sdp);
                            let s = RtpStream::new(sdp.clone());
                            streams.insert(sdp, s);
                            // TODO: ATTACH LISTENERS
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
            }
        });
    }
}
// Ipv4Addr::new(224,2,127,254)
// 9875
