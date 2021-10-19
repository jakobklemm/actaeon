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
use crate::switch::{Channel, Command, SystemAction};
use crate::transaction::Transaction;

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

/// Wrapper structure to enable faster operations on all stored
/// subscribers of a Topic.
#[derive(Debug)]
pub struct SubscriberBucket {
    subscribers: Vec<Address>,
}

/// A simple structure to store a collection of Topics. Since the
/// normal Topics use a custom implementation of Deref the thread has
/// to use a different structure, which is identically but doesn't
/// implement the same methodhs.
#[derive(Debug)]
pub struct TopicBucket {
    pub topics: Vec<Topic>,
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

    pub fn recv(&mut self) -> Option<Transaction> {
        loop {
            match self.channel.recv() {
                Some(m) => match m {
                    Command::User(t) => {
                        return Some(t);
                    }
                    Command::System(action) => match action {
                        SystemAction::Subscribe(a) => {
                            self.subscribers.add(a);
                        }
                        SystemAction::Unsubscribe(a) => {
                            let _ = self.subscribers.remove(&a);
                        }
                        SystemAction::Send(_, _) => {
                            log::warn!("unable to process received message type.");
                        }
                    },
                    _ => {
                        continue;
                    }
                },
                None => {
                    return None;
                }
            }
        }
    }

    /// Sends a message to all subscribed Addresses.
    pub fn broadcast(&mut self, body: Vec<u8>) -> Result<(), Error> {
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

        for sub in &self.subscribers.subscribers {
            // TODO! Ownership issues, reduce clone calls.
            let action = Command::System(SystemAction::Send(sub.clone(), body.clone()));
            let e = self.channel.send(action);
            if e.is_err() {
                log::error!("channel is unavailable, it is possible the thread crashed.")
            }
        }
        return Ok(());
    }

    /// Shorthand function to get the Address of a Topic.
    pub fn address(&self) -> Address {
        self.address.clone()
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

impl Iterator for SubscriberBucket {
    type Item = Address;

    fn next(&mut self) -> Option<Self::Item> {
        self.subscribers.pop()
    }
}

impl TopicBucket {
    pub fn new() -> Self {
        Self { topics: Vec::new() }
    }

    pub fn add(&mut self, topic: Topic) {
        if self.find(&topic.address).is_none() {
            self.topics.push(topic)
        }
    }

    pub fn find(&self, search: &Address) -> Option<&Topic> {
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

    pub fn is_local(&self, query: &Address) -> bool {
        match self.find(query) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }
}

#[cfg(test)]
mod tests {
    //use super::*;
}
