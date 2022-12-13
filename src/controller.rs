use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use webrtc::peer_connection::{
    peer_connection_state::RTCPeerConnectionState, sdp::session_description::RTCSessionDescription,
};

use crate::{
    client::Client,
    stats::{SystemStatus, SystemStatusReader},
    stream_manager::StreamManager,
    StreamDef,
};

#[derive(Serialize, Clone)]
pub struct AppStats {
    system_status: SystemStatus,
    clients: usize,
}
pub struct AppController {
    // Channel that notifies when specific client IDs should begin renegotiation
    // Note: Functions that explicitly take an offer and return a response do not
    // trigger this listener. Their reneg is intended to be handled in a sync fashion
    stream_manager: StreamManager,

    clients: Arc<RwLock<HashMap<String, Arc<Client>>>>,
    sys_stats: SystemStatusReader,

    client_renegotiation_notifier: broadcast::Sender<String>,
    client_state_notifier: broadcast::Sender<(String, RTCPeerConnectionState)>,
}

impl AppController {
    pub fn new(stream_manager: StreamManager) -> AppController {
        let (client_renegotiation_notifier, _) = broadcast::channel(10);
        let (client_state_notifier, _) = broadcast::channel(10);

        AppController {
            stream_manager,
            sys_stats: SystemStatusReader::new(),
            client_renegotiation_notifier,
            client_state_notifier,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn ensure_client(&self, client_id: &String) -> Weak<Client> {
        let client_exists = self.clients.read().await.contains_key(client_id);

        if !client_exists {
            self.initialize_client(client_id).await
        } else {
            Arc::downgrade(self.clients.read().await.get(client_id).unwrap())
        }
    }

    pub async fn initialize_client(&self, client_id: &String) -> Weak<Client> {
        let mut m = self.clients.write().await;
        let c = Arc::new(Client::new().await);
        m.insert(client_id.clone(), c.clone());
        drop(m);

        

        // Spawn kill watcher. Deallocs and cleans up after closed clients.
        let c_inner = c.clone();
        let id_inner = client_id.clone();
        let clients = self.clients.clone();
        tokio::spawn(async move {
            c_inner.watch_fail().changed().await.unwrap();
            drop(c_inner);

            let mut m = clients.write().await;
            let c = m.remove(&id_inner).unwrap();
            
            c.discard().await;
            println!(
                "Client dropped! ({} strong arcs)",
                Arc::strong_count(&c)
            );

            drop(c);
        });

        // TEST: ADD STREAM
        let s = self
            .stream_manager
            .get_stream("test_stream".to_string())
            .unwrap();

        c.add_stream(s).await;

        Arc::downgrade(&c)
    }

    pub async fn signal(
        &self,
        client_id: String,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription, anyhow::Error> {
        let c = self.ensure_client(&client_id).await;

        c.upgrade().unwrap().signal(offer).await
    }

    pub async fn client_add_stream(&self, client_id: String, stream_id: String) {
        let c = self.ensure_client(&client_id).await;

        let s = self.stream_manager.get_stream(stream_id).unwrap();

        c.upgrade().unwrap().add_stream(s).await;
    }

    pub async fn client_remove_stream(&self, _client_id: String, _stream_id: String) {
        todo!()
    }

    fn add_stream(_def: StreamDef) {}

    fn delete_stream(_id: String) {}

    pub fn streams(&self) -> Vec<StreamDef> {
        self.stream_manager.stream_defs()
    }

    pub async fn stats(&self) -> AppStats {
        AppStats {
            system_status: self.sys_stats.stats().await,
            clients: self.clients.read().await.len(),
        }
    }
}
