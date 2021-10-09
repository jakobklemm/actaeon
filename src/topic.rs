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
use crate::switch::{Channel, Command, SwitchAction, SystemAction};
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
    /// List of subscribers
    subscribers: SubscriberBucket,
}

/// Since the Topic implements Deref a different structure needs to be
/// used on the thread. Since most of the functions on the topic are
/// wrappers around the Channel, this structure simply has all fields
/// as public. It should only ever used internally.
#[derive(Debug)]
pub struct HandlerTopic {
    /// Same fields as Topic.
    pub address: Address,
    /// Interactions can be made directly with the Channel instead of
    /// going through an interface.
    pub channel: Channel,
}

/// A simple structure to store a collection of Topics. Since the
/// normal Topics use a custom implementation of Deref the thread has
/// to use a different structure, which is identically but doesn't
/// implement the same methodhs.
#[derive(Debug)]
pub struct TopicBucket {
    topics: Vec<HandlerTopic>,
}

#[derive(Debug)]
struct SubscriberBucket {
    subscribers: Vec<Address>,
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
    pub fn new(address: Address, channel: Channel, subscribers: Vec<Address>) -> Self {
        Self {
            address,
            channel,
            subscribers: SubscriberBucket::new(subscribers),
        }
    }

    pub fn recv(&mut self) -> Option<Message> {
        match self.channel.recv() {
            Some(m) => match m {
                Command::User(t) => {
                    return Some(t.message);
                }
                Command::System(action) => match action {
                    SystemAction::Subscribe(a) => {
                        self.subscribers.add(a);
                        return None;
                    }
                    SystemAction::Unsubscribe(a) => {
                        let _ = self.subscribers.remove(&a);
                        return None;
                    }
                },
                _ => {
                    return None;
                }
            },
            None => {
                return None;
            }
        }
    }

    /// Sends a message to all subscribed Addresses.
    pub fn broadcast(&mut self, t: Transaction) -> Result<(), Error> {
        loop {
            match self.channel.try_recv() {
                Some(m) => match m {
                    Command::System(action) => match action {
                        SystemAction::Subscribe(a) => self.subscribers.add(a),
                        SystemAction::Unsubscribe(a) => self.subscribers.add(a),
                        _ => continue,
                    },
                    _ => continue,
                },
                None => {
                    break;
                }
            }
        }
        self.channel.send(Command::User(t))
    }

    /// Shorthand function to get the Address of a Topic.
    pub fn address(&self) -> Address {
        self.address.clone()
    }
}

impl Deref for Topic {
    type Target = Address;

    /// Should a Topic held by the user go out of scope it also needs
    /// to be deleted in the Handler thread.
    fn deref(&self) -> &Self::Target {
        let e = self
            .channel
            .send(Command::Switch(SwitchAction::Unsubscribe));
        if e.is_err() {
            // this might not work since the Topic will be derefed on
            // both ends.
            log::error!("channel no longer available");
        }
        &self.address
    }
}

impl SubscriberBucket {
    pub fn new(subscribers: Vec<Address>) -> Self {
        Self { subscribers }
    }

    pub fn sort(&mut self) {
        self.subscribers.sort();
    }

    pub fn add(&mut self, address: Address) {
        self.subscribers.push(address)
    }

    pub fn find(&self, search: &Address) -> Option<&Address> {
        let index = self.subscribers.iter().position(|e| e == search);
        match index {
            Some(i) => self.subscribers.get(i),
            None => None,
        }
    }

    pub fn remove(&mut self, target: &Address) -> Result<(), Error> {
        let index = self.subscribers.iter().position(|e| e == target);
        match index {
            Some(i) => {
                self.subscribers.remove(i);
                Ok(())
            }
            None => Err(Error::Unknown),
        }
    }

    pub fn dedup(&mut self) {
        self.sort();
        self.subscribers.dedup_by(|a, b| a == b);
    }

    pub fn len(&self) -> usize {
        self.subscribers.len()
    }
}

impl TopicBucket {
    pub fn new() -> Self {
        Self { topics: Vec::new() }
    }

    pub fn add(&mut self, topic: HandlerTopic) {
        if self.find(&topic.address).is_none() {
            self.topics.push(topic)
        }
    }

    pub fn find(&self, search: &Address) -> Option<&HandlerTopic> {
        let index = self.topics.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.topics.get(i),
            None => None,
        }
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
        let mut table = TopicBucket::new();
        let t = gen_topic();
        table.add(HandlerTopic::convert(t));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_topictable_remove() {
        let mut table = TopicBucket::new();
        let t = gen_topic();
        let a = t.address.clone();
        table.add(HandlerTopic::convert(t));
        table.remove(&a).unwrap();
        assert_eq!(table.len(), 0);
    }

    fn gen_topic() -> Topic {
        let (channel, _) = Channel::new();
        Topic::new(Address::generate("abc").unwrap(), channel, Vec::new())
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
