use std::sync::Arc;
use tokio::select;

use tokio::sync::{broadcast, mpsc, oneshot};
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter};

use crate::{StreamDef, TrackDef};
pub struct BufferedTrack {
    pub track: Arc<TrackLocalStaticRTP>,
    kill: tokio::sync::oneshot::Sender<()>,
    pub track_def: TrackDef,
    pub stream_def: StreamDef,
    pub sender: Arc<Option<RTCRtpSender>>
}

/**
 * Responsible for "Feeding" a track
 */
impl BufferedTrack {
    pub fn new(
        buf: Vec<Arc<Packet>>,
        subscription: broadcast::Receiver<Arc<Packet>>,
        track_def: &TrackDef,
        stream_def: &StreamDef,
    ) -> BufferedTrack {
        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: track_def.mime_type().unwrap().to_string(),
                ..Default::default()
            },
            track_def.stream_id().to_string(), // id describes this track, within the context of its group. IE you usually have "video" and "audio"
            stream_def.id.clone(), // Stream ID is the unique group this track belongs to.
        ));

        let (kill, rx_kill) = oneshot::channel::<()>();

        println!("Starting buffered pusher task");
        BufferedTrack::pusher_task(track.clone(), rx_kill, buf, subscription);

        BufferedTrack {
            track,
            kill,
            track_def: track_def.clone(),
            stream_def: stream_def.clone(),
            sender: Arc::new(None)
        }
    }

    // Creates tokio task for async feeding webrtc track with rtp packets.
    fn pusher_task(
        track: Arc<TrackLocalStaticRTP>,
        mut kill: oneshot::Receiver<()>,
        ff_buf: Vec<Arc<Packet>>,
        mut subscription: broadcast::Receiver<Arc<Packet>>,
    ) {
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::unbounded_channel::<Arc<Packet>>();

            // Populate the initial send queue before starting on the tx loop
            for pkt in ff_buf {
                tx.send(pkt).unwrap();
            }

            //let recv_future = subscription.recv();
            // tokio::pin!(recv_future);

            println!("Going into buffered loop!");
            loop {
                let recv_future = subscription.recv();
                tokio::pin!(recv_future);

                select! {
                    biased;

                    v = &mut kill => {
                        println!("KILLED, {:?}", v);
                        break;
                    },

                    // All the buffered packets from when this track is created should
                    // be pushed through RTP before any new packets are pushed
                    // because this select is biased. Otherwise problems will happen
                    Some(pkt) = rx.recv()  => {
                        println!("BUFFERED SEND");
                        track.write_rtp(&pkt).await.unwrap();
                    },

                    // Once all buffered packets have sent, simply listen on the
                    // broadcast channel for newly received packets
                    // and dispatch them
                    r = &mut recv_future => {
                        match r {
                            Ok(pkt) => {
                                println!("BROADCASTED SEND");
                                track.write_rtp(&pkt).await.unwrap();

                            }
                            Err(e) => {
                                println!("Broadcast RX error {}", e);
                            }
                        }
                    }



                }
            }
            println!("FELL OUT OF BUFFERED LOOP");
        });
    }

    pub fn discard(self) -> Result<(), ()> {
        self.kill.send(())
    }
}
