mod app_controller;

mod rtp_track;
mod track_def;
//mod server;
mod buffered_track;
use std::{
    fs::{self, read_to_string},
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
mod stats;

use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{from_value, Value};
use stream_manager::StreamManager;
use structopt::StructOpt;
use track_def::{StreamDef, TrackDef};
mod client;
mod net_util;
mod server;
mod stream_manager;
use anyhow::{anyhow, Context, Result};

#[derive(Deserialize, Debug)]
struct Config {
    lel: String,
}

/**
 * Attempts to read JSON as the passed type. The passed string can either 
 * be raw JSON directly, or a filepath to a file containing JSON.   
 */
fn json_or_file<T: DeserializeOwned>(str: &str) -> Result<T> {
    let raw_json_obj: Value = serde_json::from_str(str).or_else(|_| {
        let path = str.parse::<PathBuf>()?;
        let dat = read_to_string(path).or(Err(anyhow!(
            "Passed argument isn't a valid file path or JSON"
        )))?;
        let val: Value = serde_json::from_str(&dat).or(Err(anyhow!(
            "Passed argument IS a valid file, but it doesn't contain valid JSON"
        )))?;
        Ok::<Value, anyhow::Error>(val)
    })?;

    from_value::<T>(raw_json_obj)
        .with_context(|| format!("Found JSON, but could not parse into {}", std::any::type_name::<T>()))
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "EasyStreamer",
    about = "Simple, low-latency browser video streamer"
)]
struct Opt {
    /// EasyStreamer configuration. Can either be a JSON file path or a JSON string.
    #[structopt(short="c", parse(try_from_str=json_or_file))]
    config: Config,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    // let test_stream = StreamDef {
    //     id: "test_stream".to_string(),
    //     default: true,
    //     video: Some(TrackDef {
    //         codec: track_def::Codec::H264,
    //         port: 5002,
    //         ip: Some("0.0.0.0".parse::<IpAddr>().unwrap()),
    //     }),
    //     audio: None,
    // };

    let sm = Arc::new(StreamManager::new());

    //sm.create_stream(test_stream).await;
    //sm.create_stream(test_stream2);

    let c = Arc::new(app_controller::AppController::new(sm));

    server::init(c).await
}
