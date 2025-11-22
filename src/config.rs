//! Configuration file parsing and management.
//!
//! This module handles loading and parsing YAML configuration files that define
//! the distributed system topology and algorithm parameters.
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Definition of a single node in the configuration.
///
/// Each node has a list of neighbors and algorithm-specific configuration.
#[derive(Deserialize, Debug)]
pub struct NodeDefinition {
    /// IDs of neighboring nodes this node can communicate with
    pub neighbours: Option<Vec<String>>,
    pub host: Option<String>,
    pub port: Option<u16>,

    /// Algorithm-specific configuration (flattened into this struct)
    #[serde(flatten)]
    pub alg_config: serde_json::Value,
}

/// Configuration file format.
///
/// Maps node IDs to their definitions.
pub type ConfigFile = HashMap<String, NodeDefinition>;

/// Loads and parses a configuration file.
///
/// # Arguments
///
/// * `path` - Path to the YAML configuration file
///
/// # Returns
///
/// The parsed configuration mapping node IDs to their definitions, or an error
/// if the file cannot be opened or parsed.
///
/// # Errors
///
/// Returns an error string if:
/// - The file cannot be opened (file not found, permission denied, etc.)
/// - The YAML cannot be parsed (invalid format, syntax errors, etc.)
///
/// # Examples
///
/// ```ignore
/// use std::path::Path;
/// use distbench::config::load_config;
///
/// match load_config(Path::new("config.yaml")) {
///     Ok(config) => println!("Loaded {} nodes", config.len()),
///     Err(e) => eprintln!("Failed to load config: {}", e),
/// }
/// ```
pub fn load_config(path: &Path) -> Result<ConfigFile, String> {
    let config_file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open config file '{}': {}", path.display(), e))?;

    // First, parse as raw YAML to expand merge keys
    let yaml_value: serde_yaml::Value = serde_yaml::from_reader(config_file)
        .map_err(|e| format!("Failed to parse config YAML '{}': {}", path.display(), e))?;

    // Expand merge keys
    let expanded = yaml_merge_keys::merge_keys_serde(yaml_value).map_err(|e| {
        format!(
            "Failed to expand YAML merge keys in '{}': {}",
            path.display(),
            e
        )
    })?;

    // Deserialize into the typed structure
    let mut config: ConfigFile = serde_yaml::from_value(expanded)
        .map_err(|e| format!("Failed to deserialize config '{}': {}", path.display(), e))?;

    // Filter out template keys (keys starting with _)
    config.retain(|key, _| !key.starts_with('_'));

    Ok(config)
}
