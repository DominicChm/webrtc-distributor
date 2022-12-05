use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, HashMap},
    io::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    string::ParseError,
};
use webrtc::rtcp::packet;

use crate::{
    net_util::{join_multicast, listen_udp},
    rtp_track::RtpTrack,
};
use crate::{rtp_track, StreamDef, TrackDef};

pub struct Stream {
    pub video: Option<RtpTrack>,
    pub audio: Option<RtpTrack>,
}
pub struct StreamManager {
    streams: HashMap<StreamDef, Arc<Stream>>,
}

impl StreamManager {
    pub fn new() -> StreamManager {
        StreamManager {
            streams: HashMap::new(),
        }
    }

    pub fn sync_tracks(&mut self, stream_defs: Vec<StreamDef>) {
        let current_streams: HashSet<StreamDef> = self.streams.keys().cloned().collect();
        let incoming_streams: HashSet<StreamDef> = HashSet::from_iter(stream_defs.iter().cloned());

        let created_streams = incoming_streams.difference(&current_streams);

        // Instantiate new streams
        for stream in created_streams {
            self.create_stream(stream.clone());
        }

        let deleted_streams = current_streams.difference(&incoming_streams);

        // Delete old ones
        for stream in deleted_streams {}
    }

    pub fn create_stream(&mut self, stream: StreamDef) -> Arc<Stream> {
        let video = stream.video.as_ref().map(|t| RtpTrack::new(&t, &stream));
        let audio = stream.audio.as_ref().map(|t| RtpTrack::new(&t, &stream));

        let s = Arc::new(Stream { video, audio });
        self.streams.insert(stream, s.clone());
        s
    }

    pub fn delete_stream(&self, def: StreamDef) {}
}
// Ipv4Addr::new(224,2,127,254)
// 9875
pub fn sap_to_sdp(buf: &[u8]) -> String {
    String::from_utf8(buf[24..].to_vec()).expect("sdp to string")
}
