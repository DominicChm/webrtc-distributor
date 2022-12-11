use crate::net_util::listen_udp;
use crate::{StreamDef, TrackDef};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::RwLock;
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

    /**
     * RTP reader task. Continuously grabs RTP packets from the IP specified in the track definition
     * and distributes them to BufferedTracks. Optionally, also handles buffering packets
     * for fast-starting new clients.
     */
    fn task_rtp_reader(
        ff_packets: Option<Arc<RwLock<Vec<Arc<Packet>>>>>,
        send: Sender<Arc<Packet>>,
        def: TrackDef,
    ) {
        tokio::spawn(async move {
            let mut stream_state = StreamState::default();

            // Connect to the specified RTP track's socket.
            let sock = listen_udp(&def.socket_addr()).unwrap();
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

                    // Broadcast the packet. Listeners should be BufferedTracks, which
                    // then distribute to individual clients
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
