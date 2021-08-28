//! # Config
//!
//! System wide configuration data like network details and system parameters.
//! The config struct can be populated either through hard coded values, loaded from a file or set through command line arguments.

use serde::Deserialize;
use std::fs;
use tracing::info;

#[derive(Deserialize)]
struct LoadConfig {
    node: NodeConfig,
    network: NetworkConfig,
    environment: Environment,
}

#[derive(Deserialize)]
struct NodeConfig {
    address: String,
    port: u16,
    private: String,
    public: String,
}

#[derive(Deserialize)]
struct NetworkConfig {
    size: u16,
    limit: u16,
    signaling: Vec<String>,
}

#[derive(Deserialize)]
struct Environment {
    log: String,
}

// Public configuration struct, fields can be directly accessed.
pub struct Config {
    pub address: String,
    pub port: u16,
    pub private: String,
    pub public: String,
    pub size: u16,
    pub limit: u16,
    // TODO: Replace with signaling servers object
    signaling: Vec<String>,
    pub log: String,
}

impl Config {
    pub fn from_file(path: &str) -> Config {
        let conf = fs::read_to_string(path).expect("Config path not valid!");
        let parsed: LoadConfig = toml::from_str(&conf).expect("Config not valid!");
        info!("Loading configuration from file: {}", path);
        Config {
            address: parsed.node.address,
            port: parsed.node.port,
            private: parsed.node.private,
            public: parsed.node.public,
            size: parsed.network.size,
            limit: parsed.network.limit,
            signaling: parsed.network.signaling,
            log: parsed.environment.log,
        }
    }
}
