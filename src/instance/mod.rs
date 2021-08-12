//! # Instance
//!
//! The program needs one instance which stores the config, the routing table and the local storage engine.

use crate::config::Config;
use crate::router::node::Node;
use crate::router::table::Table;

// The Instance struct has public fields and is meant to be publicly.
pub struct Instance {
    pub config: Config,
    pub table: Table,
}

impl Instance {
    pub fn new(config: Config, center: Node) -> Self {
        tracing::info!("Actaeon is starting!");
        let size = config.limit;
        Self {
            config: config,
            table: Table::new(center, size.into()),
        }
    }
}
