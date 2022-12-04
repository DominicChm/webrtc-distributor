mod api;
mod ffmpeg;
mod rtp_stream;
mod server;
mod stream_peer;
mod track_feeder;
use std::net::Ipv4Addr;

use client::Client;
use stream_manager::StreamManager;
use structopt::StructOpt;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
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

#[tokio::main]
async fn main() {
    //let opt = Opt::from_args();

    //let offer: RTCSessionDescription = serde_json::from_str(&opt.offer).unwrap();

    //let mut client = client::Client::new(offer).await;

    let sm = StreamManager::new(std::net::IpAddr::V4(Ipv4Addr::new(224, 2, 127, 254)), 9875);
    //let stream = rtp_stream::RtpStream::new(5000);

    //client.set_track_stream(0, &stream);

    //let mut streams: Vec<rtp_stream::RtpStream> = vec![rtp_stream::RtpStream::new(3333)];

    // Keep the process alive.
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
    }
    // loop {
    //     poll_api().await;
    // }
}

// async fn poll_api() -> Result<api::RXMessages> {
//     let mut input = String::new();

//     std::thread::spawn(move || {
//         server::init();
//     });

//     std::io::stdin().read_line(&mut input)?;

//     let v: api::RXMessages = serde_json::from_str(&input)?;

//     match v {
//         api::RXMessages::WebrtcOffer => todo!(),
//     }
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