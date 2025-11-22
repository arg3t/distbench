//! Middleware transport that adds artificial latency.
//!
//! This module provides a transport wrapper that injects a fixed delay
//! before every message send operation.

use async_trait::async_trait;
use log::trace;
use rand::Rng;
use std::time::Duration;
use std::{ops::Range, sync::Arc};
use tokio::sync::Notify;
use tokio::time::sleep;

use super::{Connection, Result, Server, Transport};

/// A transport that adds latency to all outgoing messages.
///
/// Wraps an inner transport and adds a delay to every connection created.
#[derive(Clone)]
pub struct DelayedTransport<T: Transport> {
    inner: Arc<T>,
    delay: Range<u64>,
}

impl<T: Transport> DelayedTransport<T> {
    /// Creates a new delayed transport.
    ///
    /// # Arguments
    ///
    /// * `inner` - The underlying transport to wrap
    /// * `delay` - The latency to add to each message send
    pub fn new(inner: Arc<T>, delay: Range<u64>) -> Arc<Self> {
        Arc::new(Self { inner, delay })
    }
}

#[async_trait]
impl<T: Transport + 'static> Transport for DelayedTransport<T> {
    type Address = T::Address;
    type Connection = DelayedConnection<T::Connection>;

    async fn connect(&self, addr: Self::Address) -> Result<Self::Connection> {
        let inner_conn = self.inner.connect(addr).await?;
        Ok(DelayedConnection {
            inner: inner_conn,
            delay: self.delay.clone(),
        })
    }

    async fn serve(
        &self,
        server: impl Server<Self> + 'static,
        stop_signal: Arc<Notify>,
    ) -> Result<()> {
        // We need to wrap the server so that it handles messages for the inner transport type.
        // However, since Server<DelayedTransport<T>> and Server<T> process the same messages
        // (same Address type, same Vec<u8> payload), we can just adapt the type.
        //
        // But wait: Server trait is `impl Server<Self>`.
        // `Self` here is `DelayedTransport<T>`.
        // The inner transport expects `Server<T>`.
        //
        // We need a wrapper struct that implements Server<T> and delegates to Server<DelayedTransport<T>>.
        let wrapped_server = ServerWrapper {
            inner_server: server,
            _phantom: std::marker::PhantomData,
        };

        self.inner.serve(wrapped_server, stop_signal).await
    }
}

/// Wrapper for the server to adapt types between DelayedTransport and T.
#[derive(Clone)]
struct ServerWrapper<S, T> {
    inner_server: S,
    _phantom: std::marker::PhantomData<T>,
}

#[async_trait]
impl<T, S> Server<T> for ServerWrapper<S, T>
where
    T: Transport + 'static,
    S: Server<DelayedTransport<T>> + 'static,
{
    async fn handle(&self, addr: &T::Address, msg: Vec<u8>) -> Result<Option<Vec<u8>>> {
        // Since T::Address is the same as DelayedTransport<T>::Address,
        // we can just pass it through.
        self.inner_server.handle(addr, msg).await
    }
}

#[async_trait]
impl<T, S> crate::algorithm::SelfTerminating for ServerWrapper<S, T>
where
    T: Transport + 'static,
    S: Server<DelayedTransport<T>> + Send + Sync + 'static,
{
    async fn terminate(&self) {
        self.inner_server.terminate().await
    }

    async fn terminated(&self) -> bool {
        self.inner_server.terminated().await
    }
}

/// A connection that adds latency to outgoing messages.
#[derive(Clone)]
pub struct DelayedConnection<C> {
    inner: C,
    delay: Range<u64>,
}

#[async_trait]
impl<T, C> Connection<DelayedTransport<T>> for DelayedConnection<C>
where
    T: Transport + 'static,
    C: Connection<T>,
{
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>> {
        let delay_ms = if self.delay.is_empty() {
            self.delay.start
        } else {
            rand::thread_rng().gen_range(self.delay.clone())
        };

        if delay_ms > 0 {
            trace!("Delaying send by {:?}", self.delay);
            sleep(Duration::from_millis(delay_ms)).await;
        }
        self.inner.send(msg).await
    }

    async fn cast(&self, msg: Vec<u8>) -> Result<()> {
        let delay_ms = if self.delay.is_empty() {
            self.delay.start
        } else {
            rand::thread_rng().gen_range(self.delay.clone())
        };

        if delay_ms > 0 {
            trace!("Delaying cast by {:?}", self.delay);
            sleep(Duration::from_millis(delay_ms)).await;
        }
        self.inner.cast(msg).await
    }

    async fn close(&self) -> Result<()> {
        self.inner.close().await
    }
}
