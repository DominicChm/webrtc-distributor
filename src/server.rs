use std::{io::Read, sync::Arc};

// Powers the internal server
use crate::{app_controller::AppController, stats::SystemStatusReader};
use anyhow::{anyhow, Result};
use rouille::{router, Request, Response};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt};
use tokio::runtime::Handle;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Deserialize)]
struct SignallingRequest {
    stream_ids: Vec<String>,
    uid: String,
    offer: RTCSessionDescription,
}

#[derive(Deserialize)]
struct SyncRequest {
    stream_ids: Vec<String>,
    uid: String,
}

pub fn init(c: Arc<AppController>, rt: Handle) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        rouille::start_server("0.0.0.0:80", move |request| {
            router!(request,
                // Hosts main page
                (GET) (/) => {
                    serve_index(&request)
                },

                // WebRTC Signalling API
                // Controls what streams are being sent and WebRTC signalling
                (POST) (/api/signal) => {
                    rt.block_on(async {
                        match signal(&request, &c).await {
                            Ok(r) => r,
                            Err(e) => Response::text(e.to_string()).with_status_code(500),
                        }
                     })
                },

                //
                (POST) (/api/resync) => {
                    rt.block_on(async {
                        match resync(&request, &c).await {
                            Ok(r) => r,
                            Err(e) => Response::text(e.to_string()).with_status_code(500),
                        }
                     })
                },

                // Pollable endpoint with stats about system
                (GET) (/api/stats) => {
                    rt.block_on(async { stats(&c).await })
                },

                // Pollable endpoint with info about all available streams.
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

async fn signal(request: &Request, app_controller: &Arc<AppController>) -> Result<Response> {
    println!("Got signalling request");

    // Parse incoming request into a SignallingRequest object
    let mut buf = String::new();
    request
        .data()
        .ok_or(anyhow!("No request data received"))?
        .read_to_string(&mut buf)?;
    let signalling: SignallingRequest = serde_json::from_str(&buf).unwrap();

    let app_controller = app_controller.clone();

    // Sync streams of client with passed UID
    app_controller
        .client_sync_streams(&signalling.uid, signalling.stream_ids)
        .await?;

    let signal_result = app_controller
        .signal(&signalling.uid, signalling.offer)
        .await;

    match signal_result {
        Ok(res) => Ok(Response::json(&res)),
        Err(e) => {
            eprintln!("Signalling request failed: {}", e);
            Err(anyhow!(e.to_string()))
        }
    }
}

async fn resync(request: &Request, app_controller: &Arc<AppController>) -> Result<Response> {
    // Parse incoming request into a SignallingRequest object
    let mut buf = String::new();
    request
        .data()
        .ok_or(anyhow!("No request data received"))?
        .read_to_string(&mut buf)?;

    let req: SyncRequest = serde_json::from_str(&buf).unwrap();

    app_controller
        .client_resync_streams(&req.uid, req.stream_ids)
        .await
        .map(|_| Response::text("OK").with_status_code(200))
}

async fn stats(a: &Arc<AppController>) -> Response {
    let stats = a.stats().await;
    Response::json(&stats)
}
