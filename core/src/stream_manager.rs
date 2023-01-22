use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc};

use tokio::sync::RwLock;

use crate::rtp_track::RtpTrack;
use crate::track_def::StreamDef;

pub struct Stream {
    pub video: Option<Arc<RtpTrack>>,
    pub audio: Option<Arc<RtpTrack>>,
    pub def: StreamDef,
}
pub struct StreamManager {
    streams: RwLock<HashMap<String, Arc<Stream>>>,
}

impl StreamManager {
    pub fn new() -> StreamManager {
        StreamManager {
            streams: RwLock::new(HashMap::new()),
        }
    }

    pub async fn sync_tracks(&self, stream_defs: Vec<StreamDef>) {
        let current_streams: HashSet<StreamDef> = self
            .streams
            .write()
            .await
            .values()
            .cloned()
            .map(|s| s.def.clone())
            .collect();

        let incoming_streams: HashSet<StreamDef> = HashSet::from_iter(stream_defs.iter().cloned());

        let created_streams = incoming_streams.difference(&current_streams);

        // Instantiate new streams
        for stream in created_streams {
            self.create_stream(stream.clone()).await;
        }

        let deleted_streams = current_streams.difference(&incoming_streams);

        // Delete old ones
        for _stream in deleted_streams {}
    }

    pub async fn create_stream(&self, def: StreamDef) -> Arc<Stream> {
        let id = def.id.to_string();
        if self.streams.read().await.contains_key(&id) {
            panic!("Already contains stream");
        }

        let video = def
            .video
            .as_ref()
            .map(|t| Arc::new(RtpTrack::new(&t, &def)));
            
        let audio = def
            .audio
            .as_ref()
            .map(|t| Arc::new(RtpTrack::new(&t, &def)));

        let s = Arc::new(Stream {
            video,
            audio,
            def: def.clone(),
        });

        self.streams.write().await.insert(def.id.clone(), s.clone());

        s
    }

    pub async fn delete_stream(&self, id: &String) {
        self.streams.write().await.remove(id);
        todo!("Finish stream deletion");
    }

    pub async fn get_stream(&self, stream_id: &String) -> Option<Arc<Stream>> {
        self.streams.read().await.get(stream_id).clone().map(|f| f.clone())
    }

    pub async fn stream_defs(&self) -> Vec<StreamDef> {
        self.streams.read().await.values().map(|f| f.def.clone()).collect()
    }
}
