use std::{fmt::Display, hash::Hash, sync::Arc};

pub mod channel;
pub mod error;
pub mod manager;
pub mod tcp;

use async_trait::async_trait;
pub use error::{Result, TransportError};
pub use manager::ThinConnectionManager;
use tokio::sync::Notify;

use crate::SelfTerminating;

pub trait Address: Hash + Eq + Clone + Display + Send + Sync {}

#[async_trait]
pub trait Connection<T: Transport>: Send + Sync + Clone {
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>>;
    async fn cast(&self, msg: Vec<u8>) -> Result<()>;
    async fn close(&self) -> Result<()>;
}

#[async_trait]
pub trait ConnectionManager<T: Transport>: Send + Sync {
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>>;
    async fn cast(&self, msg: Vec<u8>) -> Result<()>;
}

#[async_trait]
pub trait Transport: Clone + Send + Sync {
    type Address: Address;
    type Connection: Connection<Self>;

    async fn connect(&self, addr: Self::Address) -> Result<Self::Connection>;
    async fn serve(
        &self,
        server: impl Server<Self> + 'static,
        stop_signal: Arc<Notify>,
    ) -> Result<()>;
}

#[async_trait]
pub trait Server<T: Transport>: Clone + Send + Sync + SelfTerminating {
    async fn handle(&self, addr: &T::Address, msg: Vec<u8>) -> Result<Option<Vec<u8>>>;
}
