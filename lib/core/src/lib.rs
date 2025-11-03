pub mod community;
pub mod error;
pub mod transport;

use async_trait::async_trait;
use log::debug;
use log::error;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::sync::Notify;

use std::sync::Arc;
use std::time::Duration;

use crate::community::PeerId;
pub use error::{ConfigError, PeerError};
pub use procs::handlers;
pub use procs::message;
pub use procs::state;

tokio::task_local! {
    pub static NODE_ID_CTX: String;
}

pub fn get_node_context() -> Option<String> {
    NODE_ID_CTX.try_with(|id| id.clone()).ok()
}

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
pub trait SelfTerminating {
    async fn terminate(&self);
    async fn terminated(&self) -> bool;
}

#[async_trait]
pub trait Algorithm<T: transport::Transport>:
    AlgorithmHandler + Send + Sync + SelfTerminating
{
    async fn on_start(&self) {}
    async fn on_exit(&self) {}
}

pub trait AlgorithmFactory<T: transport::Transport> {
    type Algorithm: Algorithm<T>;
    fn build(
        self,
        community: &community::Community<T>,
    ) -> Result<Arc<Self::Algorithm>, ConfigError>;
}

#[derive(Serialize, Deserialize)]
pub enum NodeMessage {
    Started,
    Finished,
    GetPubKey,
    Algorithm(String, Vec<u8>),
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub enum NodeStatus {
    #[default]
    NotStarted,
    Starting,
    Stopping,
    Terminated,
    Running,
}

struct NodePeer<T: transport::Transport> {
    conn_manager: Arc<dyn transport::ConnectionManager<T>>,
}

impl<T: transport::Transport> NodePeer<T> {
    pub fn new(conn_manager: Arc<dyn transport::ConnectionManager<T>>) -> Self {
        Self { conn_manager }
    }

    pub async fn started(&self) -> Result<(), transport::TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::Started)?)
            .await
    }

    pub async fn finished(&self) -> Result<(), transport::TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::Finished)?)
            .await
    }

    pub async fn get_pubkey(&self) -> Result<(), transport::TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::GetPubKey)?)
            .await
    }
}

pub struct Node<T: transport::Transport, A: Algorithm<T>> {
    id: PeerId,
    status: Arc<watch::Sender<NodeStatus>>,
    community: Arc<community::Community<T>>,
    algo: Arc<A>,
}

impl<T: transport::Transport, A: Algorithm<T>> Clone for Node<T, A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            status: Arc::clone(&self.status),
            community: Arc::clone(&self.community),
            algo: Arc::clone(&self.algo),
        }
    }
}

impl<T: transport::Transport + 'static, A: Algorithm<T> + 'static> Node<T, A> {
    pub fn new(
        id: PeerId,
        community: Arc<community::Community<T>>,
        algo: Arc<A>,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            id,
            community,
            algo,
            status: Arc::new(watch::channel(NodeStatus::NotStarted).0),
        })
    }

    pub fn id(&self) -> &PeerId {
        &self.id
    }

    async fn monitor(&self) {
        let mut status_rx = self.status.subscribe();
        let mut algo_terminated = false;

        loop {
            let status = status_rx.borrow_and_update().clone();
            match status {
                NodeStatus::Starting => {
                    let mut futures = Vec::new();

                    for conn in self.community.all_peers() {
                        let peer = NodePeer::new(conn); // Assuming NodePeer::new exists
                        futures.push(async move { peer.started().await });
                    }
                    futures::future::join_all(futures).await;

                    let statuses = self.community.statuses().await;
                    if statuses
                        .values()
                        .all(|status| *status == NodeStatus::Running)
                    {
                        let _ = self.status.send(NodeStatus::Running);
                    }
                }
                NodeStatus::Running => {
                    self.algo.on_start().await;
                }
                NodeStatus::Stopping => {
                    let mut futures = Vec::new();
                    for conn in self.community.all_peers() {
                        let peer = NodePeer::new(conn); // Assuming NodePeer::new exists
                        futures.push(async move { peer.finished().await });
                    }

                    futures::future::join_all(futures).await;

                    let statuses = self.community.statuses().await;
                    if statuses
                        .values()
                        .all(|status| *status == NodeStatus::Terminated)
                    {
                        let _ = self.status.send(NodeStatus::Terminated);
                    }
                }
                NodeStatus::Terminated => {
                    self.algo.on_exit().await;
                    break;
                }
                NodeStatus::NotStarted => {}
            }

            if algo_terminated {
                if status_rx.changed().await.is_err() {
                    error!("Monitor task stopped");
                    break;
                }
            } else {
                tokio::select! {
                    res = status_rx.changed() => {
                        if res.is_err() {
                            error!("Monitor task stopped");
                            break;
                        }
                    }
                    res = self.algo.terminated() => {
                        if res {
                            self.terminate().await;
                            algo_terminated = true;
                        }
                    }
                }
            }
        }
    }

    pub async fn start(
        &self,
        stop_signal: Arc<Notify>,
    ) -> transport::Result<tokio::task::JoinHandle<transport::Result<()>>> {
        let transport = self.community.transport();
        let serve_self = self.clone();
        let monitor_self = self.clone();
        let node_id_str = self.id.to_string();
        let node_id_str_2 = self.id.to_string();

        let _ = tokio::spawn(NODE_ID_CTX.scope(node_id_str, async move {
            monitor_self.monitor().await;
        }));
        let serve_handle = tokio::spawn(NODE_ID_CTX.scope(node_id_str_2.clone(), async move {
            debug!("Serving node {}", serve_self.id.to_string());
            transport.serve(serve_self, stop_signal).await
        }));

        tokio::time::sleep(Duration::from_millis(1000)).await;

        let _ = self.status.send(NodeStatus::Starting);

        Ok(serve_handle)
    }
}

#[async_trait]
impl<T: transport::Transport + 'static, A: Algorithm<T>> transport::Server<T> for Node<T, A> {
    async fn handle(&self, src: &T::Address, msg: Vec<u8>) -> transport::Result<Option<Vec<u8>>> {
        let peer_id = self
            .community
            .id_of(&src)
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
            NodeMessage::Started => {
                self.community
                    .set_status(peer_id, NodeStatus::Running)
                    .await;

                if *self.status.borrow() == NodeStatus::Starting {
                    let statuses = self.community.statuses().await;
                    if statuses
                        .values()
                        .all(|status| *status == NodeStatus::Running)
                    {
                        self.status.send(NodeStatus::Running)?;
                    }
                }
                None
            }
            NodeMessage::Finished => {
                self.community
                    .set_status(peer_id, NodeStatus::Terminated)
                    .await;

                if *self.status.borrow() == NodeStatus::Stopping {
                    let statuses = self.community.statuses().await;
                    if statuses
                        .values()
                        .all(|status| *status == NodeStatus::Terminated)
                    {
                        self.status.send(NodeStatus::Terminated)?;
                    }
                }
                None
            }
            _ => todo!(),
        };

        Ok(result)
    }
}

#[async_trait]
impl<T: transport::Transport + 'static, A: Algorithm<T>> SelfTerminating for Node<T, A> {
    async fn terminate(&self) {
        let _ = self.status.send(NodeStatus::Stopping);
    }

    async fn terminated(&self) -> bool {
        let mut status_rx = self.status.subscribe();
        let _ = status_rx
            .wait_for(|status| *status == NodeStatus::Terminated)
            .await;
        true
    }
}
