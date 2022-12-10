use crate::buffered_track::BufferedTrack;
use crate::net_util::listen_udp;
use crate::{StreamDef, TrackDef};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::RwLock;
use webrtc::api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8};
use webrtc::rtcp::packet;
use webrtc::rtp::packet::Packet;
use webrtc::util::Unmarshal;

pub struct RtpTrack {
    ff_packets: Arc<RwLock<Vec<Arc<Packet>>>>,
    pub stream_def: StreamDef,
    pub track_def: TrackDef,
    subscriber: Receiver<Arc<Packet>>,
}

#[derive(Default, Clone, Copy)]
pub struct StreamState {
    /**
     * Index of the last group of packets.
     * A group of packets is designated by a common timestamp.
     */
    idx_last_gop: usize,
    idx_last_kframe_gop: usize,
    found_kf: bool,
    num_packets_buffered: usize,
}
/**
 * Handles the ingestion and buffering of an RTP track.
 */
impl RtpTrack {
    pub fn new(track_def: &TrackDef, stream_def: &StreamDef) -> RtpTrack {
        // Distribute to feeders
        let ff_packets: Arc<RwLock<Vec<Arc<Packet>>>> = Arc::new(RwLock::new(Vec::new()));

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
        ff_packets: Option<Arc<RwLock<Vec<Arc<Packet>>>>>,
        send: Sender<Arc<Packet>>,
        def: TrackDef,
    ) {
        tokio::spawn(async move {
            let mut stream_state = StreamState::default();

            // ===== INIT SOCKET =====
            let ip = def.ip.unwrap_or(IpAddr::from(Ipv4Addr::LOCALHOST));
            let sock_addr = SocketAddr::new(ip, def.port);

            // Connect to the specified UDP track's socket.
            let sock = listen_udp(&sock_addr).unwrap();
            let sock = UdpSocket::from_std(sock).unwrap();

            // Begin main UDP packet consumption loop
            let mut buf = vec![0u8; 1600];
            loop {
                if let Ok((n, _)) = sock.recv_from(&mut buf).await {
                    // Trim the main buf to the # of bytes received
                    let trimmed = buf[..n].to_vec();

                    if n > 1200 {
                        println!("ERROR: Received an RTP packet greater than 1200 bytes! Make sure your format address specifies a max packet size of 1200!! (Hint: try adding `-pkt_size 1200` to your FFMPEG command)");
                        break;
                    }

                    // Parse the incoming data into a Packet struct using WebRtc-rs's unmarshal
                    // impl. This provides a few utility fields used throughout the process.
                    let mut b: &[u8] = &trimmed;
                    let pkt = Arc::new(Packet::unmarshal(&mut b).unwrap());

                    // If passed a FF buffer, handle FF buffering
                    if let Some(ff) = ff_packets.clone() {
                        RtpTrack::handle_ff_buffering(
                            ff.clone(),
                            pkt.clone(),
                            &def,
                            &mut stream_state,
                        )
                        .await;
                    }

                    // Broadcast the packet
                    if let Err(e) = send.send(pkt) {
                        println!("BROADCAST ERR: {}", e);
                    }
                } else {
                    println!("Problem receiving from UDP socket");
                }
            }
        });
    }

    /**
     * KF buffering logic.
     * Chrome seems to require TWO total keyframes to being displaying video.
     * This buffering logic attempts to keep two keyframes in its buffer, with one
     * at ff[0].
     * When a new keyframe is identified, the buffer is trimmed to the previously most-recent
     * keyframe Group Of Packets (GOP).
     */
    pub async fn handle_ff_buffering(
        ff: Arc<RwLock<Vec<Arc<Packet>>>>,
        pkt: Arc<Packet>,
        def: &TrackDef,
        state: &mut StreamState,
    ) {
        // Lock the fast-forward buffer for reading/writing
        let mut ff = ff.write().await;

        // Ensures that buffering code is only run if there's at least one pkt in the ff buffer.
        if let Some(last_pkt) = ff.last() {
            let is_new_gop = last_pkt.header.timestamp != pkt.header.timestamp;
            state.found_kf |= is_keyframe(&pkt.clone(), def);

            if is_new_gop {
                // If previous GOP was a KF, handle a buffer trim
                if state.found_kf {
                    // Keep the index of the last KF gop for trimming
                    let trimmed_start = state.idx_last_kframe_gop;
                    
                    // Update indices for the incoming KF GOP
                    state.idx_last_kframe_gop = state.idx_last_gop;
                    state.found_kf = false;

                    // Trim the packet vec, update indices
                    // TODO: UPDATE VEC TO BE A VECDEQUE
                    // https://users.rust-lang.org/t/best-way-to-drop-range-of-elements-from-front-of-vecdeque/31795
                    drop(ff.drain(..trimmed_start));
                    //println!("New KF! New start: {}", trimmed_start);
                    state.idx_last_gop -= trimmed_start;
                    state.idx_last_kframe_gop -= trimmed_start;
                    state.num_packets_buffered -= trimmed_start;
                }

                state.idx_last_gop = ff.len();

                //println!("NUM PACKETS IN BUFFER: {}", state.num_packets_buffered);
            }
        }

        // Store new packet in fast forward buffer
        state.num_packets_buffered += 1;
        ff.push(pkt.clone());
    }

    pub async fn ff_buf(&self) -> Vec<Arc<Packet>> {
        self.ff_packets.read().await.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Packet>> {
        self.subscriber.resubscribe()
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
        MIME_TYPE_H264 => {
            // https://stackoverflow.com/questions/1957427/detect-mpeg4-h264-i-frame-idr-in-rtp-stream
            let p = &pkt.payload;
            let fragment_type = p.get(0).unwrap() & 0x1F;
            let nal_type = p.get(1).unwrap() & 0x1F;
            let start_bit = p.get(1).unwrap() & 0x80;

            ((fragment_type == 28 || fragment_type == 29) && nal_type == 5 && start_bit == 128)
                || fragment_type == 5
        }
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
