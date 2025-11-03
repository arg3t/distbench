use clap::{Parser, ValueEnum};
use distbench::config::load_config;
use distbench::logging::init_logger;
use distbench::transport_setup::setup_offline_transport;
use framework::community::PeerId;
use framework::transport::channel::{ChannelTransport, ChannelTransportBuilder};
use framework::transport::ThinConnectionManager;
use framework::JsonFormat;
use log::{error, info};
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
    #[arg(long, value_enum, default_value_t = FormatType::Json)]
    format: FormatType,

    /// Timeout for the algorithm to run in seconds
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Verbosity level (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 0)]
    verbose: u8,
}

/// Execution mode for the distributed system.
#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Mode {
    /// Network mode (spawns a process listening on specific IP/port)
    Network,
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

    match (&args.id, &args.mode) {
        (_, Mode::Offline) => {
            run_offline_mode(&args, &config).await;
        }
        _ => {
            // let node_def = config
            //     .get(&args.id)
            //     .unwrap_or_else(|| panic!("Node ID '{}' not found in config file", args.id));
            todo!()
        }
    };
}

/// Runs the distributed system in offline mode.
///
/// Spawns all nodes in the same process using in-memory channels for communication.
///
/// # Arguments
///
/// * `args` - Command-line arguments
/// * `config` - Parsed configuration
async fn run_offline_mode(args: &CliArgs, config: &distbench::config::ConfigFile) {
    let stop_signal = Arc::new(Notify::new());
    let builder = ChannelTransportBuilder::new();
    let mut handles = Vec::new();

    for (node_id_str, node_def) in config.iter() {
        let node_id = PeerId::new(node_id_str.clone());
        let community = setup_offline_transport::<ThinConnectionManager<ChannelTransport>>(
            config,
            &builder,
            &node_id,
            &node_def.neighbours,
        );
        let format = Arc::new(JsonFormat {});

        let serve_handle = start_node!(
            args.algorithm,
            node_def.alg_config,
            node_id,
            community,
            stop_signal.clone(),
            format
        );
        handles.push(serve_handle);
    }

    info!(
        "Started {} nodes. Waiting for {} seconds",
        handles.len(),
        args.timeout
    );

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.timeout)) => {
            info!("Timeout reached. Sending stop signal");
            stop_signal.notify_waiters();
        },
        _ = futures::future::join_all(handles) => {
            tokio::time::sleep(Duration::from_millis(10)).await;
            info!("All nodes finished");
        }
    }
}
