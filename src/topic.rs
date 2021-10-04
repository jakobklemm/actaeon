//! # Topics
//!
//! The main component of the PubSub system. Topics can be used in two
//! ways: They can be stored on their designated target node in the
//! local database or they can represent that remote object for a user
//! to interact with. Each topic has (like most other elements of the
//! system) an Address, which can be generated randomly or from the
//! user.

use crate::error::Error;
use crate::message::{Message, Body};
use crate::node::{Address, Center};
use crate::switch::{Channel, SwitchAction, SwitchCommand};
use crate::transaction::{Class, Transaction};
use std::ops::Deref;

/// The main structure for representing Topics in the system. It will
/// be the main interaction point for the user. Each Topic the user
/// has will also require a copy of the same Topic in the Handler
/// Thread. The only difference between the two is the opposing
/// Channel, with which the two can communicate.
/// TODO: The thread might require a dedicated struct.
/// TODO: Add methods for fetching fields.
#[derive(Debug)]
pub struct Topic {
    /// Throughout the entire system all components have the same
    /// Address type. Each Topic also has a uniqe Address, which can
    /// be generated through any number of ways.
    pub address: Address,
    /// Since each Topic can receive messages individually a dedicated
    /// Channel (mpsc connection) is required.
    pub channel: Channel,
}

/// A simple structure to store a collection of Topics. Since the
/// normal Topics use a custom implementation of Deref the thread has
/// to use a different structure, which is identically but doesn't
/// implement the same methodhs.
pub struct TopicTable {
    pub topics: Vec<HandlerTopic>,
}

/// Since the Topic implements Deref a different structure needs to be
/// used on the thread. Since most of the functions on the topic are
/// wrappers around the Channel, this structure simply has all fields
/// as public. It should only ever used internally.
pub struct HandlerTopic {
    /// Same fields as Topic.
    pub address: Address,
    /// Interactions can be made directly with the Channel instead of
    /// going through an interface.
    pub channel: Channel,
}

impl HandlerTopic {
    /// Takes in a topic and returns a HandlerTopic. Since the two
    /// have identical fields no real conversion is required.
    pub fn convert(topic: Topic) -> Self {
        Self {
            address: topic.address,
            channel: topic.channel,
        }
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
    pub fn send(&self, m: Body) -> Result<(), Error> {
	let m = 
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

    /// Should a Topic held by the user go out of scope it also needs
    /// to be deleted in the Handler thread.
    fn deref(&self) -> &Self::Target {
        let e = self
            .channel
            .send(SwitchCommand::SwitchAction(SwitchAction::Unsubscribe));
        if e.is_err() {
            // this might not work since the Topic will be derefed on
            // both ends.
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
    pub fn add(&mut self, topic: HandlerTopic) {
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
    pub fn get(&self, address: &Address) -> Option<&HandlerTopic> {
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
    fn test_topictable_get() {
        let mut table = TopicTable::new();
        let t = gen_topic();
        table.add(HandlerTopic::convert(t));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_topictable_remove() {
        let mut table = TopicTable::new();
        let t = gen_topic();
        let a = t.address.clone();
        table.add(HandlerTopic::convert(t));
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
