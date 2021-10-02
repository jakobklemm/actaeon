//! # Topics
//!
//! The main component of the PubSub system. Topics can be used in two
//! ways: They can be stored on their designated target node in the
//! local database or they can represent that remote object for a user
//! to interact with. Each topic has (like most other elements of the
//! system) an Address, which can be generated randomly or from the
//! user.

use crate::node::Address;
use std::time::{SystemTime, UNIX_EPOCH};

struct DataTopic {
    address: Address,
    subscribers: Vec<Address>,
    timestamp: SystemTime,
    length: [u8; 2],
}

pub struct Topic {
    address: Address,
}

impl DataTopic {
    pub fn new(address: Address) -> Self {
        Self {
            address,
            subscribers: Vec::new(),
            timestamp: SystemTime::now(),
            length: [0, 42],
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = self.length.to_vec();
        // This will fail only if the SystemTiem is before UNIX_EPOCH,
        // which is unlikely or easily fixed.
        // TODO: Add SystemTime to startup validation.
        let diff = self.timestamp.duration_since(UNIX_EPOCH).unwrap();
        let time = diff.as_secs().to_be_bytes();
        data.append(&mut self.address.as_bytes().to_vec());
        data.append(&mut time.to_vec());
        for i in &self.subscribers {
            data.append(&mut i.as_bytes().to_vec());
        }
        return data;
    }

    pub fn subscribe(&mut self, address: Address) {
        self.subscribers.push(address);
        self.update_length();
    }

    fn update_length(&mut self) {
        let mut base: usize = 42;
        for _ in 0..self.subscribers.len() {
            base += 32;
        }
        let ins = base % 255;
        let sig = base / 255;
        self.length = [sig as u8, ins as u8];
    }
}

impl Topic {
    pub fn new(address: Address) -> Self {
        Self { address }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datatopic_length_update() {
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        assert_eq!(t.length, [0, 42]);
        t.subscribe(Address::generate("new").unwrap());
        t.update_length();
        assert_eq!(t.length, [0, 74]);
    }

    #[test]
    fn test_datatopic_length_update_wrap() {
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        assert_eq!(t.length, [0, 42]);
        for i in 0..10 {
            t.subscribe(Address::generate(&i.to_string()).unwrap());
        }
        assert_eq!(t.length, [1, 107]);
    }

    #[test]
    fn test_datatopic_bytes() {
        let addr = Address::generate("topic").unwrap();
        let t = DataTopic::new(addr);
        let b = t.as_bytes();
        assert_eq!(b.len(), 42);
        assert_eq!(b[1], 42);
    }
}
