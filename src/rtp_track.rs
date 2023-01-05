use crate::net_util::listen_udp;
use crate::{StreamDef, TrackDef};
use std::collections::VecDeque;
use std::sync::{Arc, Weak};
use tokio::net::UdpSocket;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::RwLock;
use webrtc::rtp::packet::Packet;
use webrtc::util::Unmarshal;

// NOTE: This should probably be a VecDequeue to optimize
// removal of old packets on new keyframe (see handle_fs_buffering)
// but VecDequeue is missing truncate_front. For now, not worth reworking
// for an operation that's done once every second or so.
// (unless proven otherwise)
// https://github.com/rust-lang/rust/issues/92547
type FastStartBuf = RwLock<Vec<Arc<Packet>>>;

pub struct RtpTrack {
    pub stream_def: StreamDef,
    pub track_def: TrackDef,
    ff_packets: Arc<FastStartBuf>,
    subscriber: Receiver<Arc<Packet>>,
}
const MAX_PACKETS: usize = 10000;

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
        let ff_packets = Arc::new(RwLock::new(Vec::new()));

        let (tx, subscriber) = broadcast::channel::<Arc<Packet>>(MAX_PACKETS);

        RtpTrack::task_rtp_reader(Arc::downgrade(&ff_packets), tx, track_def.clone(), true);

        RtpTrack {
            ff_packets,
            stream_def: stream_def.clone(),
            track_def: track_def.clone(),
            subscriber,
        }
    }

    /**
     * RTP reader task. Continuously grabs RTP packets from the IP specified in the track definition
     * and distributes them to BufferedTracks. Optionally, also handles buffering packets
     * for fast-starting new clients.
     */
    fn task_rtp_reader(
        fast_start_packets: Weak<FastStartBuf>,
        broadcast: Sender<Arc<Packet>>,
        def: TrackDef,
        fast_start: bool,
    ) {
        tokio::spawn(async move {
            let mut stream_state = StreamState::default();

            let sock = listen_udp(&def.socket_addr()).unwrap();
            let sock = UdpSocket::from_std(sock).unwrap();

            let mut buf = vec![0u8; 1600];
            loop {
                if let Ok((n, _)) = sock.recv_from(&mut buf).await {
                    let trimmed = buf[..n].to_vec();

                    if n > 1200 {
                        println!(
                            "ERROR: Received an RTP packet greater than 1200 bytes! 
                        Make sure your format address specifies a max packet size of 1200!! 
                        (Hint: try adding `-pkt_size 1200` to your FFMPEG command)"
                        );
                        break;
                    }

                    // Parse the incoming data into a Packet struct
                    // using WebRtc-rs's unmarshal to access RTP information.
                    let mut b: &[u8] = &trimmed;
                    let pkt = Arc::new(Packet::unmarshal(&mut b).unwrap());

                    // Handle buffering (if enabled) and exiting on main struct deletion
                    // We use the dropping of the fast_start_packets Arc to recognize the
                    // deletion of the parent track.
                    match fast_start_packets.upgrade() {
                        Some(ff) if fast_start => {
                            RtpTrack::handle_fast_start_buffering(
                                ff,
                                pkt.clone(),
                                &def,
                                &mut stream_state,
                            )
                            .await;
                        }

                        None => {
                            println!("FF buffer gone. Exiting RTP");
                            break;
                        }
                        _ => (),
                    }

                    // Broadcast the packet to listening BufferedTracks
                    if let Err(e) = broadcast.send(pkt) {
                        println!("BROADCAST ERR: {}", e);
                    }
                } else {
                    println!("Problem receiving from UDP socket");
                }
            }
            println!("RTP reader exited.")
        });
    }

    /**
     * KeyFrame buffering logic.
     * Chrome seems to require TWO keyframes to being displaying video (from testing).
     * This buffering logic attempts to keep two keyframes in its buffer, with one
     * at ff[0]. When a new keyframe is identified, the buffer is trimmed to the
     * previously most-recent keyframe Group Of Packets (GOP).
     *
     * This approach has some disadvantages, but it's the lesser evil compared to
     * manually generating I-frames or having a short GOP interval.
     * Notably, this approach should be good for CPU-constrained environments
     * like embedded systems. (TO BE PROVEN :/)
     */
    pub async fn handle_fast_start_buffering(
        ff: Arc<RwLock<Vec<Arc<Packet>>>>,
        pkt: Arc<Packet>,
        def: &TrackDef,
        state: &mut StreamState,
    ) {
        let mut ff = ff.write().await;

        // Ensure that buffering code is only run if there's at least one pkt in the ff buffer.
        if let Some(last_pkt) = ff.last() {
            // New timestamp = new group of packets = new frame
            let is_new_gop = last_pkt.header.timestamp != pkt.header.timestamp;

            state.found_kf |= def.is_keyframe(&pkt);

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

                    state.idx_last_gop -= trimmed_start;
                    state.idx_last_kframe_gop -= trimmed_start;
                    state.num_packets_buffered -= trimmed_start;
                }
                // Update index of the last GOP.
                // At this point, newest packet hasn't been added
                // so len is eq. to it's index
                state.idx_last_gop = ff.len();
            }
        }

        state.num_packets_buffered += 1;
        ff.push(pkt.clone());
    }

    pub async fn ff_buf(&self) -> Vec<Arc<Packet>> {
        self.ff_packets.read().await.clone()
    }

    /**
     * Returns a new broadcast handle that distributes this stream's RTP packets
     * as they're received. Should be used to distribute a stream's packets
     * to a client. 
     */
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Packet>> {
        self.subscriber.resubscribe()
    }
}
