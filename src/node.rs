//! # Node
//!
//! Datastructures and functions related to representing members of the
//! network and how to connect to them.
//!
//! The functionality is split into two core areas:
//!
//! - Address: The kademlia-like routing details for finding a node in the
//!   decentralized routing system.
//!
//! - Connection: How to establish a connection to a node using direct
//!   TCP/UDP connections. In the future this will have to be
//!   modularized further to allow for different transport layers and
//!   protocol. (TODO: Integrate into proxy / indirect system).
//!
//! In addition each node also contains other fields like timestamps
//! and (in the future) a cache of recent messages.

use std::time::SystemTime;

struct Node<'a> {
    timestamp: SystemTime,
    address: Address<'a>,
    connection: Connection,
}

/// Current short hand for the self node, used for calculating the distance in the Ord impl.
struct Center([u8; 32]);

struct Address<'a> {
    id: [u8; 32],
    center: &'a Center,
}

struct Connection {}
