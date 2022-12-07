use crate::buffered_track::BufferedTrack;
use crate::net_util::listen_udp;
use crate::{StreamDef, TrackDef};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::broadcast::{self, Receiver, Sender};
use webrtc::api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8};
use webrtc::rtp::packet::Packet;
use webrtc::util::Unmarshal;

pub struct RtpTrack {
    ff_packets: Arc<Mutex<Vec<Arc<Packet>>>>,
    pub stream_def: StreamDef,
    pub track_def: TrackDef,
    subscriber: Receiver<Arc<Packet>>,
}

/**
 * Handles the ingestion and buffering of an RTP track.
 */
impl RtpTrack {
    pub fn new(track_def: &TrackDef, stream_def: &StreamDef) -> RtpTrack {
        // Distribute to feeders
        let ff_packets: Arc<Mutex<Vec<Arc<Packet>>>> = Arc::new(Mutex::new(Vec::new()));

        let (tx, subscriber) = broadcast::channel::<Arc<Packet>>(100);

        RtpTrack::task_rtp_reader(Some(ff_packets.clone()), tx, track_def.clone());

        RtpTrack {
            ff_packets,
            stream_def: stream_def.clone(),
            track_def: track_def.clone(),
            subscriber,
        }
    }

    fn task_rtp_reader(
        ff_packets: Option<Arc<Mutex<Vec<Arc<Packet>>>>>,
        send: Sender<Arc<Packet>>,
        def: TrackDef,
    ) {
        tokio::spawn(async move {
            let ip = def.ip.unwrap_or(IpAddr::from(Ipv4Addr::LOCALHOST));

            //TODO: error handling
            // Connect to the specified UDP track's socket.
            let sock = listen_udp(&SocketAddr::new(ip, def.port)).expect("Failed to bind!");
            let sock = UdpSocket::from_std(sock).expect("Failed to convert to tokio socket");

            let mut buf = vec![0u8; 1600];
            loop {
                if let Ok((n, _)) = sock.recv_from(&mut buf).await {
                    let trimmed = buf[..n].to_vec();

                    if n > 1200 {
                        println!("ERROR: Received an RTP packet greater than 1200 bytes! Make sure your format address specifies a max packet size of 1200!! (Hint: try adding `-pkt_size 1200` to your FFMPEG command)");
                        break;
                    }

                    let mut b: &[u8] = &trimmed;
                    let pkt = Arc::new(Packet::unmarshal(&mut b).unwrap());

                    // If passed a FF buffer, handle FF buffering
                    if let Some(ff) = ff_packets.clone() {
                        let mut ff = ff.lock().unwrap();

                        let is_kf = is_keyframe(&pkt.clone(), &def);
                        let kf_debounced = ff.len() == 0 || !is_keyframe(&ff.last().unwrap(), &def);

                        // Reset fast-forward buffer on new GOP/keyframe.
                        if is_kf && kf_debounced {
                            ff.clear();
                        }

                        // Store new packet in fast forward buffer
                        ff.push(pkt.clone());
                    }

                    // Broadcast the packet

                    match send.send(pkt) {
                        Err(e) => println!("BROADCAST ERR: {}", e),
                        _ => (),
                    }
                } else {
                    println!("Problem receiving from UDP socket");
                }
            }
        });
    }

    pub fn get_buffered_track(&self) -> BufferedTrack {
        let f = BufferedTrack::new(
            self.ff_packets.lock().unwrap().clone(),
            self.subscriber.resubscribe(),
            &self.track_def,
            &self.stream_def,
        );

        // Add to feeder array for future pushes
        f
    }
}

// https://stackoverflow.com/questions/1957427/detect-mpeg4-h264-i-frame-idr-in-rtp-stream
fn is_keyframe(pkt: &Packet, def: &TrackDef) -> bool {
    let mime = def.mime_type().unwrap();
    match mime {
        MIME_TYPE_VP8 => {
            // https://datatracker.ietf.org/doc/html/rfc7741#section-4.3
            // https://github.com/FFmpeg/FFmpeg/blob/master/libavformat/rtpdec_vp8.c

            // Note: bit 0 is MSB
            let mut b = &pkt.payload[..];

            let x = b[0] & 0x80 != 0;
            let s = b[0] & 0x10 != 0;
            let pid = b[0] & 0x0f;

            if x {
                b = &b[1..];
            }

            let i = x && b[0] & 0x80 != 0;
            let l = x && b[0] & 0x40 != 0;
            let t = x && b[0] & 0x20 != 0;
            let k = x && b[0] & 0x10 != 0;

            b = &b[1..];

            // Handle I
            if i && b[0] & 0x80 != 0 {
                b = &b[2..];
            } else if i {
                b = &b[1..];
            }

            // Handle L
            if l {
                b = &b[1..];
            }

            // Handle T/K
            if t || k {
                b = &b[1..];
            }

            b[0] & 0x01 == 0 && s && pid == 0
        }
        MIME_TYPE_H264 => pkt.payload.get(0).unwrap().eq(&124),
        _ => false,
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

// ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libx264 -b:v 200k -cpu-used 5 -g 3 -f sap -packet_size 1000 sap://239.7.69.7:5002
