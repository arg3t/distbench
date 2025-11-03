use ansi_term::Colour::{Blue, Cyan, Green, Purple, Red, Yellow};
use clap::{Parser, ValueEnum};
use framework::community::{Community, PeerId};
use framework::transport::channel::{ChannelTransport, ChannelTransportBuilder};
use log::info;
use log::Level;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Parser, Debug)]
#[command(author, version, about = "Distributed Systems Framework")]
struct CliArgs {
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

    /// Base IP address for Docker mode (e.g., 10.5.0.0)
    #[arg(long)]
    base_ip: Option<IpAddr>,

    /// Base port for auto-spawned nodes in Local mode
    #[arg(long, default_value_t = 8000)]
    base_port: u16,

    /// Timeout for the algorithm to run in seconds
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 0)]
    verbose: u8,
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
    let args = CliArgs::parse();

    let level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format(|buf, record| {
            use std::io::Write;

            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

            let level = match record.level() {
                Level::Error => Red.paint("ERROR"),
                Level::Warn => Yellow.paint("WARN"),
                Level::Info => Green.paint("INFO"),
                Level::Debug => Blue.paint("DEBUG"),
                Level::Trace => Purple.paint("TRACE"),
            };

            let node_id = framework::get_node_context()
                .map(|id| Cyan.paint(format!("[{}]", id)).to_string())
                .unwrap_or_default();

            writeln!(buf, "{ts} {node_id} {level}: {}", record.args())
        })
        .init();

    let config_file = std::fs::File::open(&args.config).expect("Failed to open config file");
    let config: ConfigFile =
        serde_yaml::from_reader(config_file).expect("Failed to parse config YAML");
    let stop_signal = Arc::new(Notify::new());

    match (args.id, args.mode) {
        (_, Mode::Offline) => {
            let builder = ChannelTransportBuilder::new();
            let mut handles = Vec::new();

            for (node_id_str, node_def) in config.iter() {
                let node_id = PeerId::new(node_id_str.clone());
                let community =
                    setup_local_transport(&config, &builder, &node_id, &node_def.neighbours);

                let serve_future = start_node!(
                    args.algorithm,
                    node_def.alg_config,
                    node_id,
                    community,
                    stop_signal.clone()
                );

                let node_id_for_task = node_id_str.clone();
                let serve_handle =
                    tokio::spawn(framework::NODE_ID_CTX.scope(node_id_for_task, serve_future));

                handles.push(serve_handle);
            }

            info!("Started nodes. Waiting for {} seconds", args.timeout);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(args.timeout)) => {
                    info!("Sending stop signal");
                    stop_signal.notify_waiters();
                },
                _ = futures::future::join_all(handles) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    info!("All nodes finished");
                }
            }
        }
        _ => {
            // let node_def = config
            //     .get(&args.id)
            //     .unwrap_or_else(|| panic!("Node ID '{}' not found in config file", args.id));
            todo!()
        }
    };
}

fn setup_local_transport(
    config: &ConfigFile,
    builder: &ChannelTransportBuilder,
    node_id: &PeerId,
    neighbours: &Vec<String>,
) -> Arc<Community<ChannelTransport>> {
    let transport = builder.build(node_id.clone());

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

    let neighbour_ids: HashSet<PeerId> = neighbours
        .iter()
        .filter_map(|id| {
            let id = PeerId::new(id.clone());
            if id != *node_id {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    let community = Community::new(neighbour_ids, all_node_addrs, transport);

    Arc::new(community)
}
