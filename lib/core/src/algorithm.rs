//! Algorithm traits and interfaces.
//!
//! This module defines the core traits that distributed algorithms must implement
//! to work with the framework.

use async_trait::async_trait;
use std::sync::Arc;

use crate::community::{Community, PeerId};
use crate::error::ConfigError;
use crate::transport::{ConnectionManager, Transport};

/// Trait for handling incoming messages in a distributed algorithm.
///
/// Implementors of this trait define how their algorithm processes messages
/// received from other nodes.
#[async_trait]
pub trait AlgorithmHandler {
    /// Handles an incoming message from a peer.
    ///
    /// # Arguments
    ///
    /// * `src` - The ID of the peer that sent the message
    /// * `msg_type_id` - The type identifier for the message (used for deserialization)
    /// * `msg_bytes` - The serialized message payload
    ///
    /// # Returns
    ///
    /// * `Ok(Some(response))` - A response message to send back to the sender
    /// * `Ok(None)` - No response needed (for cast messages)
    /// * `Err(e)` - An error occurred while processing the message
    async fn handle(
        &self,
        src: PeerId,
        msg_type_id: String,
        msg_bytes: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait for algorithms that can self-terminate.
///
/// Algorithms implement this trait to signal when they have completed their
/// execution and should shut down.
#[async_trait]
pub trait SelfTerminating {
    /// Initiates termination of the algorithm.
    async fn terminate(&self);

    /// Checks if the algorithm has terminated.
    ///
    /// # Returns
    ///
    /// `true` if the algorithm has terminated, `false` otherwise.
    async fn terminated(&self) -> bool;
}

/// The main trait that distributed algorithms must implement.
///
/// This trait combines message handling and self-termination capabilities,
/// and provides lifecycle hooks for algorithm initialization and cleanup.
///
/// # Type Parameters
///
/// * `T` - The transport layer implementation
///
/// # Examples
///
/// ```ignore
/// use framework::algorithm::{Algorithm, AlgorithmHandler, SelfTerminating};
/// use framework::transport::Transport;
///
/// struct MyAlgorithm<T: Transport> {
///     // ... fields
/// }
///
/// #[async_trait]
/// impl<T: Transport> Algorithm<T> for MyAlgorithm<T> {
///     async fn on_start(&self) {
///         // Initialize algorithm state
///     }
///
///     async fn on_exit(&self) {
///         // Clean up resources
///     }
/// }
/// ```
#[async_trait]
pub trait Algorithm: AlgorithmHandler + Send + Sync + SelfTerminating {
    /// Called when the algorithm starts running.
    ///
    /// This is invoked after all nodes in the community have synchronized
    /// and are ready to begin execution.
    async fn on_start(&self) {}

    /// Called when the algorithm is exiting.
    ///
    /// This is invoked after the algorithm has terminated and the node
    /// is shutting down. Use this for cleanup operations.
    async fn on_exit(&self) {}
}

/// Factory trait for creating algorithm instances.
///
/// Implementors of this trait know how to construct algorithm instances
/// from configuration and a community context.
///
/// # Type Parameters
///
/// * `T` - The transport layer implementation
pub trait AlgorithmFactory<T: Transport, CM: ConnectionManager<T>> {
    /// The type of algorithm this factory produces.
    type Algorithm: Algorithm;

    /// Builds an algorithm instance from this configuration.
    ///
    /// # Arguments
    ///
    /// * `community` - The community context in which the algorithm will run
    ///
    /// # Returns
    ///
    /// An `Arc` wrapping the algorithm instance.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if the algorithm cannot be constructed
    /// (e.g., missing required configuration fields).
    fn build(self, community: &Community<T, CM>) -> Result<Arc<Self::Algorithm>, ConfigError>;
}
