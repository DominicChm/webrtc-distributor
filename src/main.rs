mod api;
mod ffmpeg;
mod rtp_stream;
mod server;
mod stream_peer;
mod track_feeder;
use anyhow::Result;
use structopt::StructOpt;
mod client;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// File name: only required when `out-type` is set to `file`
    #[structopt(name = "Offer")]
    file_name: Option<String>,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let mut sinks: Vec<stream_peer::StreamPeer> = Vec::new();
    //let mut streams: Vec<rtp_stream::RtpStream> = vec![rtp_stream::RtpStream::new(3333)];

    loop {
        poll_api().await;
    }
}

async fn poll_api() -> Result<api::RXMessages> {
    let mut input = String::new();

    std::thread::spawn(move || {
        server::init();
    });

    std::io::stdin().read_line(&mut input)?;

    let v: api::RXMessages = serde_json::from_str(&input)?;

    match v {
        api::RXMessages::WebrtcOffer => todo!(),
    }
}

async fn add_client() {}
// ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -vcodec libvpx -cpu-used 5 -deadline 1 -g 10 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:3333?pkt_size=1200
