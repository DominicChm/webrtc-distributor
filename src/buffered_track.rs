use std::sync::{Arc, Weak};
use tokio::select;

use tokio::sync::{broadcast, mpsc, Notify, RwLock};
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::rtp_track::RtpTrack;
pub struct BufferedTrack {
    pub track: Arc<TrackLocalStaticRTP>,
    pub rtp_track: Arc<RtpTrack>,
    pusher_spawned: RwLock<bool>,
}

/**
 * Responsible for "Feeding" a track
 */
impl BufferedTrack {
    pub fn new(rtp_track: Arc<RtpTrack>) -> BufferedTrack {
        let suffix: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(char::from)
            .collect();
            
        let mut sid = rtp_track.stream_def.id.clone();
        sid.push_str("_");
        sid.push_str(suffix.as_str());

        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: rtp_track.track_def.mime_type().unwrap().to_string(),
                ..Default::default()
            },
            rtp_track.track_def.stream_id().to_string(), // id describes this track, within the context of its group. IE you usually have "video" and "audio"
            sid, // Stream ID is the unique group this track belongs to.
        ));

        BufferedTrack {
            rtp_track,
            track,
            pusher_spawned: RwLock::new(false),
        }
    }

    // Creates tokio task for async feeding webrtc track with rtp packets.
    fn pusher_task(
        track: Weak<TrackLocalStaticRTP>,
        ff_buf: Vec<Arc<Packet>>,
        mut subscription: broadcast::Receiver<Arc<Packet>>,
    ) {
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::unbounded_channel::<Arc<Packet>>();

            println!("Starting pusher task. Will burst {} packets", ff_buf.len());
            // Populate the initial send queue before starting on the tx loop
            for pkt in ff_buf.iter() {
                tx.send(pkt.clone()).unwrap();
            }

            //let recv_future = subscription.recv();
            // tokio::pin!(recv_future);
            // Reference: https://github.com/meetecho/janus-gateway/blob/master/src/postprocessing/pp-h264.c
            // https://webrtchacks.com/what-i-learned-about-h-264-for-webrtc-video-tim-panton/
            // https://github.com/steely-glint/srtplight
            loop {
                tokio::pin! {
                    let recv_future = subscription.recv();
                }

                select! {
                    biased;

                    // All the buffered packets from when this track is created should
                    // be pushed through RTP before any new packets are pushed
                    // because this select is biased. Otherwise problems will happen
                    Some(pkt) = rx.recv()  => {
                        if let Some(track) = track.upgrade() {
                            track.write_rtp(&pkt).await.unwrap();
                        } else {
                            break;
                        }
                    },

                    // Once all buffered packets have sent, simply listen on the
                    // broadcast channel for newly received packets
                    // and dispatch them
                    r = &mut recv_future => {
                        match r {
                            Ok(pkt) => {
                                if let Some(track) = track.upgrade() {
                                    track.write_rtp(&pkt).await.unwrap();
                                } else {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Broadcast RX error {}", e);
                            }
                        }
                    }

                }
            }
            eprintln!("Buffered writer stopped.");
        });
    }

    pub async fn resume(&self) {
        if self.pusher_spawned.read().await.clone() {
            eprintln!("Pusher already spawned!");
            return;
        }

        println!("Resuming buffered track!");
        let ff_buf = self.rtp_track.ff_buf().await;
        let pkt_sub = self.rtp_track.subscribe();
        BufferedTrack::pusher_task(Arc::downgrade(&self.track), ff_buf, pkt_sub);

        let mut b = self.pusher_spawned.write().await;
        *b = true;
    }
}
