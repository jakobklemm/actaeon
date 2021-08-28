//! # Router Address
//!
//! Kademlia style addresses to be used in the XOR distance metric.

// The actual address is calcuated through the hash of the public key
// of the node. This ensures the key is valid, otherwise the data
// would be wrongly encrypted. All fields are private since they
// should not be changable by the user, only with the key.

use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Address {
    pub bytes: [u8; 32],
    pub public: Option<String>,
    verified: bool,
}

#[derive(Clone, Debug)]
pub struct Connection {
    ip: String,
    port: u16,
}

impl Address {
    pub fn new(public: &str) -> Self {
        let hash = blake3::hash(public.as_bytes()).as_bytes().to_owned();
        Self {
            bytes: hash,
            public: Some(public.to_string()),
            verified: true,
        }
    }

    pub fn from_message(bytes: [u8; 32]) -> Self {
        Self {
            bytes: bytes,
            public: None,
            verified: false,
        }
    }

    pub fn serialize(&self) -> &[u8] {
        &self.bytes
    }

    pub fn distance(&self, source: &Self) -> [u8; 32] {
        let mut d: [u8; 32] = [0; 32];
        for i in 0..(self.bytes.len()) {
            d[i] = self.bytes[i] ^ source.bytes[i];
        }
        return d;
    }

    pub fn bucket(&self, center: &Address) -> usize {
        let distance = self.distance(center);
        distance[0] as usize
    }

    // Not the trait implementation, since more parameters are required.
    pub fn cmp(&self, other: &Address, center: &Address) -> Ordering {
        let first = self.distance(center);
        let second = other.distance(center);
        first.cmp(&second)
    }
}

impl Connection {
    pub fn new(ip: &str, port: u16) -> Self {
        Self {
            ip: ip.to_string(),
            port: port,
        }
    }
}
