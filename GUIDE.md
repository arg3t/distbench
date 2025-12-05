# Distbench Guide

## Running the framework

### Commands to Run

To execute an algorithm:

```bash
cargo run -- \
  --config configs/your_config.yaml \
  --algorithm YourAlgorithm \
  --mode [offline|local|network] \
  [--id NODE_ID (only for network mode)] \
  [-s | --port-base PORT (base port for local, or port for network mode, default: 8000)] \
  [-f | --format [Json|Bincode] (serialization format, default: Bincode)] \
  [-t | --timeout SECONDS (timeout for the algorithm in seconds, default: 10)] \
  [-v | -vv | -vvv (logging verbosity)] \
  [-r | --report-folder PATH (path to save reports)] \
  [-l | --latency RANGE (latency range in ms, e.g. "100-200", default: "0-0")] \
  [-d | --startup-delay MS (startup delay in ms, default: 200)]
```

**Examples:**

- **Offline Mode:**
  ```bash
  cargo run -- --config configs/pingpong.yaml --algorithm PingPong --mode offline --timeout 30
  ```
- **Local Mode:**
  ```bash
  cargo run -- --config configs/your_config.yaml --algorithm YourAlgorithm --mode local -p 4242 --nodes 4 --timeout 10
  ```
- **Network Mode (Terminal 1 for node1):**
  ```bash
  cargo run -- --config configs/your_config.yaml --id node1 --algorithm YourAlgorithm --mode network
  ```

### Final Reporting

Reports are logged on exit and can also be written to the directory specified by the `--report-folder` flag. Each node's report is saved as `node_id.json`, with each log entry on a new line.

**Example JSON Log Entry:**

```json
{"elapsed_time":1,"messages_received":{"Echo":1},"bytes_received":43,"algorithm":"Echo","details":{<algorithm_specific>}}
```

The `"messages_received"` field is a map indicating the number of messages received on each layer, where each key represents a path.

## API

### Defining Messages

Use `#[distbench::message]` to define data structures exchanged between nodes.

```rust
#[distbench::message]
struct Ping {
    sequence: u32,
}
```

### Algorithm State

Use `#[distbench::state]` for per-node state and configuration. Non-config fields must implement `Send + Default`.

```rust
#[distbench::state]
pub struct PingPong {
    #[distbench::config] // Required field from config
    initiator: bool,

    #[distbench::config(default = 5)] // Optional field with default
    max_rounds: u32,

    pings_received: AtomicU64, // Internal state (Send + Default)
}
```

### Lifecycle

Implement the `distbench::Algorithm` trait for lifecycle hooks:

```rust
use async_trait::async_trait;
use distbench::Algorithm;

#[async_trait]
impl Algorithm for PingPong {
    async fn on_start(&self) { /* Called when all nodes are ready */ }
    async fn on_exit(&self) { /* Called during shutdown for cleanup */ }
    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> { /* Optional: return metrics */ }
}
```

### Handlers

Use `#[distbench::handlers]` to define functions that process incoming messages.
The macro generates peer proxy methods (`peer.message(&msg)`).

```rust
#[distbench::handlers]
impl PingPong {
    // Fire-and-forget pattern
    async fn ping(&self, src: PeerId, msg: &Ping) { /* ... */ }

    // Request-response pattern
    async fn pong(&self, src: PeerId, msg: &Pong) -> Option<String> { /* ... */ }
}
```

### Algorithm Layering (Child Algorithms and Interfaces)

Compose complex protocols from simpler building blocks.

- **Parent defines child:** Use `#[distbench::child]` in parent's state struct:
  ```rust
  #[distbench::state]
  pub struct UpperLayer {
      #[distbench::child]
      broadcast: LowerLayer,
  }
  ```
- **Child defines interface:** Use `#[distbench::interface]` for methods callable by parent. These methods receive serialized bytes (`Vec<u8>`). Child delivers raw bytes to parent via `self.deliver(self.id().clone(), &msg).await?`.
  ```rust
  #[distbench::interface]
  impl LowerLayer {
      pub async fn broadcast(&self, msg: Vec<u8>, extra_param: bool) -> Result<(), FormatError> { /* ... */ }
  }
  ```
- **Parent calls child interface:** Parent passes message structs directly; framework handles serialization.
  ```rust
  self.broadcast.broadcast(&send_msg).await?;
  ```
- **Parent intercepts child messages:** Use `#[distbench::handlers(from = child_name)]`. Framework deserializes bytes to parent's typed handlers.
  ```rust
  #[distbench::handlers(from = broadcast)]
  impl UpperLayer {
      async fn broadcast_send(&self, src: PeerId, msg: &BroadcastSend) { /* ... */ }
  }
  ```

### Available Utility Functions (on `self`)

- `self.id() -> &PeerId`: Returns the unique identifier of the current node.
- `self.N() -> u32`: Returns the total number of nodes in the network.
- `self.nodes() -> NodeSet`: Returns a set of all the node ids in the network.
- `self.peers() -> impl Iterator<Item = (&PeerId, &Peer)>`: Iterates over all neighboring peers.
- `self.terminate() -> impl Future<Output = ()>`: Signals that this node has completed execution. All nodes must call this for clean shutdown.
- `self.sign<M: Digest>(value: M) -> Signed<M>`: Creates a cryptographically signed version of a message.
- `self.deliver(src: PeerId, msg: &[u8]) -> Result<Option<Vec<u8>, Err>`: Deliver a message to the upper layer for processing

## Signed Messages

Use `distbench::signing::Signed<M>`. The framework **automatically verifies the signature** of all incoming `Signed<M>` messages before your handler is called. If the signature is invalid, the message is dropped and an error is logged.

```rust
use distbench::signing::Signed;

#[distbench::message]
struct Vote { round: u32, value: String, }

let signed_vote = self.sign(Vote { round: 1, value: "commit".to_string() });

async fn handle_vote(&self, src: PeerId, msg: &Signed<Vote>) {
    info!("Vote for round {}: {}", msg.round, msg.value); // Access inner value via Deref
    info!("Received: {}", msg); // Display shows signature
}
```

## Configuration

### File Format

YAML configuration maps node IDs to node definitions:

```yaml
node_id:
  neighbours: [list of neighbor node IDs]
  host: "network address" # For network mode
  port: port_number # For network mode
  algorithm_field: value # Passed to algorithm
```

### Fully Connected Topology

Use an empty `neighbours` list to connect to all other nodes, or just omit the field:

```yaml
node1:
  neighbours: [] # Automatically connects to node2, node3, node4
  is_sender: true
```

### Using YAML Anchors for Reusability

```yaml
# Shared template (starts with _ to indicate it's not a node)
_template: &config_template
  messages: []

node1:
  <<: *config_template # Merge template
  neighbours: [node2, node3]
  start_node: true
  messages: ["Hello", "World"] # Override template value
```

### Nested Configuration for Child Algorithms

Child algorithms are configured using nested structures in the YAML configuration file, mirroring the `#[distbench::child]` declaration in the parent's state.

```yaml
node1:
  neighbours: [node2, node3]
  start_node: true
  messages: ["Hello", "World"]
  # Child algorithm configuration
  broadcast:
    max_retries: 3

node2:
  neighbours: [node1, node3]
  start_node: false
  messages: []
  broadcast:
    max_retries: 3
```

## Docker Deployment

Distbench can run in Docker containers with each node in a separate container.

### Quick Start

1.  **Generate Docker Compose File:**
    ```bash
    python3 dockerize.py \
      -c configs/echo.yaml \
      -a Echo \
      -o docker-compose.yaml
    ```
2.  **Run the System:**
    ```bash
    docker-compose up --build         # Build and start all containers
    docker-compose up --build -d      # Run in detached mode
    docker-compose logs -f            # View logs
    docker-compose down               # Stop and clean up
    ```
