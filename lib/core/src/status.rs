//! Node status tracking.
//!
//! This module defines the different states a node can be in during its lifecycle.

/// The current status of a node in the distributed system.
///
/// Nodes progress through these states during their lifecycle:
/// `NotStarted` → `Starting` → `Running` → `Stopping` → `Terminated`
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub enum NodeStatus {
    /// The node has not yet started.
    #[default]
    NotStarted,

    /// The node is in the process of starting up and synchronizing with peers.
    Starting,

    /// The node is in the process of shutting down.
    Stopping,

    /// The node has completely terminated.
    Terminated,

    /// The node is running and actively executing the algorithm.
    Running,
}
