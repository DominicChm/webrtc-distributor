use std::sync::mpsc::Receiver;

use serde::{Deserialize, Serialize};
use tokio::{spawn, sync::broadcast};
use webrtc::{
    peer_connection::sdp::session_description::RTCSessionDescription, sdp::SessionDescription,
};

#[derive(Deserialize)]
//#[serde(tag = "type")]
pub enum RXMessages {
    WebrtcOffer,
}

#[derive(Deserialize)]
pub struct WebrtcOffer {
    id: String,
    offer: RTCSessionDescription,
}

#[derive(Clone)]
pub struct Negotiate {
    offer: RTCSessionDescription,
}

#[derive(Clone)]
pub enum APIi {
    NEGOTIATE(Negotiate),
}

#[derive(Clone)]
pub enum APIo {}

pub struct Dispatchable {
    
}

pub struct RoutedApi<Tin, Tout> {
    pub unhandled: broadcast::Receiver<Tin>,
    pub out: broadcast::Receiver<Tout>
}

impl<Tin, Tout> RoutedApi<Tin, Tout> {

    pub fn dispatch() {

    }

}

pub async fn init_api() -> (broadcast::Sender<APIi>, broadcast::Receiver<APIo>) {
    let (_tx, mut rx) = broadcast::channel::<APIi>(10);
    let (tx, _rx) = broadcast::channel::<APIo>(10);

    loop {
        let req = rx.recv().await.expect("API RX ERROR");

        spawn(async {});
    }

    (_tx, _rx)
}

pub async fn handle_negotiation() -> Option<APIo> {}

pub struct AppController {
    fn negotiate(client_id, offer) -> RTCSessionDescription {

    }

    fn client_add_stream(client_id, stream_id, offer) {

    }

    fn client_remove_stream(client_id, stream_id, offer) {

    }

    fn add_stream(id) {

    }

    fn delete_stream(id) {

    }


    fn recv_client_state() -> Reciever<ClientState> {
        client_state_notifier.resubscribe()
    }

    // Channel that notifies when specific client IDs should begin renegotiation
    // Note: Functions that explicitly take an offer and return a response do not
    // trigger this listener. Their reneg is intended to be handled in a sync fashion
    fn recv_client_reneg() -> Reciever<ClientState> {
        client_state_notifier.resubscribe()
    }

}