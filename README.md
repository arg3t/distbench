# Distbench

> [!NOTE]
> This is the Rust implementation of Distbench. For the Python version, see the [python](./py) directory.

A Rust framework for implementing and testing distributed algorithms

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

Distbench handles the infrastructure concerns (networking, message passing, node lifecycle) so you can focus on algorithm logic. It was built for the lab assignments in TU Delft’s [Distributed Algorithms](https://studyguide.tudelft.nl/courses/study-guide/educations/14765) course of the Computer Science master’s program. A [python](./py) version is also available.

## Features

- Clean API - Procedural macros eliminate boilerplate
- Pluggable Transports - In-memory channels or TCP sockets
- Multiple Formats - JSON or Bincode serialization
- Three Execution Modes - Offline, Local (via localhost), Network
- Cryptographic Signing - Built-in Ed25519 signatures for Byzantine algorithms
- Automatic Lifecycle - Node synchronization and coordination handled for you
- Algorithm Layering - Compose complex protocols from simpler building blocks

## Quick Start

### Installation

```bash
git clone https://github.com/yourusername/distbench
cd distbench
cargo build --release
```

### Running an Example

Run a basic Echo algorithm:

```bash
cargo run --release -- \
  --config configs/echo.yaml \
  --algorithm Echo \
  --mode offline \
  --timeout 5
```

## Example Algorithms

- **[Echo](src/algorithms/echo.rs)** - Simple request-response pattern
- **[Chang-Roberts](src/algorithms/chang_roberts.rs)** - Ring-based leader election
- **[Message Chain](src/algorithms/message_chain.rs)** - Demonstrates cryptographic signatures
- **[Simple Broadcast](src/algorithms/simple_broadcast.rs)** - Demonstrates algorithm layering with parent-child communication

## Documentation

- **[Implementation Guide](GUIDE.md)** - Learn how to implement your own algorithms
- **[Architecture](ARCHITECTURE.md)** - Deep dive into framework design

## Usage

### Command-Line Options

```
Options:
  -c, --config <FILE>       Path to configuration file
  -a, --algorithm <NAME>    Algorithm to run
  -m, --mode <MODE>         Execution mode: offline, local or network [default: offline]
  -p, --port-base <PORT>    First port used when spawning local instances
      --id <ID>             The id of the node to spawn (when running in network mode)
      --format <FORMAT>     Serialization: json or bincode [default: json]
      --timeout <SECONDS>   Timeout in seconds [default: 10]
  -v, --verbose             Increase verbosity (-v, -vv, -vvv)
  -l, --latency <LATENCY>   Latency range in milliseconds [default: 0-0]
  -s, --startup-delay <MS>  Startup delay in millis [default: 0]
```

### Configuration

Create a YAML configuration file:

```yaml
node1:
  neighbours: [node2, node3] # Or [] for fully connected
  is_sender: true
  max_rounds: 10

node2:
  neighbours: [node1, node3]
  is_sender: false

node3:
  neighbours: [node1, node2]
  is_sender: false
```

## Implementing an Algorithm

See the **[Implementation Guide](GUIDE.md)** for a detailed introduction to the framework.

Quick overview:

1. Define message types with `#[distbench::message]`
2. Define algorithm state with `#[distbench::state]`
3. Implement the `Algorithm` trait (lifecycle hooks)
4. Define message handlers with `#[distbench::handlers]`
5. Optionally compose algorithms using `#[distbench::child]` for layering
6. Create a configuration file (with YAML anchors for reusable configs)
7. Run with `cargo run -- --config config.yaml --algorithm YourAlgorithm`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-algorithm`)
3. Commit your changes (`git commit -m 'Add amazing algorithm'`)
4. Push to the branch (`git push origin feature/amazing-algorithm`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- Uses [serde](https://serde.rs/) for serialization
- Cryptography via [ed25519-dalek](https://github.com/dalek-cryptography/ed25519-dalek)

## Contact

- **Issues**: [GitHub Issues](https://github.com/arg3t/distbench/issues)
- **Discussions**: [GitHub Discussions](https://github.com/arg3t/distbench/discussions)
