use std::{fmt::Display, hash::Hash};

pub mod error;
pub mod tcp;

pub use error::{Result, TransportError};

pub trait Address: Hash + Eq + Clone + Display {}

pub trait Transport: Clone {
    type Address: Address;
    type Connection: Connection<Self>;

    async fn connect(&self, addr: Self::Address) -> Result<Self::Connection>;
    async fn serve(&self, server: impl Server<Self>) -> Result<()>;
}

pub trait Server<T: Transport>: Clone {
    async fn handle(&self, addr: T::Address, msg: Vec<u8>) -> Result<Vec<u8>>;
}

pub trait Connection<T: Transport>: Clone {
    async fn send(&self, msg: Vec<u8>) -> Result<Vec<u8>>;
    async fn cast(&self, msg: Vec<u8>) -> Result<()>;
    async fn close(&self) -> Result<()>;
}
