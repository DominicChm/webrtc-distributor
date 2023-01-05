use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use crate::{
    client::Client,
    stats::{SystemStatus, SystemStatusReader},
    stream_manager::StreamManager,
    StreamDef,
};
use anyhow::Result;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use webrtc::peer_connection::{
    peer_connection_state::RTCPeerConnectionState, sdp::session_description::RTCSessionDescription,
};

#[derive(Serialize, Clone)]
pub struct AppStats {
    system_status: SystemStatus,
    clients: usize,
}
pub struct AppController {
    stream_manager: StreamManager,

    clients: Arc<RwLock<HashMap<String, Arc<Client>>>>,
    sys_stats: SystemStatusReader,
}

impl AppController {
    pub fn new(stream_manager: StreamManager) -> AppController {
        AppController {
            stream_manager,
            sys_stats: SystemStatusReader::new(),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn ensure_client(&self, client_id: &String) -> Result<Arc<Client>> {
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
        let c = Arc::new(Client::new().await?);
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

    pub async fn signal(
        &self,
        client_id: &String,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        let c = self.ensure_client(client_id).await?;
        let res = c.signal(offer).await;
        if res.is_err() {
            self.discard_client(&client_id).await;
        }

        res
    }

    async fn discard_client(&self, client_id: &String) {
        let mut clients = self.clients.write().await;

        // Remove the client from our list
        let c = clients.remove(client_id).unwrap();

        // Finalize the client and drop it.
        // This should deallocate the client (strong arc = 0)
        // which should stop any related tasks holding a weak as well
        c.discard().await;
        drop(c);
    }

    // // Use client_sync_streams to emulate this functionality.
    // pub async fn client_add_stream(&self, client_id: &String, stream_id: &String) {
    //     todo!();
    //     let c = self.ensure_client(&client_id).await;

    //     let s = self.stream_manager.get_stream(stream_id).unwrap();

    //     //c.upgrade().unwrap().add_stream(s).await;
    // }

    // pub async fn client_remove_stream(&self, _client_id: String, _stream_id: String) {
    //     todo!();
    // }

    pub async fn client_sync_streams(
        &self,
        client_id: &String,
        stream_ids: Vec<String>,
    ) -> Result<()> {
        println!("SYNCING");
        let c = self.ensure_client(&client_id).await?;

        let incoming_stream_set: HashSet<String> = HashSet::from_iter(stream_ids.into_iter());
        let current_stream_set: HashSet<String> = c.stream_ids().await;

        let added_stream_ids = incoming_stream_set.difference(&current_stream_set);
        dbg!(&added_stream_ids);
        for id in added_stream_ids {
            if let Some(s) = self.stream_manager.get_stream(id) {
                println!("Sync: Adding stream {} to client", id);
                c.add_stream(s).await;
            }
        }

        let removed_stream_ids = current_stream_set.difference(&incoming_stream_set);
        dbg!(&removed_stream_ids);
        for id in removed_stream_ids {
            if let Some(s) = self.stream_manager.get_stream(id) {
                println!("Sync: Removing stream {} from client", id);
                c.remove_stream(s).await;
            }
        }

        Ok(())
    }

    pub async fn client_resync_streams(&self, client_id: &String, stream_ids: Vec<String>) -> Result<()> {
        let c = self.ensure_client(&client_id).await?;

        for id in stream_ids {
            c.resync_stream(id).await;
        }

        Ok(())
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
