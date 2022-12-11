use std::{
    io::Read,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

// Powers the internal server
use rouille::{router, session, Request, Response};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::controller::AppController;

#[derive(Serialize, Deserialize)]
pub struct ClientRPC {
    session: String,
    rpc: RPC,
}

#[derive(Serialize, Deserialize)]
//#[serde(tag = "type")]
pub enum RPC {
    NEGOTIATE(RTCSessionDescription),
}

pub enum RPCResponse {
    NEGOTIATE(RTCSessionDescription),
}

pub fn init(c: Arc<AppController>, rt: Handle) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        rouille::start_server("0.0.0.0:80", move |request| {
            router!(request,
                // first route
                (GET) (/) => {
                    serve_index(&request)
                },

                (POST) (/signal/{uid: String}) => {
                    signal(&request, c.clone(), rt.clone(), uid)
                },

                // default route
                _ => Response::text("Endpoint not found").with_status_code(400)
            )
        })
    })
}

fn serve_index(request: &Request) -> Response {
    if cfg!(debug_assertions) {
        Response::from_file(
            "text/html",
            std::fs::File::open("public/index.html").expect("Unable to read index file."),
        )
    } else {
        Response::html(include_str!("../public/index.html"))
    }
}

fn signal(request: &Request, a: Arc<AppController>, rt: Handle, uid: String) -> Response {
    println!("Got signalling request. UID: {}", uid);

    let mut buf = String::new();
    request.data().unwrap().read_to_string(&mut buf).unwrap();

    let offer: RTCSessionDescription = serde_json::from_str(&buf).unwrap();

    // Call the asynchronous connect method using the main runtime handle.
    // Use block_on to allow this sync request to call an async handler.
    let a_i = a.clone();
    let res = rt.block_on(async move { a_i.signal(uid, offer).await });
    println!(" Signalling done!");

    Response::text(serde_json::to_string(&res).unwrap())
}
