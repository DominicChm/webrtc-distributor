mod controller;

mod rtp_track;
//mod server;
mod buffered_track;
mod stream_peer;
use std::{net::{IpAddr, Ipv4Addr}, sync::Arc};

use client::Client;
use stream_manager::StreamManager;
use structopt::StructOpt;
use webrtc::{
    api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8},
    peer_connection::sdp::session_description::RTCSessionDescription,
};
mod client;
mod stream_manager;
mod server;
mod net_util;
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// File name: only required when `out-type` is set to `file`
    #[structopt(name = "Offer")]
    offer: String,
}

pub struct Config {}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct StreamDef {
    id: String,    // Stream ID. Should be unique
    default: bool, // Added by default when a new client connects?
    video: Option<TrackDef>,
    audio: Option<TrackDef>,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct TrackDef {
    port: u16,          // Stream port
    ip: Option<IpAddr>, // Optional IP to get stream from. Used for multicast addresses. Default is localhost
    codec: String,      // Codec used.
}

impl TrackDef {
    fn mime_type(&self) -> Result<&str, anyhow::Error> {
        match self.codec.to_ascii_lowercase().as_str() {
            "h264" | "libx264" => Ok(MIME_TYPE_H264),
            "vp8" | "libvpx" => Ok(MIME_TYPE_VP8),
            _ => Err(anyhow::Error::msg("Unsupported codec")),
        }
    }

    fn stream_id(&self) -> &str {
        match self.codec.to_ascii_lowercase().as_str() {
            ("h264" | "libx264") | ("vp8" | "libvpx") => "video",
            _ => "UNKNOWN",
        }
    }
}

// https://jsfiddle.net/xq6eua2k/1/
#[tokio::main]
async fn main() {
    //let opt = Opt::from_args();
    let test_stream = StreamDef {
        id: "test_stream".to_string(),
        default: true,
        video: Some(TrackDef {
            codec: "h264".to_string(),
            port: 5002,
            ip: Some("239.7.69.7".parse::<IpAddr>().unwrap()),
        }),
        audio: None,
    };
    let mut sm = StreamManager::new();
    sm.create_stream(test_stream);

    let c = Arc::new(controller::AppController::new(sm));

    server::init(c, tokio::runtime::Handle::current()).join().unwrap();
}