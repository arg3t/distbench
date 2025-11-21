use std::{
    collections::HashMap,
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use common_macros::hash_map;
use distbench::{self, community::PeerId, Algorithm};
use log::info;

use crate::algorithms::echo::{Echo, EchoConfig};

#[distbench::message]
struct Message {
    sender: String,
    message: String,
}

#[distbench::state(comm = Echo)]
pub struct Test {
    #[distbench::config(default = false)]
    start_node: bool,
    messages_received: AtomicU64,
}

#[async_trait]
impl Algorithm for Test {
    async fn on_start(&self) {}

    async fn on_exit(&self) {
        info!("Echo algorithm exiting");
    }

    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> {
        Some(hash_map! {
            "messages_received" => self.messages_received.load(Ordering::Relaxed).to_string(),
        })
    }
}

#[distbench::handlers]
impl Test {
    async fn message(&self, src: PeerId, msg: &Message) -> Option<String> {
        info!("Received message from {}: {}", src.to_string(), msg.message);
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        Some(msg.message.clone())
    }
}
