use std::sync::atomic::Ordering::Relaxed;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use webrtc::track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocalWriter};
pub struct TrackFeeder {
    sender: Sender<Vec<u8>>,
    track: Arc<TrackLocalStaticRTP>,
    active: Arc<AtomicBool>,
}

/**
 * Responsible for "Feeding" a track
 */
impl TrackFeeder {
    pub fn new(buf: Vec<Vec<u8>>, track: Arc<TrackLocalStaticRTP>) -> TrackFeeder {
        let active_atomic = Arc::new(AtomicBool::new(true));

        // Start the async feeder task.
        let send = TrackFeeder::task(track.clone(), active_atomic.clone());

        for packet in buf {
            send.send(packet);
        }

        TrackFeeder {
            sender: send,
            track: track,
            active: active_atomic,
        }
    }

    // Creates tokio task for async feeding webrtc track with rtp packets.
    fn task(track: Arc<TrackLocalStaticRTP>, task_active: Arc<AtomicBool>) -> Sender<Vec<u8>> {
        let (send, mut recive) = channel::<Vec<u8>>(10);

        tokio::spawn(async move {
            loop {
                // Simple: Wait on channel reciever...
                let packet = recive.recv().await.unwrap();

                // If discarded, end the task.
                if !task_active.load(Relaxed) {
                    break;
                }

                // If not discarded, directly write the incoming packet to webrtc track
                track.write(&packet).await.unwrap();
            }
        });

        send
    }

    pub fn track() {}

    pub fn push(&self, packet: Vec<u8>) {
        self.sender.send(packet);
    }

    pub fn discard(&self) {
        self.active.store(false, Relaxed);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Relaxed)
    }
}
