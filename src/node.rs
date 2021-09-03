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

struct Node {
    timestamp: SystemTime,
    address: Address,
    connection: Connection,
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

struct Address {
    key: PublicKey,
}

struct Connection {
    ip: String,
    port: usize,
}

impl Address {
    /// Create a new Address from a public key. (currently the only
    /// field so it could be created manually.) This is usally not
    /// meant to be used by the user / externally.
    fn new(public: PublicKey) -> Self {
        Self { key: public }
    }

    /// Most of the times addresses will be created from bytes coming
    /// over the network, this function can be used, although it might
    /// fail if the key is invalid.
    fn from_bytes(bytes: [u8; 32]) -> Result<Self, Error> {
        if let Some(public) = PublicKey::from_slice(&bytes) {
            Ok(Self { key: public })
        } else {
            Err(Error::Invalid(String::from("public key is invalid")))
        }
    }

    fn as_bytes(&self) -> [u8; 32] {
        let mut bytes: [u8; 32] = [0; 32];
        let key = self.key.as_ref();
        for (i, j) in key.into_iter().enumerate() {
            bytes[i] = *j;
        }
        return bytes;
    }

    /// If a random Address is required this can generate a public key
    /// from an input string by hashing it.
    fn generate(source: &str) -> Result<Self, Error> {
        let bytes = blake3::hash(source.as_bytes()).as_bytes().to_owned();
        Address::from_bytes(bytes)
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
}
