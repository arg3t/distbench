pub mod community;
pub mod error;
pub mod transport;

pub use error::PeerError;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::community::PeerId;

pub trait AlgorithmHandler {
    async fn handle(
        &self,
        src: PeerId,
        msg_type_id: String,
        msg_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}
pub trait Algorithm {
    fn init(&self) {}
    fn on_start(&self) {}
    fn on_exit(&self) {}
}

#[derive(Serialize, Deserialize)]
pub enum NodeMessage {
    Started,
    Finished,
    GetPubKey,
    Algorithm(String, Vec<u8>),
}

pub struct Node<T: transport::Transport, A: Algorithm + AlgorithmHandler> {
    id: PeerId,
    address: T::Address,
    community: community::Community<T>,
    transport: T,
    algo: Arc<A>,
}

impl<T: transport::Transport, A: Algorithm + AlgorithmHandler> Node<T, A> {
    pub fn new(
        id: PeerId,
        address: T::Address,
        transport: T,
        community: community::Community<T>,
        algo: Arc<A>,
    ) -> Self {
        Self {
            id,
            address,
            transport,
            community,
            algo,
        }
    }

    pub async fn start(self) -> transport::Result<()> {
        let transport = self.transport.clone();
        let server = Arc::new(self);
        transport.serve(server).await
    }
}

impl<T: transport::Transport, A: Algorithm + AlgorithmHandler> transport::Server<T>
    for Arc<Node<T, A>>
{
    async fn handle(&self, src: T::Address, msg: Vec<u8>) -> transport::Result<Vec<u8>> {
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
}
