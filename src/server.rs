use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time,
};

// Powers the internal server
use crate::{app_controller::AppController, stats::SystemStatusReader};
use anyhow::{anyhow, bail, Result};
use rouille::{extension_to_mime, router, Request, Response};
use rust_embed::RustEmbed;
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

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

pub fn init(c: Arc<AppController>, rt: Handle) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        rouille::start_server("0.0.0.0:80", move |request| {
            router!(request,
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
                _ => serve_default(&request)
            )
        })
    })
}

fn serve_default(request: &Request) -> Response {
    match_embedded_asset(request, "index.html")
        .unwrap_or_else(|e| Response::text(e.to_string()).with_status_code(400))
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

pub fn match_embedded_asset(request: &Request, root: &str) -> Result<Response> {
    let raw_path = if request.url().len() <= 1 {
        root.to_string()
    } else {
        request.url().split_at(1).1.to_string()
    };

    eprintln!("SERVING: {}", &raw_path);

    let file = Assets::get(&raw_path).ok_or(anyhow!("No file found"))?;

    let path = request.url().parse::<PathBuf>()?;
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("html");

    Ok(Response::from_data(extension_to_mime(ext), file.data))
}
