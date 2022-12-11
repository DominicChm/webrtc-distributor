use std::sync::Arc;
use std::time::Duration;
use tokio::select;

use tokio::sync::{broadcast, mpsc, oneshot, Notify, RwLock};
use tokio::task::JoinHandle;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter};

use crate::rtp_track::RtpTrack;
use crate::{StreamDef, TrackDef};
pub struct BufferedTrack {
    pub track: Arc<TrackLocalStaticRTP>,
    pub rtp_track: Arc<RtpTrack>,
    pub kill: Arc<tokio::sync::Notify>,
    pusher_spawned: RwLock<bool>,
}

/**
 * Responsible for "Feeding" a track
 */
impl BufferedTrack {
    pub fn new(rtp_track: Arc<RtpTrack>) -> BufferedTrack {
        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: rtp_track.track_def.mime_type().unwrap().to_string(),
                ..Default::default()
            },
            rtp_track.track_def.stream_id().to_string(), // id describes this track, within the context of its group. IE you usually have "video" and "audio"
            rtp_track.stream_def.id.clone(), // Stream ID is the unique group this track belongs to.
        ));

        let not = Arc::new(Notify::new());

        println!("Starting buffered pusher task");

        BufferedTrack {
            rtp_track,
            track,
            kill: not,
            pusher_spawned: RwLock::new(false),
        }
    }

    // Creates tokio task for async feeding webrtc track with rtp packets.
    fn pusher_task(
        track: Arc<TrackLocalStaticRTP>,
        kill: Arc<Notify>,
        ff_buf: Vec<Arc<Packet>>,
        mut subscription: broadcast::Receiver<Arc<Packet>>,
    ) {
        println!("Trying to start pusher task");
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::unbounded_channel::<Arc<Packet>>();

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
                    let kill_future = kill.notified();
                }

                select! {
                    biased;

                    _ = &mut kill_future => {
                        drop(kill_future);
                        drop(recv_future);
                        break;
                    },

                    // All the buffered packets from when this track is created should
                    // be pushed through RTP before any new packets are pushed
                    // because this select is biased. Otherwise problems will happen
                    Some(pkt) = rx.recv()  => {
                        track.write_rtp(&pkt).await.unwrap();
                        println!("buf wrt")
                    },

                    // Once all buffered packets have sent, simply listen on the
                    // broadcast channel for newly received packets
                    // and dispatch them
                    r = &mut recv_future => {
                        match r {
                            Ok(pkt) => {
                                track.write_rtp(&pkt).await.unwrap();
                            }
                            Err(e) => {
                                println!("Broadcast RX error {}", e);
                            }
                        }
                    }

                }
            }
            println!("Buffered writer is dead.");
        });
    }

    pub async fn restart(&self) {
        self.pause().await;
        self.resume().await;
    }

    pub async fn pause(&self) {
        self.kill.notify_waiters();

        let mut b = self.pusher_spawned.write().await;
        *b = false;
    }

    pub async fn resume(&self) {
        if self.pusher_spawned.read().await.clone() {
            println!("Pusher already spawned!");
            return;
        }

        println!("Resuming buffered track!");
        let ff_buf = self.rtp_track.ff_buf().await;
        let pkt_sub = self.rtp_track.subscribe();
        BufferedTrack::pusher_task(self.track.clone(), self.kill.clone(), ff_buf, pkt_sub);

        let mut b = self.pusher_spawned.write().await;
        *b = true;
    }

    pub async fn is_paused(&self) -> bool {
        !self.pusher_spawned.read().await.clone()
    }

    pub async fn discard(self) {
        self.pause().await;
    }
}
