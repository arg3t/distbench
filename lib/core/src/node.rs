//! Node implementation for distributed algorithms.
//!
//! This module provides the core [`Node`] structure that represents a participant
//! in a distributed system, along with supporting types for peer communication.

use async_trait::async_trait;
use log::{debug, error};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Notify};

use crate::algorithm::Algorithm;
use crate::community::{Community, PeerId};
use crate::messages::NodeMessage;
use crate::status::NodeStatus;
use crate::transport::{self, ConnectionManager, Server, Transport, TransportError};
use crate::SelfTerminating;
use crate::NODE_ID_CTX;

/// Internal peer representation for node-to-node communication.
///
/// Wraps a connection manager and provides methods to send node lifecycle messages.
struct NodePeer<T, CM>
where
    T: Transport,
    CM: ConnectionManager<T>,
{
    conn_manager: Arc<CM>,
    _phantom: PhantomData<T>,
}

impl<T, CM> NodePeer<T, CM>
where
    T: Transport,
    CM: ConnectionManager<T>,
{
    /// Creates a new `NodePeer` with the given connection manager.
    pub fn new(conn_manager: Arc<CM>) -> Self {
        Self {
            conn_manager,
            _phantom: PhantomData,
        }
    }

    /// Notifies the peer that this node has started.
    pub async fn started(&self) -> Result<(), TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::Started)?)
            .await
    }

    /// Notifies the peer that this node has finished.
    pub async fn finished(&self) -> Result<(), TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::Finished)?)
            .await
    }

    /// Requests the public key from the peer.
    #[allow(dead_code)]
    pub async fn get_pubkey(&self) -> Result<(), TransportError> {
        self.conn_manager
            .cast(serde_json::to_vec(&NodeMessage::GetPubKey)?)
            .await
    }
}

/// A node in the distributed system.
///
/// A `Node` represents a single participant in a distributed algorithm execution.
/// It manages the node's lifecycle (starting, running, stopping, termination),
/// coordinates with peers in the community, and executes the assigned algorithm.
///
/// # Type Parameters
///
/// * `T` - The transport layer implementation (e.g., TCP, channels)
/// * `A` - The algorithm implementation to execute on this node
///
/// # Examples
///
/// ```ignore
/// use framework::{Node, community::Community};
/// use std::sync::Arc;
///
/// // Assuming you have a community and algorithm set up
/// let node = Node::new(
///     peer_id,
///     Arc::new(community),
///     Arc::new(algorithm),
/// )?;
/// ```
pub struct Node<T, CM, A>
where
    T: Transport,
    CM: ConnectionManager<T>,
    A: Algorithm,
{
    id: PeerId,
    status: Arc<watch::Sender<NodeStatus>>,
    status_rx: Arc<tokio::sync::Mutex<watch::Receiver<NodeStatus>>>,
    community: Arc<Community<T, CM>>,
    algo: Arc<A>,
}

impl<T: Transport, CM: ConnectionManager<T>, A: Algorithm> Clone for Node<T, CM, A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            status: Arc::clone(&self.status),
            status_rx: Arc::clone(&self.status_rx),
            community: Arc::clone(&self.community),
            algo: Arc::clone(&self.algo),
        }
    }
}

impl<T, CM, A> Node<T, CM, A>
where
    T: Transport + 'static,
    CM: ConnectionManager<T> + 'static,
    A: Algorithm + 'static,
{
    /// Creates a new node with the specified ID, community, and algorithm.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this node
    /// * `community` - The community this node belongs to
    /// * `algo` - The algorithm instance to execute
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if the node cannot be initialized.
    pub fn new(
        id: PeerId,
        community: Arc<Community<T, CM>>,
        algo: Arc<A>,
    ) -> Result<Self, crate::error::ConfigError> {
        let (status_tx, status_rx) = watch::channel(NodeStatus::NotStarted);
        Ok(Self {
            id,
            community,
            algo,
            status: Arc::new(status_tx),
            status_rx: Arc::new(tokio::sync::Mutex::new(status_rx)),
        })
    }

    /// Returns the ID of this node.
    pub fn id(&self) -> &PeerId {
        &self.id
    }

    /// Monitors the node's lifecycle and coordinates state transitions.
    ///
    /// This method runs in a separate task and handles:
    /// - Synchronizing startup with other nodes
    /// - Calling algorithm lifecycle hooks (`on_start`, `on_exit`)
    /// - Coordinating shutdown when the algorithm terminates
    async fn monitor(&self) {
        let mut status_rx = self.status.subscribe();
        let mut algo_terminated = false;

        loop {
            let status = status_rx.borrow_and_update().clone();
            match status {
                NodeStatus::Starting => {
                    let mut futures = Vec::new();

                    for conn in self.community.all_peers() {
                        let peer = NodePeer::new(conn);
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
                        let peer = NodePeer::new(conn);
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

    /// Starts the node and begins serving requests.
    ///
    /// This spawns background tasks for monitoring and serving, then transitions
    /// the node to the `Starting` state.
    ///
    /// # Arguments
    ///
    /// * `stop_signal` - A signal that can be used to stop the node externally
    ///
    /// # Returns
    ///
    /// A `JoinHandle` for the serving task, which completes when the node stops.
    ///
    /// # Errors
    ///
    /// Returns a `TransportError` if the node fails to start serving.
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
impl<T, CM, A> Server<T> for Node<T, CM, A>
where
    T: Transport + 'static,
    CM: ConnectionManager<T> + 'static,
    A: Algorithm + 'static,
{
    async fn handle(&self, src: &T::Address, msg: Vec<u8>) -> transport::Result<Option<Vec<u8>>> {
        let peer_id = self
            .community
            .id_of(&src)
            .ok_or(TransportError::UnknownPeer {
                addr: src.to_string(),
            })?;

        let msg = serde_json::from_slice(&msg)?;

        let result = match msg {
            NodeMessage::Algorithm(msg_type, msg) => self
                .algo
                .handle(peer_id, msg_type, msg)
                .await
                .map_err(|e| TransportError::AlgorithmError {
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
impl<T, CM, A> SelfTerminating for Node<T, CM, A>
where
    T: Transport + 'static,
    CM: ConnectionManager<T> + 'static,
    A: Algorithm + 'static,
{
    async fn terminate(&self) {
        let _ = self.status.send(NodeStatus::Stopping);
    }

    async fn terminated(&self) -> bool {
        let mut status_rx = self.status_rx.lock().await;
        let _ = status_rx
            .wait_for(|status| *status == NodeStatus::Terminated)
            .await;
        true
    }
}
