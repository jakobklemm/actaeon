//! # Topics
//!
//! The main component of the PubSub system. Topics can be used in two
//! ways: They can be stored on their designated target node in the
//! local database or they can represent that remote object for a user
//! to interact with. Each topic has (like most other elements of the
//! system) an Address, which can be generated randomly or from the
//! user.

use crate::error::Error;
use crate::node::Address;
use crate::switch::{Channel, SwitchCommand};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DataTopic {
    address: Address,
    subscribers: Vec<Address>,
    timestamp: SystemTime,
    length: [u8; 2],
}

#[derive(Debug)]
pub struct Topic {
    address: Address,
    channel: Channel,
}

pub struct TopicTable {
    topics: Vec<Topic>,
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
    pub fn new(address: Address, channel: Channel) -> Self {
        Self { address, channel }
    }

    pub fn try_recv(&self) -> Option<SwitchCommand> {
        self.channel.try_recv()
    }

    pub fn recv(&self) -> Option<SwitchCommand> {
        self.channel.recv()
    }

    pub fn send(&self, c: SwitchCommand) -> Result<(), Error> {
        self.channel.send(c)
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }
}

impl TopicTable {
    pub fn new() -> Self {
        Self { topics: Vec::new() }
    }

    pub fn add(&mut self, topic: Topic) {
        self.topics.push(topic);
    }

    pub fn remove(&mut self, target: &Address) -> Result<(), Error> {
        let index = self.topics.iter().position(|e| &e.address == target);
        match index {
            Some(i) => {
                self.topics.remove(i);
                Ok(())
            }
            None => Err(Error::Unknown),
        }
    }

    pub fn get(&self, address: &Address) -> Option<&Topic> {
        let index = self.topics.iter().position(|e| &e.address == address);
        match index {
            Some(i) => self.topics.get(i),
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::transaction::{Class, Transaction};

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

    #[test]
    fn test_topictable_get() {
        let mut table = TopicTable::new();
        let t = gen_topic();
        table.add(t);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_topictable_remove() {
        let mut table = TopicTable::new();
        let t = gen_topic();
        let a = t.address.clone();
        table.add(t);
        table.remove(&a).unwrap();
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_topic_channel() {
        let (c1, c2) = Channel::new();
        let t1 = Topic::new(Address::generate("abc").unwrap(), c1);
        let t2 = Topic::new(Address::generate("abc").unwrap(), c2);
        let handle = std::thread::spawn(move || {
            let _ = t1.send(SwitchCommand::UserAction(transaction()));
        });
        handle.join().unwrap();
        let m = t2.recv();
        assert_eq!(m.is_none(), false);
    }

    fn gen_topic() -> Topic {
        let (channel, _) = Channel::new();
        Topic::new(Address::generate("abc").unwrap(), channel)
    }

    fn transaction() -> Transaction {
        let message = Message::new(
            Class::Ping,
            Address::generate("abc").unwrap(),
            Address::generate("def").unwrap(),
            "test".to_string().as_bytes().to_vec(),
        );
        let t = Transaction::new(message);
        return t;
    }
}
