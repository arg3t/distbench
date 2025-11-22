# Architecture Documentation

This document describes the architecture of the distributed algorithms framework. It is designed to provide sufficient detail for porting the framework to another language while remaining language-agnostic.

## Overview

The framework provides infrastructure for implementing and testing distributed algorithms. It abstracts away networking, message passing, and node lifecycle management, allowing developers to focus on algorithm logic.

### Core Concepts

- **Node**: A participant in the distributed system that executes an algorithm
- **Community**: A collection of nodes that can communicate with each other
- **Algorithm**: The distributed algorithm logic that runs on each node
- **Transport**: The underlying communication mechanism (TCP, in-memory channels, etc.)
- **Message**: Data exchanged between nodes

## System Architecture

### Layered Design

```
┌─────────────────────────────────┐
│   Algorithm Implementation      │  User-defined algorithm logic
├─────────────────────────────────┤
│   Algorithm Framework Layer     │  State, handlers, lifecycle, layering
├─────────────────────────────────┤
│   Node Management Layer         │  Node lifecycle, coordination
├─────────────────────────────────┤
│   Community Layer               │  Peer discovery, status tracking
├─────────────────────────────────┤
│   Connection Management         │  Connection pooling, retries
├─────────────────────────────────┤
│   Transport Layer               │  Network abstraction
└─────────────────────────────────┘
```

### Algorithm Layering Architecture

In addition to the system layers above, algorithms themselves can be layered hierarchically:

```
┌─────────────────────────────────┐
│   Parent Algorithm              │
│   ┌─────────────────────────┐   │
│   │  Child Algorithm 1      │   │  deliver() ↑
│   └─────────────────────────┘   │
│   ┌─────────────────────────┐   │
│   │  Child Algorithm 2      │   │  method calls ↓
│   └─────────────────────────┘   │
└─────────────────────────────────┘
```

**Key Properties**:

- Parent algorithms contain child algorithms as fields
- Children deliver messages upward to parents
- Parents call methods downward on children
- Each layer has independent state and handlers
- Configuration cascades from parent to children

## Component Details

### 1. Transport Layer

The transport layer provides an abstraction over different communication mechanisms.

#### Responsibilities

- Establishing connections between nodes
- Sending and receiving raw bytes
- Serving incoming connections
- Connection lifecycle management

#### Key Abstractions

**Address**: Identifies a node's network location. Must be:

- Hashable and comparable (for use in maps/sets)
- Cloneable
- Thread-safe
- Convertible to string for display

**Connection**: Represents an active connection to another node. Provides:

- `send(bytes)`: Send message and wait for response (request-response pattern)
- `cast(bytes)`: Send message without waiting for response (fire-and-forget)
- `close()`: Terminate the connection

**Transport**: Manages the overall networking. Provides:

- `connect(address)`: Establish a connection to a peer
- `serve(server, stop_signal)`: Start accepting incoming connections

#### Implementation Notes

- Connections should be multiplexable (multiple concurrent messages)
- Transport implementations should handle reconnection logic
- Both synchronous (request-response) and asynchronous (cast) messaging must be supported

### 2. Connection Management Layer

Sits above the transport layer to manage connections efficiently.

#### Responsibilities

- Lazy connection establishment
- Connection pooling and reuse
- Automatic reconnection on failures
- Thread-safe access to connections

#### Connection Manager Interface

- `new(transport, address)`: Create a manager for a specific peer
- `send(bytes)`: Send and wait for response (establishes connection if needed)
- `cast(bytes)`: Send without response (establishes connection if needed)

#### Design Pattern

Connection managers act as per-peer proxies that:

1. Lazily establish connections on first use
2. Cache connections for reuse
3. Handle connection failures transparently
4. Provide thread-safe access

### 3. Community Layer

Manages the set of peers a node can communicate with.

#### Responsibilities

- Maintaining the set of neighbor nodes
- Mapping addresses to peer identities
- Tracking peer status (not started, starting, running, stopping, terminated)
- Providing access to connection managers
- Storing peer metadata (public keys, etc.)

#### Data Structures

- **Neighbors Set**: The subset of peers this node directly communicates with
- **Connections Map**: Maps peer IDs to their connection managers
- **Address Map**: Maps network addresses to peer IDs (for identifying incoming messages)
- **Status Map**: Tracks the current lifecycle status of each peer
- **Keys Map**: Stores cryptographic keys for peers (optional)

#### Key Operations

- `neighbours()`: Get connection managers for all neighbor peers
- `connection(peer_id)`: Get connection manager for a specific peer
- `id_of(address)`: Look up peer ID from network address
- `set_status(peer_id, status)`: Update a peer's status
- `statuses()`: Get current status of all peers

### 4. Node Management Layer

Coordinates the lifecycle of a single node in the system.

#### Node States

1. **NotStarted**: Initial state
2. **KeySharing**: Exchanging cryptographic public keys with peers
3. **Starting**: Node is synchronizing startup with peers
4. **Running**: Algorithm is executing
5. **Stopping**: Node is coordinating shutdown
6. **Terminated**: Node has stopped

#### State Transitions

```
NotStarted
    ↓
KeySharing ───→ (exchange public keys)
    ↓
Starting ─────→ (sync with peers)
    ↓
Running ─────→ (algorithm executes)
    ↓
Stopping ────→ (coordinate shutdown)
    ↓
Terminated
```

#### Responsibilities

- Managing node lifecycle state
- Synchronizing startup with other nodes
- Coordinating shutdown with other nodes
- Invoking algorithm lifecycle hooks
- Handling incoming messages
- Routing messages to algorithm handlers

#### Synchronization Protocol

**Key Exchange Phase** (KeySharing state):

1. Node generates a unique Ed25519 key pair
2. Node announces its public key to all peers
3. Node waits for public keys from all peers
4. Public keys are stored in the Community's KeyStore
5. Transition to Starting state when all keys received

**Startup Synchronization**:

1. Node enters Starting state
2. Node broadcasts "Started" message to all peers
3. Node waits for "Started" from all peers
4. When all peers have started, transition to Running
5. Call algorithm's `on_start()` hook

**Shutdown Synchronization**:

1. Algorithm calls `terminate()`
2. Node enters Stopping state
3. Node broadcasts "Finished" message to all peers
4. Node waits for "Finished" from all peers
5. When all peers have finished, transition to Terminated
6. Call algorithm's `on_exit()` hook

#### Message Handling

Nodes handle two types of messages:

1. **Node Messages**: Lifecycle coordination (Started, Finished)
2. **Algorithm Messages**: Algorithm-specific logic

### 5. Algorithm Framework Layer

Provides the programming model for implementing distributed algorithms.

#### Algorithm Trait

Algorithms must implement:

- `on_start()`: Called when all nodes are ready (after synchronization)
- `on_exit()`: Called during shutdown (for cleanup)
- `report()`: Optional method to return algorithm-specific metrics/results
- `terminate()`: Signal that this node wants to stop
- `terminated()`: Check if algorithm has terminated

#### Message Handler Trait

Algorithms handle incoming messages through two mechanisms:

1. **Direct Message Handling**:

   - `handle(src, msg_type_id, msg_bytes, path)`: Process incoming message
     - Returns `Some(response_bytes)` for request-response
     - Returns `None` for fire-and-forget messages
     - Uses `path` parameter to route to child algorithms

2. **Deliverable Algorithm Interface**:
   - `deliver(src, msg_bytes)`: Receive messages from child algorithms
     - Used for parent-child communication in layered architectures
     - Deserializes envelope format: `(message_type: String, payload: Vec<u8>)`
     - Routes to appropriate handler based on message type

#### Algorithm State

Each algorithm has:

- **Configuration Fields**: Loaded from config file at startup
- **Internal State**: Algorithm-specific data
- **Child Algorithms**: Optional child algorithms for layered architectures
- **Peer Access**: Methods to access connection managers for sending messages:
  - `peers()`: Iterator over all neighbor peers
  - `id()`: Get this node's unique identifier
  - `N()`: Get total number of nodes in the system
  - `sign(message)`: Create cryptographically signed message
  - `deliver(src, msg_bytes)`: Send message to parent algorithm (if exists)
- **Termination Signal**: Mechanism to signal completion

#### Algorithm Layering

The framework supports hierarchical algorithm composition:

**Child Algorithm Fields**:

- Marked with `#[distbench::child]` attribute
- Must be wrapped in `Arc<ChildAlgorithm>`
- Automatically initialized and configured during algorithm creation
- Have their own lifecycle tied to parent algorithm

**Parent-Child Communication**:

- Children can deliver messages to parents using `self.deliver()`
- Parents intercept child messages using `#[distbench::handlers(from = child_name)]`
- Message format: `(message_type_id: String, serialized_payload: Vec<u8>)`
- Parents can call public methods on child algorithms

**Configuration Hierarchy**:

- Parent configuration can include nested child configuration
- Each child has its own configuration namespace
- YAML anchors supported for reusable configuration templates

### 6. Serialization Layer

Provides abstraction over serialization formats.

#### Format Trait

- `serialize(value)`: Convert object to bytes
- `deserialize(bytes)`: Convert bytes to object
- `name()`: Format identifier (for logging)

#### Two-Level Serialization

**Outer Layer** (Node Messages):

- Uses a fixed binary format (rkyv)
- Handles node lifecycle messages
- Wraps algorithm messages

**Inner Layer** (Algorithm Messages):

- Pluggable format (JSON, bincode, etc.)
- User-configurable
- Algorithm-specific messages

#### Message Envelope Format

```
NodeMessage::Algorithm(type_id, payload_bytes)
```

Where:

- `type_id`: String identifying the message type (for deserialization)
- `payload_bytes`: Serialized algorithm message (using chosen format)

## Message Flow

### Sending a Message (Peer-to-Peer)

1. Algorithm calls peer method: `peer.my_message(&msg).await`
2. Framework serializes message using configured format
3. Message is wrapped in NodeMessage envelope
4. Envelope is serialized with rkyv
5. Connection manager sends bytes via transport
6. For request-response, wait for reply and deserialize

### Receiving a Message (Peer-to-Peer)

1. Transport receives bytes from network
2. Server handler is invoked with source address and bytes
3. Outer envelope (NodeMessage) is deserialized with rkyv
4. If lifecycle message (Started/Finished): update peer status
5. If algorithm message: extract type ID and payload
6. Algorithm handler is invoked with path parameter
7. If path is non-empty, route to child algorithm
8. Handler deserializes payload using configured format
9. Handler processes message and optionally returns response
10. Response is serialized and sent back

### Message Flow in Layered Algorithms

#### Child-to-Parent Message Delivery

1. Child algorithm calls `self.deliver(src, &envelope_bytes).await`
2. Child serializes message envelope: `(message_type_id, payload_bytes)`
3. Framework checks if parent exists via weak reference
4. Parent's `deliver()` method is invoked
5. Parent deserializes envelope to extract message type ID
6. Parent routes to handler marked with `#[distbench::handlers(from = child_name)]`
7. Handler deserializes payload and processes message
8. Handler optionally returns response bytes

#### Parent-to-Child Method Call

1. Parent algorithm calls public method on child: `self.child.method().await`
2. This is a direct method call (not message passing)
3. Child method executes and returns result
4. No serialization/deserialization involved

#### Message Routing with Path

When a message arrives at a parent algorithm with children:

1. Handler receives `path` parameter: `["child_name", "grandchild_name", ...]`
2. If path is empty: handle message locally
3. If path is non-empty:
   - Extract first element as child name
   - Look up child algorithm by name
   - Forward message to child's `handle()` with remaining path
   - Child recursively applies same routing logic

## Configuration System

### Configuration File Format

YAML file mapping node IDs to node definitions:

```yaml
node_id:
  neighbours: [list of neighbor node IDs]
  host: "network address"
  port: port_number
  algorithm_specific_field: value
```

**Special Behavior**:

- If `neighbours` is an empty list, the node automatically connects to all other nodes (fully connected topology)
- This simplifies configuration for algorithms that require full connectivity (e.g., broadcast, consensus)
- Keys starting with `_` (underscore) are ignored and can be used for YAML templates

### Configuration Loading

1. Parse YAML file into map of node definitions
2. Filter out template entries (keys starting with `_`)
3. Extract standard fields (neighbours, host, port)
4. Remaining fields are passed to algorithm as configuration
5. Algorithm factory deserializes config into typed configuration struct
6. For layered algorithms, child configurations are extracted and passed recursively

### Algorithm Configuration

Algorithms define configuration via attributes:

- Required fields: Must be present in config file
- Optional fields: Use default value if not specified

### Nested Configuration for Layered Algorithms

Child algorithms are configured using nested YAML structures:

```yaml
node1:
  neighbours: [node2]
  parent_field: value
  # Child algorithm configuration
  child_name:
    child_field: value
```

The framework automatically:

1. Extracts nested configuration for each child
2. Passes child config to child algorithm factory
3. Builds child algorithm with its configuration
4. Links child to parent with weak reference

### YAML Anchors and Templates

Configuration files support YAML anchors for reusable configuration:

```yaml
# Template (ignored due to _ prefix)
_template: &shared_config
  algorithm_field: "value"
  child_algorithm:
    child_field: "child_value"

node1:
  <<: *shared_config # Merge template
  neighbours: [node2]

node2:
  <<: *shared_config
  neighbours: [node1]
```

**Benefits**:

- Reduces duplication in configuration files
- Makes it easy to change shared settings
- Particularly useful for layered algorithms with complex nested configs
- Template keys (starting with `_`) are automatically filtered out

## Execution Modes

### Offline Mode

- All nodes run in same process
- Uses in-memory channels for communication
- No network overhead
- Useful for testing and debugging
- Deterministic ordering possible

### Network Mode

- Each node runs in separate process
- Uses TCP sockets for communication
- Real network conditions
- Production deployment
- True distribution

## Code Generation

The framework uses build-time code generation and procedural macros:

### Algorithm Registry

1. Scan `src/algorithms` directory
2. Find structs marked with algorithm state attribute
3. Generate macro that maps algorithm names to implementations
4. Allows dynamic algorithm selection at runtime

### Generated Components

For each algorithm, the `#[distbench::state]` macro generates:

- Configuration struct (including nested child configurations)
- Algorithm factory implementation
- Child algorithm initialization and linking code
- Helper methods (`peers()`, `id()`, `N()`, `deliver()`, etc.)
- Weak reference management for parent-child relationships

For each handler implementation, the `#[distbench::handlers]` macro generates:

- Message handler dispatch logic
- Peer proxy methods for each handler
- For `#[distbench::handlers(from = child)]`: DeliverableAlgorithm implementation

### Layering-Specific Generation

When child algorithms are detected:

1. **Child Field Initialization**: Generate code to build each child with its config
2. **Parent Reference Setup**: Generate code to set weak parent references
3. **DeliverableAlgorithm Wrapper**: Generate struct implementing DeliverableAlgorithm
4. **Message Routing**: Generate dispatch code to route messages from child to parent handlers
5. **Lifecycle Management**: Store child deliverable instances to prevent premature cleanup

## Implementation Requirements

### Language-Agnostic Requirements

1. **Async/Concurrent Runtime**: Need non-blocking I/O and task spawning
2. **Serialization**: Need at least one binary and one text format
3. **Networking**: TCP sockets with async I/O
4. **Thread Safety**: Shared state must be safely accessible
5. **Weak References**: For avoiding circular references in shutdown

### Critical Implementation Details

#### Termination Handling

- Nodes must support both self-termination and external stop signals
- Algorithm termination should trigger graceful shutdown
- Must handle case where some nodes terminate before others
- Child algorithm termination should be coordinated with parent

#### Status Tracking

- Status updates must be atomic
- Status checks must be consistent
- Race conditions during startup/shutdown must be handled

#### Connection Lifecycle

- Connections should be established lazily
- Failed connections should be retried
- Connection cleanup on shutdown

#### Message Ordering

- No strict ordering guarantees required
- Implementation may provide ordered delivery per connection
- Algorithm should not assume global message ordering

#### Algorithm Layering Implementation

**Parent-Child Lifecycle**:

- Child algorithms are built before parent algorithm initialization completes
- Parent holds strong references (Arc) to children
- Children hold weak references to parent (to avoid circular references)
- Child deliverable wrappers are stored in parent to prevent premature cleanup

**Message Delivery**:

- Children serialize messages before calling `deliver()`
- Envelope format: `(message_type_id: String, payload: Vec<u8>)`
- Parent upgrades weak reference to call `deliver()` on parent
- If parent reference is dropped, delivery fails silently

**Configuration Parsing**:

- Must support nested YAML structures
- Must filter out template keys (starting with `_`)
- Must support YAML anchors and merge keys (`<<: *anchor`)
- Child configurations are extracted from parent config by field name

**Memory Management**:

- Use weak references from child to parent to avoid cycles
- Parent stores Arc to DeliverableAlgorithm implementation for each child
- This keeps the deliverable alive as long as parent exists
- Prevents "cannot upgrade weak reference" errors during message delivery

## Extensibility Points

### Custom Transports

New transport implementations can:

- Use different protocols (UDP, QUIC, etc.)
- Implement custom connection pooling strategies
- Add encryption, compression, etc.

### Custom Serialization Formats

New formats can:

- Trade speed for size (or vice versa)
- Support schema evolution
- Add type safety or validation

### Custom Connection Managers

Can implement:

- Connection pooling strategies
- Load balancing across multiple connections
- Circuit breakers and backoff strategies
- Custom retry logic

## Testing Strategy

### Unit Testing

- Test individual components in isolation
- Mock transport layer for algorithm testing
- Test configuration parsing

### Integration Testing

- Test complete node lifecycle
- Test multi-node coordination
- Test failure scenarios

### Offline Mode Testing

- Run complete distributed algorithm in single process
- Deterministic testing with controlled message ordering
- Fast iteration without network overhead

## Performance Considerations

### Memory

- Connection managers cache connections
- Status maps grow with number of peers
- Message buffers are temporary

### CPU

- Serialization/deserialization overhead
- Message routing and dispatch
- State synchronization

### Network

- Startup synchronization requires all-to-all communication
- Shutdown synchronization requires all-to-all communication
- Algorithm messages depend on algorithm design

## Security Considerations

### Cryptographic Signing

The framework provides built-in support for cryptographically signed messages:

#### Key Management

- Each node generates a unique key pair on startup
- Public keys are automatically exchanged during the startup synchronization phase
- Keys are stored in a KeyStore within the Community
- Uses Ed25519 signatures via the `ed25519-dalek` library

#### Signed Messages

- Messages can be wrapped in `Signed<M>` type
- Signed messages include:
  - The original message content
  - A cryptographic signature
  - The signer's peer ID
- Signatures are created using `algorithm.sign(message)`
- Signatures can be verified against the KeyStore
- Useful for Byzantine fault-tolerant algorithms

#### Trust Model

- Nodes exchange public keys during startup
- Each node trusts the authenticity of exchanged keys
- Signed messages prove message origin and integrity
- No certificate authority or PKI required
- No built-in message encryption (signatures only provide authentication, not confidentiality)

### Attack Vectors

- Malicious messages (mitigated by type-safe deserialization and optional signing)
- Resource exhaustion (no rate limiting built-in)
- Network partitions (no automatic handling)
- Man-in-the-middle during key exchange (no authentication of initial key exchange)

## Future Extensions

Possible enhancements:

- Failure detection and recovery
- Dynamic topology changes
- Message encryption
- Rate limiting and backpressure
- Metrics and observability
- Checkpointing and recovery
