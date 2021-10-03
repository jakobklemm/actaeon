//! # Topics
//!
//! The main component of the PubSub system. Topics can be used in two
//! ways: They can be stored on their designated target node in the
//! local database or they can represent that remote object for a user
//! to interact with. Each topic has (like most other elements of the
//! system) an Address, which can be generated randomly or from the
//! user.

use crate::error::Error;
use crate::message::Message;
use crate::node::{Address, Center};
use crate::switch::{Channel, SwitchAction, SwitchCommand};
use crate::transaction::{Class, Transaction};
use std::ops::Deref;
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

/// A simple structure to store a collection of Topics. This will be
/// used in two ways, both on the Handler Thread:
///
/// - To store all Topics the user has subscribed to. The table will
/// hold the Channels for receiving Messages.
pub struct TopicTable {
    pub topics: Vec<Topic>,
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
    /// Creates a new Topic with a given Channel. This Topic is meant
    /// to be used both by the User and the Handler Thread. This
    /// function is not meant to be called by the user, since it
    /// requires the linked Channel to be stored on the Handler
    /// therad. Instead new Topics have to be created through the
    /// interface.
    pub fn new(address: Address, channel: Channel) -> Self {
        Self { address, channel }
    }

    /// Tries to get the most recent message from the Topic. It will
    /// never block and will return None should no message be in the
    /// buffer. Currently it will also return None should the Thread
    /// or the Channel be unavailable.
    pub fn try_recv(&self) -> Option<SwitchCommand> {
        self.channel.try_recv()
    }

    /// Same as try_recv, but it will block until a mesage is
    /// available. It will still return an Option, which currently
    /// just represents error states.
    pub fn recv(&self) -> Option<SwitchCommand> {
        self.channel.recv()
    }

    /// Sends a message (SwitchCommand) to the Handler thread, where
    /// it will get sent out over TCP.
    pub fn send(&self, c: SwitchCommand) -> Result<(), Error> {
        self.channel.send(c)
    }

    /// Shorthand function to get the Address of a Topic.
    pub fn address(&self) -> Address {
        self.address.clone()
    }

    /// Constructs a new Transaction to the given Topic. It currently
    /// requires the Center to be passed along in order to get the
    /// source Address, this might have to get reworked.
    pub fn parse(&self, body: Vec<u8>, center: &Center) -> SwitchCommand {
        let message = Message::new(
            Class::Action,
            center.public.clone(),
            self.address.clone(),
            body,
        );
        SwitchCommand::UserAction(Transaction::new(message))
    }
}

impl Deref for Topic {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        let e = self
            .channel
            .send(SwitchCommand::SwitchAction(SwitchAction::Unsubscribe));
        if e.is_err() {
            log::error!("channel no longer available");
        }
        &self.address
    }
}

impl TopicTable {
    /// Constructs a new TopicTable, meant to be called at startup by
    /// the Handler thread.
    pub fn new() -> Self {
        Self { topics: Vec::new() }
    }

    /// Adds a new Topic to the Table. The TopicTable currently has no
    /// size limitations or replacement rules so this will never fail.
    pub fn add(&mut self, topic: Topic) {
        self.topics.push(topic);
    }

    /// Removes a Topic from the Table. It will only fail if the Topic
    /// isn't there.
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

    /// Returns a pointer to the Topic with the matching Address. Will
    /// return None if the Address is unknown.
    pub fn get(&self, address: &Address) -> Option<&Topic> {
        let index = self.topics.iter().position(|e| &e.address == address);
        match index {
            Some(i) => self.topics.get(i),
            None => None,
        }
    }

    /// Shorthand function to get the size of the array.
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
