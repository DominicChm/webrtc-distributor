mod controller;

mod rtp_track;
//mod server;
mod buffered_track;
mod stream_peer;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use stream_manager::StreamManager;
use structopt::StructOpt;
use webrtc::{
    api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8},
    rtp::packet::Packet,
};
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
            "h264" | "libx264" | "vp8" | "libvpx" => "video",
            _ => "UNKNOWN",
        }
    }

    fn socket_addr(&self) -> SocketAddr {
        let ip = self.ip.unwrap_or(IpAddr::from(Ipv4Addr::LOCALHOST));
        SocketAddr::new(ip, self.port)
    }

    fn is_keyframe(&self, pkt: &Packet) -> bool {
        let mime = self.mime_type().unwrap();
        match mime {
            MIME_TYPE_VP8 => {
                // https://datatracker.ietf.org/doc/html/rfc7741#section-4.3
                // https://github.com/FFmpeg/FFmpeg/blob/master/libavformat/rtpdec_vp8.c

                // Note: bit 0 is MSB
                let mut b = &pkt.payload[..];

                let x = b[0] & 0x80 != 0;
                let s = b[0] & 0x10 != 0;
                let pid = b[0] & 0x0f;

                if x {
                    b = &b[1..];
                }

                let i = x && b[0] & 0x80 != 0;
                let l = x && b[0] & 0x40 != 0;
                let t = x && b[0] & 0x20 != 0;
                let k = x && b[0] & 0x10 != 0;

                b = &b[1..];

                // Handle I
                if i && b[0] & 0x80 != 0 {
                    b = &b[2..];
                } else if i {
                    b = &b[1..];
                }

                // Handle L
                if l {
                    b = &b[1..];
                }

                // Handle T/K
                if t || k {
                    b = &b[1..];
                }

                b[0] & 0x01 == 0 && s && pid == 0
            }
            MIME_TYPE_H264 => {
                // https://stackoverflow.com/questions/1957427/detect-mpeg4-h264-i-frame-idr-in-rtp-stream
                let p = &pkt.payload;
                let fragment_type = p.get(0).unwrap() & 0x1F;
                let nal_type = p.get(1).unwrap() & 0x1F;
                let start_bit = p.get(1).unwrap() & 0x80;

                ((fragment_type == 28 || fragment_type == 29) && nal_type == 5 && start_bit == 128)
                    || fragment_type == 5
            }
            _ => false,
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

    server::init(c, tokio::runtime::Handle::current())
        .join()
        .unwrap();
}
