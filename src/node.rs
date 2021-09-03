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

use crate::error::Error;
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::{PublicKey, SecretKey};
use std::time::SystemTime;

#[derive(Clone, Debug, Eq)]
pub struct Node {
    timestamp: SystemTime,
    address: Address,
    link: Option<Link>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

/// Config for self / this node, currently as part of the Node module,
/// might get restructured into a dedicated module in the future,
/// should it increase in scope. It has to be created before all other
/// nodes and stored in the interface.
struct Center {
    public: PublicKey,
    secret: SecretKey,
    uptime: SystemTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Address {
    key: PublicKey,
}

/// TODO: Check if public is required.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    pub ip: String,
    pub port: usize,
    reachable: bool,
    attempts: usize,
}

impl Address {
    /// Create a new Address from a public key. (currently the only
    /// field so it could be created manually.) This is usally not
    /// meant to be used by the user / externally.
    pub fn new(public: PublicKey) -> Self {
        Self { key: public }
    }

    /// Most of the times addresses will be created from bytes coming
    /// over the network, this function can be used, although it might
    /// fail if the key is invalid.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, Error> {
        if let Some(public) = PublicKey::from_slice(&bytes) {
            Ok(Self { key: public })
        } else {
            Err(Error::Invalid(String::from("public key is invalid")))
        }
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes: [u8; 32] = [0; 32];
        let key = self.key.as_ref();
        for (i, j) in key.into_iter().enumerate() {
            bytes[i] = *j;
        }
        return bytes;
    }

    /// If a random Address is required this can generate a public key
    /// from an input string by hashing it.
    pub fn generate(source: &str) -> Result<Self, Error> {
        let bytes = blake3::hash(source.as_bytes()).as_bytes().to_owned();
        Address::from_bytes(bytes)
    }
}

impl Link {
    /// Creates new connection details (Link). It can fail because it
    /// tires to verify the values. Currently the port has to be above
    /// 18, since all ports below that are considered to be reserved.
    /// But this does not actually verify if the port is real or
    /// available.
    pub fn new(ip: String, port: usize) -> Result<Self, Error> {
        if !ip.contains(".") || port <= 18 {
            return Err(Error::Invalid(String::from(
                "link details are not valid / represent impossible network connections",
            )));
        } else {
            return Ok(Self {
                ip,
                port,
                reachable: false,
                attempts: 0,
            });
        }
    }

    pub fn is_reachable(&mut self) {
        self.reachable = true;
    }

    pub fn attempted(&mut self) {
        self.attempts += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;

    #[test]
    fn test_address_from_bytes() {
        let (p, _s) = box_::gen_keypair();
        let real = Address::new(p.clone());
        let test = Address::from_bytes(p.0).unwrap();
        assert_eq!(real.key.0, test.key.0);
    }

    #[test]
    fn test_link_new() {
        let l = Link::new(String::from("127.0.0.1"), 42).unwrap();
        assert_eq!(l.port, 42);
    }

    #[test]
    fn test_link_new_fail() {
        let e = Link::new(String::new(), 0).is_err();
        assert_eq!(e, true);
    }
}
