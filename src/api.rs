use serde::{Serialize, Deserialize};
use webrtc::{sdp::SessionDescription, peer_connection::sdp::session_description::RTCSessionDescription};


#[derive(Deserialize)]
//#[serde(tag = "type")]
pub enum RXMessages {
    WebrtcOffer
}

#[derive(Deserialize)]
pub struct WebrtcOffer {
    id: String,
    offer: RTCSessionDescription,
}

pub fn init_api() {

}