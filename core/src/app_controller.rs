use std::{
    collections::{HashMap},
    sync::{Arc},
};

use crate::{
    client::Client,
    stats::{SystemStatus, SystemStatusReader},
    stream_manager::StreamManager, track_def::StreamDef,
};
use anyhow::Result;
use serde::Serialize;
use tokio::sync::{RwLock};
use webrtc::peer_connection::{
    sdp::session_description::RTCSessionDescription,
};

#[derive(Serialize, Clone)]
pub struct AppStats {
    system_status: SystemStatus,
    clients: usize,
}
pub struct AppController {
    stream_manager: Arc<StreamManager>,

    clients: Arc<RwLock<HashMap<String, Arc<Client>>>>,
    sys_stats: SystemStatusReader,
}

impl AppController {
    pub fn new(stream_manager: Arc<StreamManager>) -> AppController {
        AppController {
            stream_manager,
            sys_stats: SystemStatusReader::new(),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn ensure_client(&self, client_id: &String) -> Result<Arc<Client>> {
        let client_exists = self.clients.read().await.contains_key(client_id);

        if !client_exists {
            self.initialize_client(client_id).await
        } else {
            let clients_guard = self.clients.read().await;
            let client_arc = clients_guard.get(client_id).unwrap();
            Ok(client_arc.clone())
        }
    }

    pub async fn initialize_client(&self, client_id: &String) -> Result<Arc<Client>> {
        let mut m = self.clients.write().await;
        let c = Client::new(self.stream_manager.clone()).await?;
        m.insert(client_id.clone(), c.clone());
        drop(m);

        // Spawn kill watcher. Deallocs and cleans up after clients are closed.
        let c_inner = c.clone();
        let id_inner = client_id.clone();
        let clients = self.clients.clone();
        tokio::spawn(async move {
            // Wait for client to fail
            c_inner.watch_fail().changed().await.unwrap();
            drop(c_inner);

            // Remove the client from our list
            let mut m = clients.write().await;
            let c = m.remove(&id_inner).unwrap();

            // Finalize the client and drop it.
            // This should deallocate the client (strong arc = 0)
            // which should stop any related tasks holding a weak as well
            c.discard().await;
            drop(c);
        });

        // TEST: ADD STREAM

        Ok(c.clone())
    }

    pub async fn streams(&self) -> Vec<StreamDef> {
        self.stream_manager.stream_defs().await
    }

    pub async fn stats(&self) -> AppStats {
        AppStats {
            system_status: self.sys_stats.stats().await,
            clients: self.clients.read().await.len(),
        }
    }

    pub async fn client(&self, client_id: &String) -> Result<Arc<Client>> {
        self.ensure_client(&client_id).await
    }
}
