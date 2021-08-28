//! # Instance
//!
//! The program needs one instance which stores the config, the routing table and the local storage engine.

use crate::config::Config;
use crate::engine::Engine;
use crate::router::node::Node;
use crate::router::table::Table;

// The Instance struct has public fields and is meant to be publicly.
pub struct Instance {
    eninge: Option<Box<dyn Engine>>,
}

impl Instance {}
