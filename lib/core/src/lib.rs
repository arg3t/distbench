pub mod community;
pub mod error;
pub mod transport;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use std::collections::HashMap;
use std::sync::Arc;

use crate::community::PeerId;
pub use error::{ConfigError, PeerError};
pub use procs::handlers;
pub use procs::message;
pub use procs::setup;

#[async_trait]
pub trait AlgorithmHandler {
    async fn handle(
        &self,
        src: PeerId,
        msg_type_id: String,
        msg_bytes: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
pub trait Algorithm<T: transport::Transport>: AlgorithmHandler + Send + Sync {
    async fn on_start(&self) {}
    async fn on_exit(&self) {}
}

pub trait AlgorithmBuilder<T: transport::Transport> {
    type Algorithm: Algorithm<T>;
    type Peer;
    fn build(self, peers: HashMap<PeerId, Self::Peer>)
        -> Result<Arc<Self::Algorithm>, ConfigError>;
}

#[derive(Serialize, Deserialize)]
pub enum NodeMessage {
    Started,
    Finished,
    GetPubKey,
    Algorithm(String, Vec<u8>),
}

pub enum NodeStatus {
    Starting,
    Running,
    Finished,
}

pub struct Node<T: transport::Transport, A: AlgorithmBuilder<T>> {
    status: Arc<watch::Sender<NodeStatus>>,
    community: community::Community<T>,
    transport: Arc<T>,
    algo: Arc<A::Algorithm>,
}

impl<T: transport::Transport + 'static, A: AlgorithmBuilder<T> + 'static> Node<T, A> {
    pub fn new(
        transport: Arc<T>,
        community: community::Community<T>,
        builder: A,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            transport,
            community,
            algo: builder.build(Default::default())?,
            status: Arc::new(watch::channel(NodeStatus::Starting).0),
        })
    }

    pub async fn start(self) -> transport::Result<()> {
        let transport = self.transport.clone();
        let server = Arc::new(self);
        transport.serve(server).await
    }
}

#[async_trait]
impl<T: transport::Transport, A: AlgorithmBuilder<T>> transport::Server<T> for Arc<Node<T, A>> {
    async fn handle(&self, src: &T::Address, msg: Vec<u8>) -> transport::Result<Option<Vec<u8>>> {
        let peer_id = self
            .community
            .peer(&src)
            .ok_or(transport::TransportError::UnknownPeer {
                addr: src.to_string(),
            })?;

        let msg = serde_json::from_slice(&msg)?;

        let result = match msg {
            NodeMessage::Algorithm(msg_type, msg) => self
                .algo
                .handle(peer_id, msg_type, msg)
                .await
                .map_err(|e| transport::TransportError::AlgorithmError {
                    message: e.to_string(),
                })?,
            _ => todo!(),
        };

        Ok(result)
    }

    async fn stopped(&self) {
        let mut status_rx = self.status.subscribe();

        loop {
            let _ = status_rx.changed().await;

            let status = status_rx.borrow_and_update();
            match *status {
                NodeStatus::Finished => break,
                _ => continue,
            }
        }
    }
}
