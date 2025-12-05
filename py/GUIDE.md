# Distbench Guide

## Running the framework

### Commands to Run

To execute an algorithm:

```bash
uv run distbench -- \
  --config configs/your_config.yaml \
  --algorithm your_algorithm_name \
  --mode [offline|local|network] \
  [--id NODE_ID (only for network mode)] \
  [-s | --port-base PORT (base port for local, or port for network mode, default: 8000)] \
  [-f | --format [json|msgpack] (serialization format, default: msgpack)] \
  [-t | --timeout SECONDS (timeout for the algorithm in seconds, default: 30)] \
  [-v | -vv | -vvv (logging verbosity)] \
  [-r | --report-folder PATH (path to save reports)] \
  [-l | --latency RANGE (latency range in ms, e.g. "100-200", default: "0-0")] \
  [-d | --startup-delay MS (startup delay in ms, default: 200)]
```

**Examples:**

- **Offline Mode:**
  ```bash
  uv run distbench -- -c configs/echo.yaml -a echo --mode offline -t 30
  ```
- **Local Mode:**
  ```bash
  uv run distbench -- -c configs/echo.yaml -a echo --mode local -s 10000 -t 30
  ```
- **Network Mode (Terminal 1 for node1):**
  ```bash
  uv run distbench -- -c configs/echo.yaml -a echo --mode network --id n1 -t 30
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

Use the `@message` decorator on a Python dataclass to define data structures exchanged between nodes.

```python
from distbench import message

@message
class Ping:
    sequence: int
```

### Algorithm State

Decorate your algorithm class with `@distbench` and define configuration fields using `config_field()`. Internal state can be simple instance variables (`self.my_var`).

```python
from distbench import Algorithm, distbench, config_field
import logging

logger = logging.getLogger(__name__)

@distbench
class PingPong(Algorithm):
    # Configuration fields (loaded from YAML)
    initiator: bool = config_field(required=True)
    max_rounds: int = config_field(default=5)

    def __init__(self):
        super().__init__()
        self.pings_received = 0 # No Mutex needed for internal state
```

### Lifecycle

Implement `async` methods inherited from the `Algorithm` base class for lifecycle hooks.

```python
from distbench import Algorithm # Already imported above
import logging # Already imported above

class PingPong(Algorithm): # Assuming continuation from above
    async def on_start(self) -> None:
        """Called when all nodes are ready."""
        logger.info(f"Node {self.id()} starting")

    async def on_exit(self) -> None:
        """Called during shutdown for cleanup."""
        logger.info("PingPong finished")

    async def report(self) -> dict[str, str]:
        """Optional: return metrics/results as a dictionary."""
        return {"pings_received": str(self.pings_received)}
```

### Handlers

Add `async` methods marked with `@handler` to your class to define message handlers. The framework automatically generates peer proxy methods (`peer.handler_name(msg)`).

```python
from distbench import PeerId, handler # PeerId imported above

class PingPong(Algorithm): # Assuming continuation from above
    @handler
    async def ping(self, src: PeerId, msg: Ping) -> str:
        """Handles an incoming Ping message (request-response)."""
        logger.info(f"Received Ping {msg.sequence} from {src}")
        return f"ACK_Ping_{msg.sequence}"

    @handler
    async def pong(self, src: PeerId, msg: Pong) -> None:
        """Handles an incoming Pong message (fire-and-forget)."""
        logger.info(f"Received Pong {msg.sequence} from {src}")
        # No return value means this is a "cast" (fire-and-forget)
```

**Handler Patterns:**

- `async def handler_name(self, src: PeerId, msg: MessageType) -> ResponseType:`: Defines a **request-response** message. The caller will `await` the response.
- `async def handler_name(self, src: PeerId, msg: MessageType) -> None:`: Defines a **fire-and-forget (cast)** message. The caller will not `await` a response.

### Algorithm Layering (Child Algorithms and Interfaces)

Compose complex protocols from simpler building blocks.

- **Parent defines child:** Use `child_algorithm()` in the parent's class definition.

  ```python
  from distbench import Algorithm, child_algorithm, distbench, config_field
  from distbench.algorithms import LowerLayer

  @distbench
  class UpperLayer(Algorithm):
      # Child algorithm
      broadcast: LowerLayer = child_algorithm(LowerLayer)
  ```

- **Child defines interface:** Use the `@interface` decorator for methods callable by the parent. These methods receive raw `bytes`. Child can deliver messages to its parent via `await self.deliver(src, msg_bytes)`.

  ```python
  from distbench.decorators import interface

  class LowerLayer(Algorithm):
      @interface
      async def broadcast(self, msg_bytes: bytes, extra_param: bool) -> None:
          # msg_bytes is the SERIALIZED AlgorithmMessage payload
          pass
  ```

- **Parent calls child interface:** Parent passes message objects directly; the framework handles serialization.
  ```python
  class UpperLayer(Algorithm): # Assuming continuation
      async def on_start(self) -> None:
          if self.id() == PeerId("n1"):
              await self.broadcast.broadcast(Ping(sequence=0)) # Pass message object
  ```
- **Parent intercepts child messages:** Use `@handler(from_child="child_name")`. The framework deserializes bytes to the parent's typed handlers.

  ```python
  from distbench import handler

  class UpperLayer(Algorithm): # Assuming continuation
      @handler(from_child="broadcast")
      async def on_ping_from_child(self, src: PeerId, msg: Ping) -> None:
          logger.info(f"Intercepted Ping from child: {msg.sequence} from {src}")
  ```

### Available Utility Properties (on `self`)

- `self.id -> PeerId`: Returns the unique `PeerId` of the current node.
- `self.N -> int`: Returns the total number of nodes in the system (including this one).
- `self.peers -> dict[PeerId, Peer]`: A dictionary mapping all other `PeerId`s to their `Peer` proxy objects for sending messages.
- `self.nodes -> set[PeerId]`: Returns a set of all the peers' ids in the network.
- `await self.terminate()`: Signals to the framework that this node has completed its execution. All nodes must call this for clean shutdown.
- `self.sign(message: T) -> Signed[T]`: Creates a cryptographically signed wrapper for a message.
- `async self.deliver(src, msg_bytes)`: Pass a message up to the parent algorithm

## Signed Messages

The framework provides a `Signed[T]` wrapper for asymmetric signatures. The Python framework **automatically verifies all incoming `Signed[T]` messages** before your handler is called. If the signature is invalid the message is dropped.

```python
from distbench.signing import Signed
from distbench import message, handler
from dataclasses import dataclass

@message
@dataclass
class Vote:
    value: bool

class MyAlgorithm(Algorithm): # Assuming continuation
    @handler
    async def receive_vote(self, src: PeerId, msg: Signed[Vote]) -> None:
        # Access inner fields directly. Framework has already verified the signature.
        if msg.signer != src: return   # Need to verify if src is also the signer
        logger.info(f"Received vote: {msg.value} from {msg.signer}")
        logger.info(f"Full message: {msg}") # Prints "Vote(value=True) (signed by n1)"
```

## Configuration

### File Format

YAML configuration maps node IDs to node definitions:

```yaml
node_id:
  neighbours: [list of neighbor node IDs]
  host: "network address" # For network/local mode
  port: port_number # For network/local mode
  algorithm_field: value # Passed to @distbench config_field
```

### Fully Connected Topology

Use an empty `neighbours: []` list to connect a node to all other nodes in the file, or just omit the field.

```yaml
n1:
  neighbours: [] # Connects to n2, n3
  is_sender: true
```

### Using YAML Anchors for Reusability

```yaml
# Shared template (starts with _ to indicate it's not a node)
_template: &config_template
  my_config_param: 123

n1:
  <<: *config_template # Merge template
  neighbours: [n2, n3]
  start_node: true
```

### Nested Configuration for Child Algorithms

Child algorithms are configured using nested structures in the YAML configuration file, mirroring the `child_algorithm()` declaration in the parent's class.

```yaml
n1:
  neighbours: [n2, n3]
  start_node: true
  # Child algorithm configuration
  broadcast:
    is_sender: true # This maps to LowerLayer.is_sender

n2:
  neighbours: [n1, n3]
  start_node: false
  broadcast:
    is_sender: false
```

## Docker Deployment

Distbench can run in Docker containers with each node in a separate container. The system uses node IDs as hostnames in a dedicated Docker network.

### Quick Start

1.  **Generate Docker Compose File:**
    ```bash
    python3 dockerize.py \
      -c configs/echo.yaml \
      -a echo \
      -o docker-compose.yaml
    ```
2.  **Run the System:**
    ```bash
    docker-compose up --build         # Build and start all containers
    docker-compose up --build -d      # Run in detached mode
    docker-compose logs -f            # View logs
    docker-compose down               # Stop and clean up
    ```
