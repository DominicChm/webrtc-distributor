mod ffmpeg;
mod webrtc_sink;
mod rtp_stream;
mod api;
mod server;

use anyhow::Result;

#[tokio::main]
async fn main() {
    let mut sinks: Vec<webrtc_sink::WebrtcSink> = Vec::new();
    let mut streams: Vec<rtp_stream::RtpStream> = vec![
        rtp_stream::RtpStream::new(3333)
    ];
    
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

async fn add_client() {
    
}
// ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -vcodec libvpx -cpu-used 5 -deadline 1 -g 10 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:3333?pkt_size=1200
