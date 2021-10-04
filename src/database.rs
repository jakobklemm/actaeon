//! # Database
//!
//! Responsible for storing topics and possibly a copy of the routing
//! table on the local file system. All data will be stored in binary,
//! in order to minimize size and make interactions with Wire data as
//! easy as possible.

use crate::node::Address;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Database {
    pub path: String,
}

/// Dedicated datastructure for representing the data in the Database.
/// It also stores a timestamp (which is currently not used) and a
/// dedicated length field, which makes reading it from the Database
/// possible. This will be the kind of Topic representing the "local"
/// topics, so some additional data is required.
pub struct DataTopic {
    /// Same as the Topic Address, main identification of each Topic.
    address: Address,
    /// List of Subscribers, each one currently just consisting of the
    /// Address, not the Node.
    subscribers: Vec<Address>,
    /// Currently unused timestamp of the last time it was used. This
    /// should allow to delete too old Topics.
    timestamp: SystemTime,
    /// Since the Database only stores binary data the length of each
    /// Topic has to be stored directly in the beginning. It consists
    /// of two u8 values:
    ///
    /// - The first one representing the number of 255 byte blocks.
    ///
    /// - The second one stores the number of bytes in the last,
    /// incomplete block.
    ///
    /// This method is a lot easier than having a "length of length"
    /// but with two bytes a maximum object size of 65 kilo bytes is
    /// possible.
    length: [u8; 2],
}

impl Database {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn convert(bytes: Vec<u8>) -> Vec<Vec<u8>> {
        if bytes.len() < 42 {
            return Vec::new();
        }
        let mut raw: Vec<Vec<u8>> = Vec::new();

        let mut i = 0;
        loop {
            if (i + 42) > bytes.len() {
                break;
            }

            let len = [bytes[i], bytes[i + 1]];
            let total = (len[0] as usize) * 255 + len[1] as usize;

            let subset = &bytes[i..(i + total)];

            raw.push(subset.to_vec());

            i += total;
        }

        return raw;
    }
}

impl DataTopic {
    /// Creates a new DataTopic with no subscribers and the current
    /// timestamp. The length will also be initiated correctly.
    pub fn new(address: Address) -> Self {
        Self {
            address,
            subscribers: Vec::new(),
            timestamp: SystemTime::now(),
            length: [0, 42],
        }
    }

    /// Converts a DataTopic to bytes. This could fail if the
    /// SystemTime is off by too much, but that should have been
    /// validated on startup.
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

    /// Adds a new subscriber to the DataTopic. This will have to be
    /// integrated with a partial update function in the Database.
    pub fn subscribe(&mut self, address: Address) {
        self.subscribers.push(address);
        self.update_length();
    }

    /// Computes the updated length for the DataTopic using the
    /// described method.
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::node::Address;

    #[test]
    fn test_convert_one() {
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        let c = Database::convert(b);
        assert_eq!(c.first().unwrap().len(), 74);
    }

    #[test]
    fn test_convert_multi() {
        let mut data = Vec::new();
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        data.append(&mut b.to_vec());
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let n = Address::generate("another").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        data.append(&mut b.to_vec());

        let c = Database::convert(data);
        assert_eq!(c.first().unwrap().len(), 74);
        assert_eq!(c.last().unwrap().len(), 106);
        assert_eq!(c.len(), 2);
    }

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
