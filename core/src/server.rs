use std::{
    borrow::Cow,
    f32::consts::E,
    ffi::OsStr,
    future::Future,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

// Powers the internal server
use crate::{app_controller::AppController, client::Client};
use anyhow::{anyhow, Result};
use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use rust_embed::RustEmbed;
use serde::Deserialize;
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
#[folder = "../frontend/dist/"]
struct Assets;

const ASSET_ROOT: &str = "index.html";

type Controller = Arc<AppController>;

pub async fn init(controller: Arc<AppController>) {
    let api = Router::new()
        .route("/signal", post(handle_signalling))
        .route("/resync", post(handle_rtp_resync))
        .route("/stats", get(handle_stats))
        .route("/streams", get(handle_streams));

    let app = Router::new()
        .route("/", get(handle_asset))
        .nest("/api", api)
        .with_state(controller);

    let addr = SocketAddr::from(([127, 0, 0, 1], 80));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_asset(uri: Uri) -> Result<impl IntoResponse, (StatusCode, String)> {
    let raw_path = if uri.path().len() <= 1 {
        uri.path()
    } else {
        ASSET_ROOT
    };

    let path = Path::new(raw_path)
        .strip_prefix("/")
        .or(Err((StatusCode::NOT_FOUND, "Malformed path".into())))?;

    eprintln!("SERVING: {}", path.display());

    let file = Assets::get(&raw_path).ok_or((
        StatusCode::NOT_FOUND,
        format!("{} not found.", path.display()),
    ))?;

    let mime = mime_guess::from_path(path)
        .first_or_text_plain()
        .to_string();

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, mime)], file.data))
}

async fn handle_signalling(
    State(cont): State<Controller>,
    Json(req): Json<SignallingRequest>,
) -> impl IntoResponse {
    println!("Got signalling request");

    let client = get_client(&cont, &req.uid).await?;

    // Sync streams of client with passed UID
    client.sync_active_streams(req.stream_ids).await;

    // Do webrtc signalling
    client
        .signal(req.offer)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .map(|v| Json(v))
}

async fn handle_rtp_resync(
    State(cont): State<Controller>,
    Json(req): Json<SyncRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let client = get_client(&cont, &req.uid).await?;

    client.resync_rtp_streams(req.stream_ids).await;

    Ok(StatusCode::OK)
}

async fn handle_stats(State(ctx): State<Controller>) -> impl IntoResponse {
    Json(ctx.stats().await)
}

async fn handle_streams(State(ctx): State<Controller>) -> impl IntoResponse {
    Json(ctx.streams().await)
}

/**
 * Helper function to get a client from the AppController in a way that can
 * gracefully error into a response. Cuts down on some boilerplate.
 */
async fn get_client(cont: &Controller, uid: &String) -> Result<Arc<Client>, (StatusCode, String)> {
    cont.client(uid).await.map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            format!("Client could not be found or created. ({})", e),
        )
    })
}
