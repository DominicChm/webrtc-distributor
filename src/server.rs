use std::sync::mpsc::{Receiver, Sender};

// Powers the internal server
use rouille::{router, session, Request, Response};
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

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

pub fn init() -> (Sender<RPCResponse>, Receiver<RPC>) {
    let (tx, _rx) = tokio::sync::mpsc::channel::<RPC>(10);
    let (_tx, rx) = tokio::sync::mpsc::channel::<RPCResponse>(10);
    std::thread::spawn(move || {
        rouille::start_server("localhost:80", move |request| {
            router!(request,
                // first route
                (GET) (/) => {
                    serve_index(&request)
                },
    
                // second route
                (GET) (/edp) => {
                    Response::text("Howdy")
                },
    
                // default route
                _ => Response::text("Endpoint not found").with_status_code(400)
            )
        })
    });
    
}

fn serve_index(request: &Request) -> Response {
    session::session(request, "SID", 3600, |session| {
        let id: &str = session.id();

        // This id is unique to each client.

        // Response::text(format!("Session ID: {}", id))

        if cfg!(debug_assertions) {
            Response::from_file(
                "text/html",
                std::fs::File::open("public/index.html").expect("Unable to read index file."),
            )
        } else {
            Response::html(include_str!("../public/index.html"))
        }
    })
}

fn exec_rpc() -> Response {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Call the asynchronous connect method using the runtime.
    let inner = rt.block_on(crate::client::connect(addr));
}
