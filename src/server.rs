use std::{io::Read, sync::Arc};

// Powers the internal server
use rouille::{router, Request, Response};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt};
use tokio::runtime::Handle;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::{app_controller::AppController, stats::SystemStatusReader};

#[derive(Deserialize)]
struct StreamSignalling {
    stream_ids: Vec<String>,
    offer: RTCSessionDescription,
}

pub fn init(c: Arc<AppController>, rt: Handle) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        rouille::start_server("0.0.0.0:80", move |request| {
            router!(request,
                // first route
                (GET) (/) => {
                    serve_index(&request)
                },

                (POST) (/api/signal/{uid: String}) => {
                    rt.block_on(async { signal(&request, &c, uid).await })
                },
                (POST) (/api/streamsignal/{uid: String}) => {
                    rt.block_on(async { stream_signal(&request, &c, uid).await })
                },

                (GET) (/api/stats) => {
                    rt.block_on(async { stats(&c).await })
                },

                (GET) (/api/streams) => {
                    Response::json(&c.streams())
                },

                // default route
                _ => Response::text("Endpoint not found").with_status_code(400)
            )
        })
    })
}

fn serve_index(_request: &Request) -> Response {
    if cfg!(debug_assertions) {
        Response::from_file(
            "text/html",
            std::fs::File::open("public/index.html").expect("Unable to read index file."),
        )
    } else {
        Response::html(include_str!("../public/index.html"))
    }
}

async fn signal(request: &Request, a: &Arc<AppController>, uid: String) -> Response {
    println!("Got signalling request. UID: {}", uid);

    let mut buf = String::new();
    request.data().unwrap().read_to_string(&mut buf).unwrap();

    let offer: RTCSessionDescription = serde_json::from_str(&buf).unwrap();

    let a_i = a.clone();

    match a_i.signal(&uid, offer).await {
        Ok(res) => Response::json(&res),
        Err(e) => {
            eprintln!("Signalling request failed: {}", e);
            Response::text(e.to_string()).with_status_code(500)
        }
    }
}

async fn stream_signal(request: &Request, a: &Arc<AppController>, uid: String) -> Response {
    println!("Got STREAM signalling request. UID: {}", uid);

    let mut buf = String::new();
    request.data().unwrap().read_to_string(&mut buf).unwrap();

    let signalling: StreamSignalling = serde_json::from_str(&buf).unwrap();

    let a_i = a.clone();
    a_i.client_sync_streams(&uid, signalling.stream_ids).await;

    match a_i.signal(&uid, signalling.offer).await {
        Ok(res) => Response::json(&res),
        Err(e) => {
            eprintln!("Signalling request failed: {}", e);
            Response::text(e.to_string()).with_status_code(500)
        }
    }
}

async fn stats(a: &Arc<AppController>) -> Response {
    let stats = a.stats().await;
    Response::json(&stats)
}
