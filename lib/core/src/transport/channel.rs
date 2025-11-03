use async_trait::async_trait;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::community::PeerId;
pub use crate::transport::{Address, Connection, Result, Server, Transport, TransportError};

impl Address for PeerId {}

impl Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

type InternalMessage = (Option<oneshot::Sender<Result<Vec<u8>>>>, Vec<u8>);
type ConnectionRequest = (PeerId, oneshot::Sender<mpsc::Sender<InternalMessage>>);

pub struct ChannelConnection {
    tx: mpsc::Sender<InternalMessage>,
}

#[async_trait]
impl Connection<ChannelTransport> for ChannelConnection {
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send((Some(response_tx), msg))
            .await
            .map_err(|_| TransportError::ConnectionClosed)?;

        response_rx
            .await
            .map_err(|_| TransportError::ConnectionClosed)?
    }

    async fn cast(&self, msg: Vec<u8>) -> Result<()> {
        self.tx
            .send((None, msg))
            .await
            .map_err(|_| TransportError::ConnectionClosed)?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ChannelTransport {
    registry: Arc<RwLock<HashMap<PeerId, mpsc::Sender<ConnectionRequest>>>>,
    local_addr: PeerId,
}

impl ChannelTransport {
    pub fn new(local_addr: PeerId) -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            local_addr,
        }
    }
}

#[async_trait]
impl Transport for ChannelTransport {
    type Address = PeerId;
    type Connection = ChannelConnection;

    async fn connect(&self, addr: Self::Address) -> Result<Self::Connection> {
        let registry = self.registry.read().await;
        let tx = registry
            .get(&addr)
            .ok_or(TransportError::UnknownPeer {
                addr: addr.to_string(),
            })?
            .clone();

        let (channel_tx, channel_rx) = oneshot::channel();
        let message = (self.local_addr.clone(), channel_tx);

        tx.send(message)
            .await
            .map_err(|_| TransportError::ConnectionClosed)?;

        let tx = channel_rx
            .await
            .map_err(|_| TransportError::ConnectionClosed)?;

        Ok(ChannelConnection { tx })
    }

    async fn serve(&self, server: impl Server<Self> + 'static) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        {
            let mut registry = self.registry.write().await;
            registry.insert(self.local_addr.clone(), tx);
        }

        while let Some((from, request_tx)) = rx.recv().await {
            let (tx, mut rx) = mpsc::channel(100);
            let server = server.clone();
            request_tx
                .send(tx)
                .map_err(|_| TransportError::ConnectionClosed)?;

            tokio::spawn(async move {
                while let Some((response_tx, payload)) = rx.recv().await {
                    let result = server.handle(&from, payload).await;

                    if let Some(response_tx) = response_tx {
                        let response = match result {
                            Ok(Some(data)) => Ok(data),
                            Ok(None) => Ok(Vec::new()),
                            Err(e) => Err(e),
                        };

                        let _ = response_tx.send(response);
                    }
                }
            });
        }

        let mut registry = self.registry.write().await;
        registry.remove(&self.local_addr);

        Ok(())
    }
}
