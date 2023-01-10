use std::sync::{Arc, Weak};
use tokio::select;

use tokio::sync::{mpsc, Notify};
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocalWriter;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::rtp_track::RtpTrack;
pub struct BufferedTrack {
    pub rtc_track: Arc<TrackLocalStaticRTP>,
    pub rtp_track: Weak<RtpTrack>,

    controls: Arc<TaskControls>,
}

#[derive(Default)]
pub struct TaskControls {
    play: Notify,
    stop: Notify,
    kill: Notify,
}

/**
 * Manages the link between Easystreamer's internal RTP track (RTPTrack)
 * and Webrtc-rs's TrackLocalStaticRTP. Supports "fast-starting" a remote client.
 */
impl BufferedTrack {
    pub fn new(rtp_track: Arc<RtpTrack>) -> Arc<BufferedTrack> {
        let suffix: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(char::from)
            .collect();

        // Create a completely unique stream ID
        // TODO: DETERMINE IF NECESSARY (remove if not)
        // let mut sid = rtp_track.stream_def.id.clone();
        // sid.push_str("_");
        // sid.push_str(suffix.as_str());

        let rtc_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: rtp_track.track_def.codec.mime_type().to_string(),
                ..Default::default()
            },
            rtp_track.track_def.stream_id().to_string(), // id describes this track, within the context of its group. IE you usually have "video" and "audio"
            rtp_track.stream_def.id.clone(), // Stream ID is the unique group this track belongs to.
        ));

        let buffered_track = Arc::new(BufferedTrack {
            rtc_track: rtc_track.clone(),
            rtp_track: Arc::downgrade(&rtp_track),
            controls: Arc::new(TaskControls::default()),
        });

        BufferedTrack::pusher_task(Arc::downgrade(&buffered_track));

        buffered_track
    }

    /**
     * Spawns the internal async thread responsible for pushing packets to webrtc-rs
     * Takes
     */
    fn pusher_task(buffered_track: Weak<BufferedTrack>) {
        tokio::spawn(async move {
            let controls = buffered_track.upgrade().unwrap().controls.clone();
            'main: loop {
                // Wait for play before doing anything.
                controls.play.notified().await;

                // Re-initialize on every iteration
                let rtp_track = buffered_track
                    .upgrade()
                    .unwrap()
                    .rtp_track
                    .upgrade()
                    .unwrap();

                let faststart_buf = rtp_track.fast_start_packets().await;
                let mut rtp_subscription = rtp_track.subscribe();
                drop(rtp_track); // drop the rtp track arc after each iter so we don't keep it uncollected.

                // Initialize the faststart buffer.
                let (faststart_tx, mut faststart_rx) = mpsc::unbounded_channel::<Arc<Packet>>();

                println!(
                    "Starting pusher task. Will fast-start {} packets",
                    faststart_buf.len()
                );

                // Populate the fast-start send queue before starting on the tx loop
                for pkt in faststart_buf.iter() {
                    faststart_tx.send(pkt.clone()).unwrap();
                }

                // Reference: https://github.com/meetecho/janus-gateway/blob/master/src/postprocessing/pp-h264.c
                // https://webrtchacks.com/what-i-learned-about-h-264-for-webrtc-video-tim-panton/
                // https://github.com/steely-glint/srtplight
                'inner: loop {
                    tokio::pin! {
                        let faststart_recv = faststart_rx.recv();
                        let rtp_track_recv = rtp_subscription.recv();
                        let killed_recv = controls.kill.notified();
                        let stop_recv = controls.stop.notified();
                    };

                    select! {
                        biased;

                        _ = killed_recv => {
                            break 'main;
                        }

                        _ = stop_recv => {
                            break 'inner;
                        }

                        // All the buffered packets from when this track is created should
                        // be pushed through RTP before any new packets are pushed
                        // because this select is biased. Otherwise problems will happen
                        Some(pkt) = faststart_recv  => {
                            if let Some(track) = buffered_track.upgrade() {
                                track.rtc_track.write_rtp(&pkt).await.unwrap();
                            } else {
                                break 'main;
                            }
                        },

                        // Once all buffered packets have sent, simply listen on the
                        // broadcast channel for newly received packets
                        // and dispatch them
                        Ok(pkt) = rtp_track_recv => {
                            if let Some(track) = buffered_track.upgrade() {
                                track.rtc_track.write_rtp(&pkt).await.unwrap();
                            } else {
                                break 'main;
                            }
                        }

                    }
                }
                eprintln!("Buffered writer stopped.");
            }
        });
    }

    pub async fn play(&self) {
        self.controls.play.notify_waiters();
    }

    pub async fn stop(&self) {
        self.controls.play.notify_waiters();
    }

    pub async fn resync(&self) {
        println!("RESYNCING");
        self.controls.stop.notify_waiters();
        self.controls.play.notify_one();
    }

    pub async fn kill(&self) {
        self.controls.kill.notify_one();
    }
}
