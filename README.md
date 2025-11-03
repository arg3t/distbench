# Distbench - Distributed Algorithms Framework

A Rust framework for implementing and testing distributed algorithms. Distbench handles the infrastructure concerns (networking, message passing, node lifecycle) so you can focus on algorithm logic.

## Features

- Clean separation between algorithm logic and infrastructure
- Multiple transport backends (in-memory channels, TCP)
- Pluggable serialization formats (JSON, Bincode)
- Automatic node synchronization and lifecycle management
- Procedural macros for reducing boilerplate
- Built-in support for both offline (single-process) and network (multi-process) modes

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Cargo

### Installation

Clone the repository:

```bash
git clone <repository-url>
cd distbench
```

Build the project:

```bash
cargo build --release
```

### Running an Example

Run the Echo algorithm in offline mode:

```bash
cargo run --release -- \
  --config configs/test_config.yaml \
  --algorithm Echo \
  --mode offline \
  --timeout 10
```

This will:
1. Load the configuration from `configs/test_config.yaml`
2. Spawn all nodes in the same process
3. Execute the Echo algorithm
4. Terminate after 10 seconds

## Project Structure

```
cs4545-da/
├── src/
│   ├── algorithms/      # Algorithm implementations
│   │   ├── echo.rs      # Simple echo algorithm
│   │   └── ring_election.rs  # Ring election algorithm
│   ├── config.rs        # Configuration file parsing
│   ├── main.rs          # Entry point and CLI
│   └── lib.rs           # Library exports
├── lib/
│   ├── core/            # Framework core library
│   │   ├── algorithm.rs     # Algorithm traits
│   │   ├── node.rs          # Node implementation
│   │   ├── community.rs     # Peer management
│   │   ├── transport/       # Transport abstraction
│   │   └── encoding/        # Serialization formats
│   └── procs/           # Procedural macros
├── configs/             # Example configurations
└── ARCHITECTURE.md      # Detailed architecture documentation
```

## Implementing an Algorithm

### Step 1: Define Messages

Use the `#[framework::message]` macro to define message types:

```rust
use framework::message;

#[framework::message]
struct Ping {
    sequence: u32,
}

#[framework::message]
struct Pong {
    sequence: u32,
}
```

This automatically derives serialization and other necessary traits.

### Step 2: Define Algorithm State

Use the `#[framework::state]` macro to define your algorithm's state:

```rust
use framework::state;

#[framework::state]
pub struct PingPong {
    // Configuration fields (loaded from config file)
    #[framework::config]
    initiator: bool,

    #[framework::config(default = 5)]
    max_rounds: u32,

    // Internal state fields are added automatically:
    // - peers: HashMap<PeerId, Peer>
    // - termination_signal: Arc<Notify>
}
```

The `#[framework::config]` attribute marks fields that should be loaded from the configuration file:
- Without `default`: field is required in config
- With `default = value`: field is optional, uses default if not present

### Step 3: Implement Algorithm Lifecycle

Implement the `Algorithm` trait to define lifecycle hooks:

```rust
use async_trait::async_trait;
use framework::Algorithm;

#[async_trait]
impl Algorithm for PingPong {
    async fn on_start(&self) {
        // Called when all nodes are ready
        if self.initiator {
            // Get first peer and send initial ping
            if let Some((_, peer)) = self.peers.iter().next() {
                peer.ping(&Ping { sequence: 0 }).await.ok();
            }
        }
    }

    async fn on_exit(&self) {
        // Called during shutdown for cleanup
        println!("PingPong algorithm finished");
    }
}
```

### Step 4: Implement Message Handlers

Use the `#[framework::handlers]` macro to define how your algorithm handles messages:

```rust
use framework::handlers;
use framework::community::PeerId;

#[framework::handlers]
impl PingPong {
    // Handler for Ping messages (no return = fire-and-forget)
    async fn ping(&self, src: PeerId, msg: &Ping) {
        println!("Received ping {} from {}", msg.sequence, src.to_string());

        // Send pong back to sender
        if let Some(peer) = self.peers.get(&src) {
            peer.pong(&Pong { sequence: msg.sequence }).await.ok();
        }
    }

    // Handler for Pong messages (returns response = request-response)
    async fn pong(&self, src: PeerId, msg: &Pong) -> Option<String> {
        println!("Received pong {} from {}", msg.sequence, src.to_string());

        if msg.sequence >= self.max_rounds {
            // Terminate after max rounds
            self.terminate().await;
            None
        } else {
            // Send next ping
            if let Some(peer) = self.peers.get(&src) {
                peer.ping(&Ping { sequence: msg.sequence + 1 }).await.ok();
            }
            Some("acknowledged".to_string())
        }
    }
}
```

The macro automatically:
- Generates peer proxy methods (e.g., `peer.ping(msg)`)
- Handles message serialization/deserialization
- Routes incoming messages to the correct handler

### Step 5: Add to Algorithms Directory

Save your algorithm to `src/algorithms/pingpong.rs`. The build system will automatically:
- Discover the algorithm
- Generate registration code
- Make it available via the `--algorithm` flag

### Step 6: Create a Configuration File

Create a YAML configuration file (e.g., `configs/pingpong.yaml`):

```yaml
node1:
  neighbours:
    - node2
  host: "127.0.0.1"
  port: 8001
  initiator: true
  max_rounds: 10

node2:
  neighbours:
    - node1
  host: "127.0.0.1"
  port: 8002
  initiator: false
  max_rounds: 10
```

### Step 7: Run Your Algorithm

```bash
cargo run --release -- \
  --config configs/pingpong.yaml \
  --algorithm PingPong \
  --mode offline \
  --timeout 30
```

## Configuration File Format

Configuration files are in YAML format:

```yaml
<node_id>:
  neighbours: [<list of neighbor node IDs>]
  host: "<IP address>"
  port: <port number>
  <algorithm_config_field>: <value>
  ...
```

- `neighbours`: List of node IDs this node can communicate with
- `host` and `port`: Network address for this node (used in network mode)
- Additional fields are passed to the algorithm as configuration

## Command-Line Options

```
Options:
  -c, --config <FILE>       Path to the YAML configuration file
      --id <ID>             ID of this node (required for network mode)
  -a, --algorithm <NAME>    Algorithm to run
  -m, --mode <MODE>         Operating mode: offline or network [default: offline]
      --format <FORMAT>     Serialization format: json or bincode [default: json]
      --timeout <SECONDS>   Algorithm timeout in seconds [default: 10]
  -v, --verbose             Verbosity level (-v, -vv, -vvv)
```

### Execution Modes

**Offline Mode** (default):
- Runs all nodes in a single process
- Uses in-memory channels for communication
- Fast and deterministic
- Ideal for testing and development

**Network Mode**:
- Each node runs in a separate process
- Uses TCP sockets for communication
- Requires `--id` to specify which node to run
- Simulates real distributed environment

## Example Algorithms

### Echo Algorithm

A simple request-response example located in `src/algorithms/echo.rs`:

```rust
#[framework::message]
struct Message {
    sender: String,
    message: String,
}

#[framework::state]
pub struct Echo {
    #[framework::config(default = false)]
    start_node: bool,
}

#[async_trait]
impl Algorithm for Echo {
    async fn on_start(&self) {
        if self.start_node {
            let peer = self.peers.values().next().unwrap();
            match peer.message(&Message {
                sender: "Test".to_string(),
                message: "Hello, world!".to_string(),
            }).await {
                Ok(Some(response)) => info!("Message echoed: {}", response),
                Ok(None) => error!("Message not echoed"),
                Err(e) => error!("Error: {}", e),
            }
        }
        self.terminate().await;
    }
}

#[framework::handlers]
impl Echo {
    async fn message(&self, src: PeerId, msg: &Message) -> Option<String> {
        info!("Received: {}", msg.message);
        Some(msg.message.clone())
    }
}
```

### Ring Election Algorithm

A Chang-Roberts ring election algorithm in `src/algorithms/ring_election.rs`:

- Each node has a numeric ID
- Nodes forward the highest ID seen
- When a node sees its own ID, it's elected
- Election winner broadcasts termination

Run it with:

```bash
cargo run --release -- \
  --config configs/ring_config.yaml \
  --algorithm RingElection \
  --mode offline \
  --timeout 15
```

## Advanced Features

### Custom Serialization Formats

Choose between JSON (human-readable) and Bincode (compact):

```bash
# Use JSON (default)
cargo run -- --config config.yaml --algorithm Echo --format json

# Use Bincode for better performance
cargo run -- --config config.yaml --algorithm Echo --format bincode
```

### Logging

Control verbosity with `-v` flags:

```bash
# Basic logging
cargo run -- -v --config config.yaml --algorithm Echo

# Debug logging
cargo run -- -vv --config config.yaml --algorithm Echo

# Trace logging
cargo run -- -vvv --config config.yaml --algorithm Echo
```

### Accessing Peers

In your algorithm, use the `peers` field to access neighbors:

```rust
// Iterate over all peers
for (peer_id, peer) in self.peers.iter() {
    peer.some_message(&msg).await?;
}

// Get a specific peer
if let Some(peer) = self.peers.get(&peer_id) {
    peer.some_message(&msg).await?;
}

// Get first peer
if let Some((id, peer)) = self.peers.iter().next() {
    peer.some_message(&msg).await?;
}
```

### Terminating the Algorithm

Call `self.terminate().await` when your algorithm is done:

```rust
async fn on_start(&self) {
    // Do work...

    // Signal completion
    self.terminate().await;
}
```

This triggers graceful shutdown coordination with other nodes.

## Testing

Run the test suite:

```bash
cargo test
```

Run a specific test:

```bash
cargo test test_name
```

## Architecture

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

Key architectural concepts:
- **Transport Layer**: Abstraction over networking (TCP, channels)
- **Community**: Manages peer relationships and status
- **Node**: Coordinates lifecycle and message routing
- **Algorithm**: User-defined distributed logic

## Contributing

When implementing new algorithms:

1. Create a new file in `src/algorithms/`
2. Use the framework macros (`#[framework::message]`, `#[framework::state]`, `#[framework::handlers]`)
3. Implement the `Algorithm` trait
4. Add a configuration file in `configs/`
5. The build system will automatically register your algorithm

## License

[Specify your license here]

## Troubleshooting

### Algorithm not found

If you get "Unknown algorithm type" error:
- Ensure your algorithm struct has `#[framework::state]` attribute
- Verify the file is in `src/algorithms/` directory
- Run `cargo clean` and rebuild

### Nodes not synchronizing

If nodes hang during startup:
- Check that all node IDs in config file are unique
- Verify neighbor lists are correct and bidirectional
- Ensure network addresses don't conflict (in network mode)

### Message handler not called

If your message handler isn't being invoked:
- Verify you have `#[framework::handlers]` on the impl block
- Check that message type matches exactly (case-sensitive)
- Ensure the handler signature matches the pattern: `async fn name(&self, src: PeerId, msg: &MsgType)`
