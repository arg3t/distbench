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

Use the `#[distbench::message]` macro to define message types:

```rust
use distbench::message;

#[distbench::message]
struct Ping {
    sequence: u32,
}

#[distbench::message]
struct Pong {
    sequence: u32,
}
```

This automatically derives serialization and other necessary traits.

### Step 2: Define Algorithm State

Use the `#[distbench::state]` macro to define your algorithm's state:

```rust
use distbench::state;

#[distbench::state]
pub struct PingPong {
    // Configuration fields (loaded from config file)
    #[distbench::config]
    initiator: bool,

    #[distbench::config(default = 5)]
    max_rounds: u32,

    // Internal state variables for the algorithm
    pings: AtomicU64
}
```

The `#[distbench::config]` attribute marks fields that should be loaded from the configuration file:

- Without `default`: field is required in config
- With `default = value`: field is optional, uses default if not present

> [!IMPORTANT]  
> The other fields in the internal state of the algorithm must all implement the `Send` and `Default` traits.

### Step 3: Implement Algorithm Lifecycle

Implement the `Algorithm` trait to define lifecycle hooks:

```rust
use async_trait::async_trait;
use distbench::Algorithm;

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

Use the `#[distbench::handlers]` macro to define how your algorithm handles messages:

```rust
use distbench::handlers;
use distbench::community::PeerId;

#[distbench::handlers]
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
#[distbench::message]
struct Message {
    sender: String,
    message: String,
}

#[distbench::state]
pub struct Echo {
    #[distbench::config(default = false)]
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

#[distbench::handlers]
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

## Algorithm API Reference

This section documents the methods and features available to algorithm implementations.

### Methods Available in `#[distbench::state]` Structs

When you define an algorithm using `#[distbench::state]`, your struct automatically gets access to several useful methods:

#### `self.id() -> &PeerId`

Returns the unique identifier of the current node.

```rust
async fn on_start(&self) {
    info!("Node {} starting", self.id());
}
```

#### `self.N() -> u32`

Returns the total number of nodes in the system (including this node).

```rust
async fn on_start(&self) {
    let total_nodes = self.N();
    info!("Running in a system of {} nodes", total_nodes);

    // Useful for threshold calculations
    let majority = (total_nodes / 2) + 1;
    let byzantine_threshold = (total_nodes * 2 + 2) / 3;
}
```

#### `self.peers() -> impl Iterator<Item = (&PeerId, &Peer)>`

Returns an iterator over all neighboring peers.

```rust
async fn on_start(&self) {
    // Iterate over all peers
    for (peer_id, peer) in self.peers() {
        info!("Neighbor: {}", peer_id);
        peer.some_message(&msg).await?;
    }

    // Count peers
    let peer_count = self.peers().count();

    // Get first peer
    if let Some((id, peer)) = self.peers().next() {
        peer.ping(&Ping { seq: 0 }).await?;
    }
}
```

#### `self.terminate() -> impl Future<Output = ()>`

Signals that this node has completed its algorithm execution and is ready to terminate.

```rust
async fn handler(&self, src: PeerId, msg: &Done) {
    info!("Received done message, terminating");
    self.terminate().await;
}
```

**Important**: This only signals readiness to terminate. The node will continue running until:

- The timeout expires, OR
- All nodes have called `terminate()`

#### `self.sign<M: Digest>(value: M) -> Signed<M>`

Creates a cryptographically signed version of a message. The signature proves that this specific node created the message.

```rust
async fn on_start(&self) {
    // Create a signed message
    let signed_msg = self.sign(VoteMessage {
        round: 1,
        value: "commit".to_string(),
    });

    // Send the signed message
    for (_, peer) in self.peers() {
        peer.vote(&signed_msg).await?;
    }
}
```

### Signed Messages

The `Signed<M>` type wraps a message with cryptographic authentication. This is essential for Byzantine fault-tolerant algorithms where nodes need to prove message authenticity.

#### Creating Signed Messages

Use `self.sign()` to create signed messages:

```rust
// Define a message type
#[distbench::message]
struct Proposal {
    round: u32,
    value: String,
}

// Sign it
let signed_proposal = self.sign(Proposal {
    round: 1,
    value: "commit".to_string(),
});
```

#### Accessing Signed Message Content

`Signed<M>` implements `Deref`, so you can access the inner message directly:

```rust
async fn handle_proposal(&self, src: PeerId, msg: &Signed<Proposal>) {
    // Access fields directly via Deref
    info!("Received proposal for round {}: {}", msg.round, msg.value);

    // Or explicitly use inner()
    info!("Value: {}", msg.inner().value);
}
```

#### Display Implementation

`Signed<M>` has a special `Display` implementation that shows both the message and the signer:

```rust
async fn handle_vote(&self, src: PeerId, msg: &Signed<Vote>) {
    // When Display is implemented for Vote, this will print:
    // "Vote for value X (signed by node1)"
    info!("Received: {}", msg);
}
```

#### Signed Messages in Collections

You can store signed messages in vectors, sets, and maps:

```rust
#[distbench::state]
pub struct ByzantineConsensus {
    // Vector of signed proposals
    proposals: Mutex<Vec<Signed<Proposal>>>,

    // Map from value to set of signed votes
    votes: Mutex<HashMap<String, Vec<Signed<Vote>>>>,
}

#[distbench::handlers]
impl ByzantineConsensus {
    async fn receive_proposal(&self, src: PeerId, msg: &Signed<Proposal>) {
        // Store the signed proposal
        let mut proposals = self.proposals.lock().unwrap();
        proposals.push(msg.clone());

        // Log all proposals with their signers
        for (i, signed_prop) in proposals.iter().enumerate() {
            info!("[{}] Proposal {}: {}", self.id(), i, signed_prop);
        }
    }

    async fn receive_vote(&self, src: PeerId, msg: &Signed<Vote>) {
        // Collect votes by value
        let mut votes = self.votes.lock().unwrap();
        votes.entry(msg.value.clone())
            .or_insert_with(Vec::new)
            .push(msg.clone());

        // Check if we have enough signed votes
        let vote_count = votes[&msg.value].len();
        let threshold = (self.N() as usize * 2) / 3;
        if vote_count >= threshold {
            info!("Received {} signed votes for value {}", vote_count, msg.value);
        }
    }
}
```

#### Complete Example: Message Chain with Signatures

This example shows how signed messages preserve the identity of each node that handled them:

```rust
use distbench::signing::Signed;

// Each hop in the chain is signed by the node that created it
#[distbench::message]
struct ChainHop {
    hop_number: u32,
    node_name: String,
    original_value: String,
}

// Bundle containing multiple signed messages
#[distbench::message]
struct ChainBundle {
    chain: Vec<Signed<ChainHop>>,
}

#[distbench::state]
pub struct MessageChain {
    #[distbench::config(default = false)]
    is_initiator: bool,
}

#[async_trait]
impl Algorithm for MessageChain {
    async fn on_start(&self) {
        if self.is_initiator {
            // Create and sign the first hop
            let hop = self.sign(ChainHop {
                hop_number: 0,
                node_name: self.id().to_string(),
                original_value: "Hello".to_string(),
            });

            // Display shows: "ChainHop #0 (signed by node1)"
            info!("Created: {}", hop);

            // Send chain with single signed message
            for (_, peer) in self.peers() {
                peer.forward_chain(&ChainBundle {
                    chain: vec![hop.clone()],
                }).await?;
            }
        }
    }
}

#[distbench::handlers]
impl MessageChain {
    async fn forward_chain(&self, src: PeerId, bundle: &ChainBundle) {
        // Log all signed messages in the chain
        info!("Received chain with {} hops:", bundle.chain.len());
        for signed_hop in &bundle.chain {
            // Each displays with its signer
            info!("  {}", signed_hop);
        }

        // Add our signed hop
        let our_hop = self.sign(ChainHop {
            hop_number: bundle.chain.len() as u32,
            node_name: self.id().to_string(),
            original_value: bundle.chain[0].original_value.clone(),
        });

        // Create extended chain
        let mut new_chain = bundle.chain.clone();
        new_chain.push(our_hop);

        // Forward to others
        for (peer_id, peer) in self.peers() {
            if *peer_id != src {
                peer.forward_chain(&ChainBundle {
                    chain: new_chain.clone(),
                }).await?;
            }
        }
    }
}
```

### When to Use Signed vs. Unsigned Messages

**Use unsigned messages** (without `Signed<>`) when:

- Messages are simple data transfers
- The `src: PeerId` parameter provides sufficient sender identification
- You trust the network layer (no Byzantine faults)
- Examples: Echo, simple broadcasts, ring algorithms

**Use signed messages** (`Signed<M>`) when:

- Implementing Byzantine fault-tolerant algorithms
- Messages need to be verified by multiple nodes
- Messages are forwarded through untrusted intermediaries
- You need non-repudiation (proof of who sent what)
- Examples: PBFT, Byzantine broadcast, consensus algorithms

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
2. Use the framework macros (`#[distbench::message]`, `#[distbench::state]`, `#[distbench::handlers]`)
3. Implement the `Algorithm` trait
4. Add a configuration file in `configs/`
5. The build system will automatically register your algorithm

## License

[MIT LICENSE](./LICENSE)

## Troubleshooting

### Algorithm not found

If you get "Unknown algorithm type" error:

- Ensure your algorithm struct has `#[distbench::state]` attribute
- Verify the file is in `src/algorithms/` directory
- Run `cargo clean` and rebuild

### Nodes not synchronizing

If nodes hang during startup:

- Check that all node IDs in config file are unique
- Verify neighbor lists are correct and bidirectional
- Ensure network addresses don't conflict (in network mode)

### Message handler not called

If your message handler isn't being invoked:

- Verify you have `#[distbench::handlers]` on the impl block
- Check that message type matches exactly (case-sensitive)
- Ensure the handler signature matches the pattern: `async fn name(&self, src: PeerId, msg: &MsgType)`
