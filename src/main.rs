mod api;

mod rtp_track;
//mod server;
mod stream_peer;
mod buffered_track;
use std::net::{IpAddr, Ipv4Addr};

use client::Client;
use stream_manager::StreamManager;
use structopt::StructOpt;
use webrtc::{
    api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8},
    peer_connection::sdp::session_description::RTCSessionDescription,
};
mod client;
mod net_util;
mod stream_manager;
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// File name: only required when `out-type` is set to `file`
    #[structopt(name = "Offer")]
    offer: String,
}

pub struct Config {}

#[derive(Hash, Clone, PartialEq, Eq)]
pub struct StreamDef {
    id: String,    // Stream ID. Should be unique
    default: bool, // Added by default when a new client connects?
    video: Option<TrackDef>,
    audio: Option<TrackDef>,
}

#[derive(Hash, Clone, PartialEq, Eq)]
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
            codec: "vp8".to_string(),
            port: 5002,
            ip: Some("239.7.69.7".parse::<IpAddr>().unwrap()),
        }),
        audio: None,
    };

    let test_defs = vec![test_stream.clone()];

    let mut sm = StreamManager::new();
    

    let offer: RTCSessionDescription =
        serde_json::from_str(&std::fs::read_to_string("./session.test").unwrap()).unwrap();

    let mut client = client::Client::new().await;



    let stream = sm.create_stream(test_stream);
    client.add_stream(stream).await;

    let res = client.offer(offer).await;
    tokio::fs::write("./answer.test", serde_json::to_string(&res).unwrap()).await.unwrap();

    println!("OFFER CREATED");

    //let sm = StreamManager::new(std::net::IpAddr::V4(Ipv4Addr::new(224, 2, 127, 254)), 9875);
    //let stream = rtp_stream::RtpStream::new(5000);

    //client.set_track_stream(0, &stream);

    //let mut streams: Vec<rtp_stream::RtpStream> = vec![rtp_stream::RtpStream::new(3333)];

    // Keep the process alive.

   // let (tx, rx) = server::init();

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
    }
    // loop {
    //     poll_api().await;
    // }

    // let mut input = String::new();



    // std::io::stdin().read_line(&mut input)?;

    // let v: api::RXMessages = serde_json::from_str(&input)?;

    // match v {
    //     api::RXMessages::WebrtcOffer => todo!(),
    // }
}

// async fn poll_api() -> Result<api::RXMessages> {
    
// }

async fn add_client() {}

// ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libvpx -cpu-used 5 -deadline 1 -g 3 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:5000?pkt_size=1200
// ./ffprobe lel.sdp -protocol_whitelist rtp,file,udp -show_frames

// SAP test
///////// ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -vcodec libvpx -cpu-used 5 -deadline 1 -g 3 -f sap sap://224.2.127.254:5000

// ./ffprobe -f sap sap://224.2.127.254 -show_frames

//////// ./ffprobe -f sap sap://224.2.127.254 -show_frames -show_entries frame=key_frame -print_format csv -loglevel panic

// Remux:
// ./ffmpeg -re -i sap:// -vcodec copy -f rtp rtp://127.0.0.1:9553?pkt_size=1316

// ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libvpx -cpu-used 5 -deadline 1 -g 3 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:5000?pkt_size=1200
