use std::{
    collections::HashMap,
    fmt::Display,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use common_macros::hash_map;
use distbench::{self, community::PeerId, Algorithm, SelfTerminating};
use log::info;

use crate::algorithms::simple_broadcast::{BroadcastMessage, SimpleBroadcast, SimpleBroadcastConfig};

#[distbench::message]
pub struct UpperMessage {
    pub data: Vec<u8>,
    pub dummy_field: String,
}

#[distbench::state]
pub struct SimpleBroadcastUpper {
    #[distbench::config(default = false)]
    start_node: bool,
    #[distbench::config(default = "upper_default_dummy".to_string())]
    dummy_config: String,
    messages_from_lower: AtomicU64,
    #[distbench::child]
    broadcast: Arc<SimpleBroadcast>,
}

#[async_trait]
impl Algorithm for SimpleBroadcastUpper {
    async fn on_start(&self) {
        info!("SimpleBroadcastUpper starting with dummy_config: {}", self.dummy_config);

        if self.start_node {
            info!("SimpleBroadcastUpper: Initiating broadcast through lower layer");
            let test_data = b"Hello from upper layer!".to_vec();
            self.broadcast.broadcast(test_data).await;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
        self.terminate().await;
    }

    async fn on_exit(&self) {
        info!("SimpleBroadcastUpper exiting");
    }

    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> {
        Some(hash_map! {
            "messages_from_lower" => self.messages_from_lower.load(Ordering::Relaxed).to_string(),
            "dummy_config" => self.dummy_config.clone(),
        })
    }
}

#[distbench::handlers]
impl SimpleBroadcastUpper {
    async fn upper_message(&self, src: PeerId, msg: &UpperMessage) -> Option<String> {
        info!(
            "SimpleBroadcastUpper: Received direct message from {} with {} bytes (dummy: {})",
            src,
            msg.data.len(),
            msg.dummy_field
        );
        Some("Acknowledged".to_string())
    }
}

#[distbench::handlers(from = broadcast)]
impl SimpleBroadcastUpper {
    async fn broadcast_message(&self, src: PeerId, msg: &BroadcastMessage) -> Option<String> {
        info!(
            "SimpleBroadcastUpper: Intercepted message from lower layer! Source: {}, {} bytes (dummy: {})",
            src,
            msg.content.len(),
            msg.dummy_field
        );
        self.messages_from_lower.fetch_add(1, Ordering::Relaxed);

        // Pass through to lower layer
        Some(format!("Upper layer saw {} bytes", msg.content.len()))
    }
}
