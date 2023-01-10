mod app_controller;

mod rtp_track;
mod track_def;
//mod server;
mod buffered_track;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
mod stats;
use serde::{Deserialize, Serialize};
use stream_manager::StreamManager;
use structopt::StructOpt;
use track_def::{StreamDef, TrackDef};
mod client;
mod net_util;
mod server;
mod stream_manager;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// File name: only required when `out-type` is set to `file`
    #[structopt(name = "Offer")]
    offer: String,
}

pub struct Config {}

// https://jsfiddle.net/xq6eua2k/1/
#[tokio::main]
async fn main() {
    //let opt = Opt::from_args();
    let test_stream = StreamDef {
        id: "test_stream".to_string(),
        default: true,
        video: Some(TrackDef {
            codec: track_def::Codec::H264,
            port: 5002,
            ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
        }),
        audio: None,
    };

    // let test_stream2 = StreamDef {
    //     id: "test_stream_2".to_string(),
    //     default: true,
    //     video: Some(TrackDef {
    //         codec: "h264".to_string(),
    //         port: 5002,
    //         ip: Some("239.7.69.7".parse::<IpAddr>().unwrap()),
    //     }),
    //     audio: None,
    // };
    let sm = Arc::new(StreamManager::new());

    sm.create_stream(test_stream).await;
    //sm.create_stream(test_stream2);

    let c = Arc::new(app_controller::AppController::new(sm));

    server::init(c, tokio::runtime::Handle::current())
        .join()
        .unwrap();
}
