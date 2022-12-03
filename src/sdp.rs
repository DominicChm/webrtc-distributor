use std::net::{IpAddr, Ipv4Addr};
use lazy_static::lazy_static;
use regex::Regex;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Codec {
    H264,
    OPUS,
    VP8,
    VP9,
    AV1,
    G722,
    PCMU,
    PCMA,
}

impl std::str::FromStr for Codec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VP8" => Ok(Codec::VP8),
            "H264" => Ok(Codec::H264),
            _ => Err(anyhow::Error::msg("Couldn't find codec")),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum StreamType {
    AUDIO,
    VIDEO,
}

impl std::str::FromStr for StreamType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "video" => Ok(StreamType::VIDEO),
            "audio" => Ok(StreamType::AUDIO),
            _ => Err(anyhow::Error::msg("Couldn't find stream type")),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Sdp {
    pub origin: IpAddr,
    pub connection: IpAddr,
    pub streams: Vec<MediaStream>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct MediaStream {
    pub stream_type: StreamType,
    pub port: u16,
    pub codec: Codec,
}

/**
 * TODO: Test with multiple streams
 */
pub fn parse_sdp(sdp_str: String) -> Sdp {
    lazy_static! {
        static ref RE_CONNECTION: Regex = Regex::new(r"([\d.]+)/").unwrap();
        static ref RE_MEDIA: Regex = Regex::new(r"=(\w+) (\d+)").unwrap();
        static ref RE_ATTR: Regex = Regex::new(r"(\w+)/").unwrap();
    }

    let mut sdp = Sdp {
        connection: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        origin: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        streams: Vec::new(),
    };

    let mut media_stream: Option<MediaStream> = None;

    for line in sdp_str.lines() {
        match line.chars().nth(0) {
            Some('m') => {
                let caps = RE_MEDIA.captures(line).expect("regex problem");
                if media_stream.is_some() {
                    sdp.streams.push(media_stream.unwrap().clone());
                }
                media_stream = Some(MediaStream {
                    codec: Codec::H264,
                    stream_type: caps
                        .get(1)
                        .expect("no stream type result")
                        .as_str()
                        .parse::<StreamType>()
                        .expect("stream type parse error"),
                    port: caps
                        .get(2)
                        .expect("no port result")
                        .as_str()
                        .parse::<u16>()
                        .expect("port parse error"),
                });
            }
            Some('a') => {
                let caps = RE_ATTR.captures(line);
                if caps.is_some() && media_stream.is_some() {
                    media_stream.unwrap().codec = caps
                        .unwrap()
                        .get(1)
                        .expect("no codec result")
                        .as_str()
                        .parse::<Codec>()
                        .expect("port parse error");
                }
            }
            Some('c') => {
                sdp.connection = RE_CONNECTION
                    .captures(line)
                    .expect("conn regex")
                    .get(1)
                    .expect("no ip result")
                    .as_str()
                    .parse::<IpAddr>()
                    .expect("ip parse error");
            }
            Some(_) => {}
            None => {}
        }
    }

    if media_stream.is_some() {
        sdp.streams.push(media_stream.unwrap().clone());
    }

    sdp
}
