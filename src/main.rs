use clap::{Parser, ValueEnum};
use framework::community::{Community, PeerId};
use framework::transport::channel::ChannelTransport;
use framework::Node;
use log::{error, info};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about = "Distributed Systems Framework")]
struct CliArgs {
    #[arg(short, long, value_name = "FILE", required = true)]
    config: PathBuf,

    /// ID of this node (must exist in the config)
    #[arg(long, required = true)]
    id: String,

    /// The algorithm to run
    #[arg(short, long, required = true)]
    algorithm: String,

    /// Operating mode
    #[arg(short, long, value_enum, default_value_t = Mode::Local)]
    mode: Mode,

    /// Base IP address for Docker mode (e.g., 10.5.0.0)
    #[arg(long)]
    base_ip: Option<IpAddr>,

    /// Base port for auto-spawned nodes in Local mode
    #[arg(long, default_value_t = 8000)]
    base_port: u16,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Mode {
    /// Local mode (spawns a process listening on varying ports)
    Local,
    /// Docker mode (spawns a process listening on varying ips)
    Docker,
    /// Offline mode (spawns threads, uses tokio channels)
    Offline,
}

#[derive(Deserialize, Debug)]
struct NodeDefinition {
    neighbours: Vec<String>,
    #[serde(flatten)]
    alg_config: serde_json::Value,
}

type ConfigFile = HashMap<String, NodeDefinition>;

include!(concat!(env!("OUT_DIR"), "/registry.rs"));

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = CliArgs::parse();
    info!("Starting node {} with args: {:?}", args.id, args);

    let config_file = std::fs::File::open(&args.config).expect("Failed to open config file");
    let config: ConfigFile =
        serde_yaml::from_reader(config_file).expect("Failed to parse config YAML");

    let node_id = PeerId::new(args.id.clone());
    let node_def = config
        .get(&args.id)
        .unwrap_or_else(|| panic!("Node ID '{}' not found in config file", args.id));

    let (transport, community, _) = match args.mode {
        Mode::Local => setup_local_transport(&config, &node_id, node_def),
        _ => {
            todo!()
        }
    };

    let algo_name = &args.algorithm;
    let node = Node::new(
        transport,
        community,
        algconfig!(algo_name, node_def.alg_config),
    )
    .unwrap_or_else(|e| panic!("Failed to create node: {}", e));

    info!("Starting node {}...", node_id.to_string());

    if let Err(e) = node.start().await {
        error!("Node {} failed to start: {}", node_id.to_string(), e);
    } else {
        info!("Node {} stopped.", node_id.to_string());
    }
}

fn setup_local_transport(
    config: &ConfigFile,
    node_id: &PeerId,
    node_def: &NodeDefinition,
) -> (Arc<ChannelTransport>, Community<ChannelTransport>, PeerId) {
    let all_node_addrs: HashMap<PeerId, PeerId> = config
        .keys()
        .map(|id_str| {
            let id = PeerId::new(id_str.clone());
            (id.clone(), id) // For ChannelTransport, the Address IS the PeerId
        })
        .collect();

    let neighbour_ids: HashSet<PeerId> = node_def
        .neighbours
        .iter()
        .map(|id_str| PeerId::new(id_str.clone()))
        .collect();

    let community = Community::new(neighbour_ids, all_node_addrs);
    let transport = Arc::new(ChannelTransport::new(node_id.clone()));

    (transport, community, node_id.clone())
}
