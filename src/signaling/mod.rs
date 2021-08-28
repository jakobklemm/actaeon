//! Signaling
//!
//! The Signaling Object provides lookup data and interfaces with the signaling servers defined in the config.

use crate::router::node::Node;

pub struct Server {
    address: String,
    available: bool,
}

pub struct Signaling {
    servers: Vec<Server>,
    cache: Vec<Node>,
}
