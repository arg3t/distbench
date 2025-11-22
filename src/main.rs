use clap::{Parser, ValueEnum};
use distbench::community::{Community, PeerId};
use distbench::transport::channel::{ChannelTransport, ChannelTransportBuilder};
use distbench::transport::delayed::DelayedTransport;
use distbench::transport::tcp::{TcpAddress, TcpTransport};
use distbench::transport::{ConnectionManager, ThinConnectionManager};
use distbench::{BincodeFormat, Formatter, JsonFormat};
use log::{error, info};
use runner::config::{load_config, ConfigFile};
use runner::logging::init_logger;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::ops::Range;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// Command-line arguments for the distributed algorithms benchmark.
#[derive(Parser, Debug)]
#[command(author, version, about = "Distributed Systems Workbench")]
struct CliArgs {
    /// Path to the YAML configuration file
    #[arg(short, long, value_name = "FILE", required = true)]
    config: PathBuf,

    /// ID of this node (must exist in the config)
    #[arg(long)]
    id: Option<String>,

    /// The algorithm to run
    #[arg(short, long, required = true)]
    algorithm: String,

    /// Operating mode
    #[arg(short, long, value_enum, default_value_t = Mode::Offline)]
    mode: Mode,

    /// Serialization format
    #[arg(short, long, value_enum, default_value_t = FormatType::Bincode)]
    format: FormatType,

    /// Timeout for the algorithm to run in seconds
    #[arg(short, long, default_value_t = 10)]
    timeout: u64,

    /// Verbosity level (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 0)]
    verbose: u8,

    /// Report folder path
    #[arg(short, long)]
    report_folder: Option<PathBuf>,

    /// Base port for local instances, or the port for network mode
    #[arg(short, long, default_value_t = 8000)]
    port_base: u16,

    /// Latency range in milliseconds (e.g. "100-200")
    #[arg(short, long, default_value = "0-0")]
    latency: String,

    /// Startup delay in milliseconds (e.g. "100")
    #[arg(short, long, default_value_t = 200)]
    startup_delay: u64,
}

/// Execution mode for the distributed system.
#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Mode {
    /// Network mode (spawns a process listening on specific IP/port)
    Network,
    /// Local mode (spawns multiple instances listening on different ports)
    Local,
    /// Offline mode (spawns threads, uses tokio channels)
    Offline,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum FormatType {
    Json,
    Bincode,
}

include!(concat!(env!("OUT_DIR"), "/registry.rs"));

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    init_logger(args.verbose);

    let config = match load_config(&args.config) {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    if let Some(folder) = &args.report_folder {
        if let Err(e) = create_dir_all(folder) {
            error!(
                "Failed to create report directory {:?}: {}. Reports will be printed to console.",
                folder, e
            );
        } else {
            info!("Reports will be saved to {:?}", folder);
        }
    }

    let formatter = match args.format {
        FormatType::Json => Arc::new(Formatter::Json(JsonFormat {})),
        FormatType::Bincode => Arc::new(Formatter::Bincode(BincodeFormat {})),
    };

    let latency: Range<u64> = match args
        .latency
        .split('-')
        .map(|s| s.parse::<u64>())
        .collect::<Result<Vec<u64>, _>>()
    {
        Ok(values) => values[0]..values[1],
        Err(e) => {
            error!("Failed to parse latency: {}", e);
            process::exit(1);
        }
    };

    let startup_delay = args.startup_delay;

    match args.mode {
        Mode::Offline => {
            run_offline_mode(&args, &config, formatter, latency, startup_delay).await;
        }
        Mode::Local => {
            run_local_mode(&args, &config, formatter, latency, startup_delay).await;
        }
        Mode::Network => {
            run_network_mode(&args, &config, formatter, latency, startup_delay).await;
        }
    };
}

// --- Mode Implementations ---

/// Runs the distributed system in offline mode.
///
/// Spawns all nodes in the same process using in-memory channels for communication.
async fn run_offline_mode(
    args: &CliArgs,
    config: &runner::config::ConfigFile,
    format: Arc<Formatter>,
    latency: Range<u64>,
    startup_delay: u64,
) {
    let stop_signal = Arc::new(Notify::new());
    let builder = ChannelTransportBuilder::new();
    let mut handles = Vec::new();

    for (node_id_str, node_def) in config.iter() {
        let node_id = PeerId::new(node_id_str.clone());
        let (neighbours_str, _) = get_neighbours(config, node_id_str, &node_id);

        let community = setup_offline_transport::<
            ThinConnectionManager<DelayedTransport<ChannelTransport>>,
        >(config, &builder, &node_id, &neighbours_str, latency.clone());

        let serve_handle = start_node!(
            args.algorithm,
            node_def.alg_config,
            node_id,
            community,
            stop_signal.clone(),
            format,
            startup_delay
        );
        handles.push(serve_handle);
    }

    info!(
        "Started {} nodes in offline mode. Waiting for {} seconds",
        handles.len(),
        args.timeout
    );

    let join_all_fut = futures::future::join_all(handles);
    tokio::pin!(join_all_fut);

    let results = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.timeout)) => {
            info!("Timeout reached. Sending stop signal");
            stop_signal.notify_waiters();
            join_all_fut.await
        },
        results = &mut join_all_fut => {
            tokio::time::sleep(Duration::from_millis(10)).await;
            info!("All nodes finished");
            results
        }
    };

    handle_reports(results, &args.report_folder);
}

/// Runs the distributed system in local mode (all nodes on localhost).
async fn run_local_mode(
    args: &CliArgs,
    config: &runner::config::ConfigFile,
    format: Arc<Formatter>,
    latency: Range<u64>,
    startup_delay: u64,
) {
    let stop_signal = Arc::new(Notify::new());
    let mut handles = Vec::new();

    let (all_peer_addresses, all_peer_ids, peer_id_to_numeric_addr) =
        build_tcp_mappings(config, args.port_base, true);

    let all_peer_addresses = Arc::new(all_peer_addresses);

    info!("Local mode peer map:");
    for (numeric_id, address) in all_peer_addresses.iter() {
        let peer_id = all_peer_ids.get(numeric_id).unwrap();
        info!("  - {} (Node {}): {}", peer_id, numeric_id, address);
    }

    for (node_id_str, node_def) in config.iter() {
        let node_id = PeerId::new(node_id_str.clone());
        let (_, neighbour_ids) = get_neighbours(config, node_id_str, &node_id);

        let community = setup_tcp_transport::<ThinConnectionManager<_>>(
            &all_peer_ids,
            all_peer_addresses.clone(),
            &peer_id_to_numeric_addr,
            &node_id,
            &neighbour_ids,
            latency.clone(),
        );

        let serve_handle = start_node!(
            args.algorithm,
            node_def.alg_config,
            node_id,
            community,
            stop_signal.clone(),
            format,
            startup_delay
        );
        handles.push(serve_handle);
    }

    info!(
        "Started {} nodes in local mode. Waiting for {} seconds",
        handles.len(),
        args.timeout
    );

    let join_all_fut = futures::future::join_all(handles);
    tokio::pin!(join_all_fut);

    let results = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.timeout)) => {
            info!("Timeout reached. Sending stop signal");
            stop_signal.notify_waiters();
            join_all_fut.await
        },
        results = &mut join_all_fut => {
            tokio::time::sleep(Duration::from_millis(10)).await;
            info!("All nodes finished");
            results
        }
    };

    handle_reports(results, &args.report_folder);
}

/// Runs one node in network mode.
async fn run_network_mode(
    args: &CliArgs,
    config: &runner::config::ConfigFile,
    formatter: Arc<Formatter>,
    latency: Range<u64>,
    startup_delay: u64,
) {
    let stop_signal = Arc::new(Notify::new());

    let node_id_str = match &args.id {
        Some(id) => id,
        None => {
            error!("Network mode requires an --id <NODE_ID> argument.");
            process::exit(1);
        }
    };
    let node_id = PeerId::new(node_id_str.clone());
    let node_def = match config.get(node_id_str) {
        Some(def) => def,
        None => {
            error!("Node ID '{}' not found in config file.", node_id_str);
            process::exit(1);
        }
    };

    let (all_peer_addresses, all_peer_ids, peer_id_to_numeric_addr) =
        build_tcp_mappings(config, args.port_base, false);

    let all_peer_addresses = Arc::new(all_peer_addresses);

    let (_, neighbour_ids) = get_neighbours(config, node_id_str, &node_id);

    let community = setup_tcp_transport::<ThinConnectionManager<DelayedTransport<TcpTransport>>>(
        &all_peer_ids,
        all_peer_addresses.clone(),
        &peer_id_to_numeric_addr,
        &node_id,
        &neighbour_ids,
        latency,
    );

    let local_numeric_id = peer_id_to_numeric_addr.get(&node_id).unwrap();
    let local_address = all_peer_addresses.get(local_numeric_id).unwrap();
    info!(
        "Starting node {} (Node {}) on {}...",
        node_id, local_numeric_id, local_address
    );

    let serve_handle = start_node!(
        args.algorithm,
        node_def.alg_config,
        node_id,
        community,
        stop_signal.clone(),
        formatter,
        startup_delay
    );
    let handles = vec![serve_handle]; // Use the same report handler

    // --- 4. Wait for completion ---
    let join_all_fut = futures::future::join_all(handles);
    tokio::pin!(join_all_fut);

    let results = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.timeout)) => {
            info!("Timeout reached. Sending stop signal");
            stop_signal.notify_waiters();
            join_all_fut.await
        },
        results = &mut join_all_fut => {
            tokio::time::sleep(Duration::from_millis(10)).await;
            info!("Node {} finished", node_id_str);
            results
        }
    };

    handle_reports(results, &args.report_folder);
}

// --- Helper Functions ---

/// Sets up an offline (channel-based) transport for a node.
///
/// Creates a community with channel-based connections to all peers defined
/// in the configuration. Channel transport uses in-memory channels instead
/// of network sockets, making it ideal for testing and simulation.
///
/// # Arguments
///
/// * `config` - The complete configuration file
/// * `builder` - The channel transport builder (shared across all nodes)
/// * `node_id` - The ID of this node
/// * `neighbours` - List of neighbor IDs for this node
/// * `latency_ms` - Optional latency in milliseconds
///
/// # Returns
///
/// A community configured with channel transport to all peers.
///
/// # Examples
///
/// ```ignore
/// use distbench::transport::channel::ChannelTransportBuilder;
/// use distbench::community::PeerId;
/// use distbench::transport_setup::setup_offline_transport;
///
/// let builder = ChannelTransportBuilder::new();
/// let node_id = PeerId::new("node1".to_string());
/// let community = setup_offline_transport(
///     &config,
///     &builder,
///     &node_id,
///     &vec!["node2".to_string(), "node3".to_string()],
///     None,
/// );
/// ```
pub fn setup_offline_transport<CM: ConnectionManager<DelayedTransport<ChannelTransport>>>(
    config: &ConfigFile,
    builder: &ChannelTransportBuilder,
    node_id: &PeerId,
    neighbours: &[String],
    latency: Range<u64>,
) -> Arc<Community<DelayedTransport<ChannelTransport>, CM>> {
    let transport = builder.build(node_id.clone());
    let delayed_transport = DelayedTransport::new(transport, latency);

    // Create addresses for all peers (excluding self)
    let all_node_addrs: HashMap<PeerId, PeerId> = config
        .keys()
        .filter_map(|id_str| {
            let id = PeerId::new(id_str.clone());
            if id != *node_id {
                Some((id.clone(), id))
            } else {
                None
            }
        })
        .collect();

    // Convert neighbor strings to PeerIds (excluding self)
    let neighbour_ids: HashSet<PeerId> = neighbours
        .iter()
        .filter_map(|id| {
            let id = PeerId::new(id.clone());
            if id != *node_id {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    let community = Community::new(neighbour_ids, all_node_addrs, delayed_transport);

    Arc::new(community)
}

/// Builds all necessary mappings for TCP transport.
///
/// # Arguments
/// * `config` - The loaded config file.
/// * `port_base` - Base port number.
/// * `is_local` - If true, use 127.0.0.1:port_base+i. If false, use node_id:port_base.
///
/// # Returns
/// Tuple containing:
/// 1. `HashMap<TcpAddress(u16), String>` - Map of numeric IDs to address strings (ip:port or hostname:port).
/// 2. `HashMap<TcpAddress(u16), PeerId>` - Map of numeric IDs to logical PeerIds.
/// 3. `HashMap<PeerId, TcpAddress(u16)>` - Map of logical PeerIds to numeric IDs.
fn build_tcp_mappings(
    config: &ConfigFile,
    port_base: u16,
    is_local: bool,
) -> (
    HashMap<TcpAddress, String>,
    HashMap<TcpAddress, PeerId>,
    HashMap<PeerId, TcpAddress>,
) {
    // Sort IDs to get a stable numeric index
    let mut sorted_node_ids: Vec<_> = config.keys().cloned().collect();
    sorted_node_ids.sort();

    let mut all_peer_addresses = HashMap::new();
    let mut all_peer_ids = HashMap::new();
    let mut peer_id_to_numeric_addr = HashMap::new();

    for (i, node_id_str) in sorted_node_ids.iter().enumerate() {
        let numeric_id = i as u16;
        let tcp_addr = TcpAddress(numeric_id);
        let peer_id = PeerId::new(node_id_str.clone());

        let addr_string = if is_local {
            // Local Mode: Assign 127.0.0.1:port_base + i
            let port = port_base + numeric_id;
            format!("127.0.0.1:{}", port)
        } else {
            // Network Mode: Use node ID as hostname with port_base
            format!("{}:{}", node_id_str, port_base)
        };

        all_peer_addresses.insert(tcp_addr, addr_string);
        all_peer_ids.insert(tcp_addr, peer_id.clone());
        peer_id_to_numeric_addr.insert(peer_id, tcp_addr);
    }

    (all_peer_addresses, all_peer_ids, peer_id_to_numeric_addr)
}

/// Sets up a TCP (network) transport for a node.
fn setup_tcp_transport<CM: ConnectionManager<DelayedTransport<TcpTransport>> + 'static>(
    all_peer_ids: &HashMap<TcpAddress, PeerId>,
    all_peer_addresses: Arc<HashMap<TcpAddress, String>>,
    peer_id_to_numeric_addr: &HashMap<PeerId, TcpAddress>,
    node_id: &PeerId,
    neighbours: &HashSet<PeerId>,
    latency: Range<u64>,
) -> Arc<Community<DelayedTransport<TcpTransport>, CM>> {
    // Find the numeric ID this node should use
    let local_numeric_id = peer_id_to_numeric_addr
        .get(node_id)
        .unwrap_or_else(|| panic!("Failed to find local numeric ID for node {}", node_id));

    // Get the local bind address (we bind to 0.0.0.0:port to accept connections on all interfaces)
    let local_addr_str = all_peer_addresses
        .get(local_numeric_id)
        .unwrap_or_else(|| panic!("Failed to find address for node {}", node_id));

    // Extract port from address string and bind to 0.0.0.0:port
    let port = local_addr_str
        .split(':')
        .nth(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("Failed to parse port from address: {}", local_addr_str));

    let local_bind_addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .unwrap_or_else(|_| panic!("Failed to create bind address for port {}", port));

    let transport = Arc::new(TcpTransport::new(
        *local_numeric_id,
        all_peer_addresses,
        local_bind_addr,
    ));
    let delayed_transport = DelayedTransport::new(transport, latency);

    // Create map of *remote* peers (numeric ID -> PeerId)
    let remote_peer_ids: HashMap<PeerId, TcpAddress> = all_peer_ids
        .iter()
        .filter(|(addr, _)| *addr != local_numeric_id)
        .map(|(addr, id)| (id.clone(), *addr))
        .collect();

    let community = Community::new(neighbours.clone(), remote_peer_ids, delayed_transport);

    Arc::new(community)
}

/// Helper to resolve neighbour list (handles fully-connected case).
fn get_neighbours(
    config: &ConfigFile,
    node_id_str: &str,
    node_id: &PeerId,
) -> (Vec<String>, HashSet<PeerId>) {
    let node_def = config.get(node_id_str).unwrap();

    // If neighbours list is empty, assume fully connected to all *other* nodes
    let neighbours_str = if node_def
        .neighbours
        .as_deref()
        .map_or(true, |n| n.is_empty())
    {
        config
            .keys()
            .filter(|k| *k != node_id_str)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        node_def.neighbours.as_ref().unwrap().to_vec()
    };

    let neighbour_ids: HashSet<PeerId> = neighbours_str
        .iter()
        .map(|id| PeerId::new(id.clone()))
        .filter(|id| id != node_id) // Ensure node is not its own neighbour
        .collect();

    (neighbours_str, neighbour_ids)
}

/// Helper to process node results and write reports.
fn handle_reports(
    results: Vec<(
        PeerId,
        Result<distbench::transport::Result<String>, tokio::task::JoinError>,
    )>,
    report_folder: &Option<PathBuf>,
) {
    for (peer_id, join_result) in results {
        let report = match join_result {
            Err(e) => {
                error!("Task join error: {}", e);
                continue;
            }
            Ok(Err(e)) => {
                error!("Node {} returned error: {}", peer_id, e);
                continue;
            }
            Ok(Ok(report)) => report,
        };

        if let Some(folder) = report_folder {
            let file_path = folder.join(format!("{}.json", peer_id));
            let line = format!("{}\n", report);

            match OpenOptions::new()
                .create(true)
                .write(true) // Overwrite file instead of appending
                .truncate(true)
                .open(&file_path)
            {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(line.as_bytes()) {
                        error!("Failed to write report for {}: {}", peer_id, e);
                    }
                }
                Err(e) => {
                    error!("Failed to open report file for {}: {}", peer_id, e);
                }
            }
        } else {
            // No report folder, just print to info log
            info!("Report for {}: {}", peer_id, report);
        }
    }
}
