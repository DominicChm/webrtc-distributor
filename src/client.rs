/*
    Manages peer connections
*/

use std::sync::{Arc, Mutex};

use webrtc::{
    api::media_engine::MIME_TYPE_VP8,
    peer_connection::sdp::session_description::RTCSessionDescription,
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability, sctp::stream,
    track::track_local::track_local_static_rtp::TrackLocalStaticRTP,
};

use crate::{rtp_stream::RtpStream, track_feeder::TrackFeeder};

struct track_info {
    track: Arc<TrackLocalStaticRTP>,
    feeder: Option<Arc<TrackFeeder>>,
}

pub struct Client {
    tracks: Vec<track_info>,
}

impl Client {
    pub async fn new(offer: RTCSessionDescription) -> Client {
        // let track = Arc::new(TrackLocalStaticRTP::new(
        //     RTCRtpCodecCapability {
        //         mime_type: MIME_TYPE_VP8.to_owned(),
        //         ..Default::default()
        //     },
        //     format!("video_{}", "kek"),
        //     "test".to_string(), // Make the stream id the passed in string
        // ));

        // track_handle
        // .write(&inbound_rtp_packet[..n])
        // .await
        // .expect("Failed to write to track!");

        Client { tracks: Vec::new() }
    }

    async fn offer_response(&self) {}

    /**
     * Links the track with the passed index to the passed RTP stream.
     * Switches with fast-forwarding, allowing seamless switches.
     */
    pub fn set_track_stream(&mut self, index: usize, stream: &RtpStream) {
        let mut track_info = self
            .tracks
            .get_mut(index)
            .expect("Attempt to set stream on track that doesn't exist");

        // If there's already a feeder connected to the track, stop and get rid of it.
        if let Some(ref f) = track_info.feeder {
            f.discard();
        }

        track_info.feeder = Some(stream.setup_feeder(track_info.track.clone()));
    }

    /**
     * Cleans up after a webrtc client has disconnected.
     * Takes ownership of self so no futher calls are possible.
     */
    pub fn discard(self) {
        for track_info in self.tracks {
            // Get rid of all tracks.

            // Get rid of all track feeders
            if let Some(ref f) = track_info.feeder {
                f.discard();
            }
        }
    }
}

/*
let t1 = new RtpStream(5000);

let client = Client::new(offer)
println(client.offer_response().await)
client.tracks(3); //allocate 3 tracks

client.set_track_stream(3, t1);
client.remove_track(3);

client.tracks(5)


*/
