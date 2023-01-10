use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use serde::{Deserialize, Serialize};
use webrtc::{
    api::media_engine::{MIME_TYPE_H264, MIME_TYPE_VP8},
    rtp::packet::Packet,
};

#[derive(Hash, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StreamDef {
    pub id: String,    // Stream ID. Should be unique
    pub default: bool, // Added by default when a new client connects?
    pub video: Option<TrackDef>,
    pub audio: Option<TrackDef>,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Codec {
    H264,
    VP8,
}

impl Codec {
    pub fn mime_type(&self) -> &str {
        match self {
            Codec::H264 => MIME_TYPE_H264,
            Codec::VP8 => MIME_TYPE_VP8,
        }
    }
}

#[derive(Hash, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TrackDef {
    pub port: u16,          // Stream port
    pub ip: Option<IpAddr>, // Optional IP to get stream from. Used for multicast addresses. Default is localhost
    pub codec: Codec,       // Codec used.
}

impl TrackDef {
    pub fn codec(&self) -> Codec {
        self.codec
    }

    pub fn stream_id(&self) -> &str {
        match self.codec() {
            Codec::H264 | Codec::VP8 => "video"
        }
    }

    pub fn socket_addr(&self) -> SocketAddr {
        let ip = self.ip.unwrap_or(IpAddr::from(Ipv4Addr::LOCALHOST));
        SocketAddr::new(ip, self.port)
    }

    pub fn is_keyframe(&self, pkt: &Packet) -> bool {
        let mime = self.codec.mime_type();
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
