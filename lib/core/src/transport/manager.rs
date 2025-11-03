use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{Connection, ConnectionManager, Result, Transport};

pub struct ThinConnectionManager<T: Transport> {
    transport: Arc<T>,
    address: T::Address,
    connection: Arc<RwLock<Option<T::Connection>>>,
}

impl<T: Transport> ThinConnectionManager<T> {
    pub fn new(transport: Arc<T>, address: T::Address) -> Self {
        Self {
            transport,
            address,
            connection: Arc::new(RwLock::new(None)),
        }
    }

    async fn ensure_connected(&self) -> Result<T::Connection> {
        {
            let conn_guard = self.connection.read().await;
            if let Some(conn) = conn_guard.as_ref() {
                return Ok(conn.clone());
            }
        }

        let mut conn_guard = self.connection.write().await;

        if let Some(conn) = conn_guard.as_ref() {
            return Ok(conn.clone());
        }

        let conn = self.transport.connect(self.address.clone()).await?;
        *conn_guard = Some(conn.clone());

        Ok(conn)
    }
}

impl<T: Transport> Clone for ThinConnectionManager<T> {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            address: self.address.clone(),
            connection: self.connection.clone(),
        }
    }
}

#[async_trait]
impl<T: Transport> ConnectionManager<T> for ThinConnectionManager<T> {
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>> {
        let connection = self.ensure_connected().await?;
        connection.send(msg).await
    }

    async fn cast(&self, msg: Vec<u8>) -> Result<()> {
        let connection = self.ensure_connected().await?;
        connection.cast(msg).await
    }
}
