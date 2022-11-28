use std::sync::Arc;

use tokio::sync::mpsc::{channel, Sender};
use webrtc::{
    api::media_engine::MIME_TYPE_VP8,
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter},
};
pub struct TrackFeeder {
    sender: Sender<Vec<u8>>,
    track: Arc<TrackLocalStaticRTP>,
    active: bool,
}

impl TrackFeeder {
    pub fn new(buf: Vec<Vec<u8>>, track: Arc<TrackLocalStaticRTP>) -> TrackFeeder {
        let (send, mut rec) = channel::<Vec<u8>>(10);

        let t_i = track.clone();
        tokio::spawn(async move {
            loop {
                // Simple: Wait on channel reciever...
                let packet = rec.recv().await.unwrap();

                // and push it to track (if not discarded)
                t_i.write(&packet).await.unwrap();
            }
        });

        for packet in buf {
            send.send(packet);
        }

        TrackFeeder {
            sender: send,
            track: track,
            active: true,
        }
    }

    // Meant to run in tokio task
    fn distribute() {}

    pub fn track() {}

    pub fn push(&self, packet: Vec<u8>) {
        self.sender.send(packet);
    }

    pub fn discard(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
