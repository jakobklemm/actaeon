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
use crate::transaction::Transaction;
use crate::util::Channel;

/// The main structure for representing Topics in the system. It will
/// be the main interaction point for the user. Each Topic the user
/// has will also require a copy of the same Topic in the Handler
/// Thread. The only difference between the two is the opposing
/// Channel, with which the two can communicate.
/// TODO: The thread might require a dedicated struct.
/// TODO: Add methods for fetching fields.
pub struct Topic {
    /// Throughout the entire system all components have the same
    /// Address type. Each Topic also has a uniqe Address, which can
    /// be generated through any number of ways.
    pub address: Address,
    /// Since each Topic can receive messages individually a dedicated
    /// Channel (mpsc connection) is required.
    pub channel: Channel<Command>,
    /// List of subscribers
    pub subscribers: SubscriberBucket,
    /// The socket can get overread so a cache is required.
    pub cache: Vec<Transaction>,
}

/// Since each Topic can interact with the Switch a dedicated enum is
/// used. The user should never have to see any of them and they are
/// only used between a Topic and the Switch.
#[derive(Debug)]
pub enum Command {
    /// Informs the Topic about a new subscriber, mostly going from
    /// the Switch to the user.
    Subscriber(Address),
    /// Opposite of Subscriber.
    Unsubscriber(Address),
    /// Since not all infos about the system (the Center) are known by
    /// the Topic a message going out from the user only gets
    /// constructed on the Switch. The Address sent here is the one of
    /// the subscriber, this message gets sent for every subscriber in
    /// the list.
    Broadcast(Address, Vec<u8>),
    /// Unlike messages from the user, new updates coming from remote
    /// nodes are passed along as entire Transactions, since the user
    /// might want to use values beyond just the body.
    Message(Transaction),
    /// If the Topic goes out of scope the Switch thread (and the rest
    /// of the network) need to be informed. A custom Drop
    /// implementation will send the Drop message to the thread. The
    /// address is not of the Topic but of the subscriber, since each
    /// one gets it individually.
    Drop(Address),
}

/// Wrapper structure to enable faster operations on all stored
/// subscribers of a Topic. This object will be used in each Topic.
#[derive(Debug, Clone)]
pub struct SubscriberBucket {
    /// List of the Addresses of all Subscribers.
    subscribers: Vec<Address>,
}

/// A simple structure to store a collection of Topics. Since the
/// normal Topics use a custom implementation of Deref the thread has
/// to use a different structure, which is identically but doesn't
/// implement the same methods.
pub struct TopicBucket {
    /// List of Topics that will be stored on the Handler Thread.
    pub topics: Vec<Simple>,
}

/// A simplified version of topics that will be used on the Switch
/// thread. The main difference is the lack of the Subscriberbucket,
/// which only gets stored on the user thread.
pub struct Simple {
    /// Matches the Topic Address owned by the user.
    pub address: Address,
    /// Connection to the user Topic.
    pub channel: Channel<Command>,
}

impl Topic {
    /// Creates a new Topic with a given Channel. This Topic is meant
    /// to be used both by the User and the Handler Thread. This
    /// function is not meant to be called by the user, since it
    /// requires the linked Channel to be stored on the Handler
    /// therad. Instead new Topics have to be created through the
    /// interface.
    pub fn new(address: Address, channel: Channel<Command>, subscribers: Vec<Address>) -> Self {
        Self {
            address,
            channel,
            subscribers: SubscriberBucket::new(subscribers),
            cache: Vec::new(),
        }
    }

    /// Blocking call to receive a Message from a Topic. It will only
    /// return once a Message from the system (usually from another
    /// user) is available or the Channel is unavailable. Since
    /// commands are sent over the same Channel the recv method takes
    /// a mutable reference to the Topic and can add / remove
    /// subscribers.
    ///
    /// (Should it receive a Send message it will simply report an
    /// error.)
    pub fn recv(&mut self) -> Option<Transaction> {
        if self.cache.len() != 0 {
            return self.cache.pop();
        }
        loop {
            match self.channel.recv() {
                Some(m) => match m {
                    Command::Message(t) => {
                        return Some(t);
                    }
                    Command::Subscriber(addr) => {
                        if addr != self.address {
                            self.subscribers.add(addr);
                        }
                    }
                    Command::Unsubscriber(addr) => {
                        self.subscribers.remove(&addr);
                    }
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

    /// Behaves the same as "recv" but is non-blocking. Internally it
    /// still uses a loop to filter out non-user messages and will
    /// return on a User message or no message at all.
    pub fn try_recv(&mut self) -> Option<Transaction> {
        if self.cache.len() != 0 {
            return self.cache.pop();
        }
        loop {
            match self.channel.try_recv() {
                Some(m) => match m {
                    Command::Message(t) => {
                        return Some(t);
                    }
                    Command::Subscriber(addr) => {
                        if addr != self.address {
                            self.subscribers.add(addr);
                        }
                    }
                    Command::Unsubscriber(addr) => {
                        self.subscribers.remove(&addr);
                    }
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

    /// The main function for sending Messages to all subscribed
    /// users. It takes in a Vec<u8>, which represents the Body. In
    /// the future this has to be replaced by a Body trait object.
    /// There should also be an option to enable / disable encryption
    /// (but that would require integration with the Transaction &
    /// Wire objects for a dedicated field (or to make encryption
    /// mandatory (will require more tests))).
    pub fn broadcast(&mut self, body: Vec<u8>) -> Result<(), Error> {
        loop {
            println!("data: cache size: {:?}", self.cache.len());
            match self.channel.try_recv() {
                Some(m) => match m {
                    Command::Message(t) => {
                        self.cache.push(t);
                    }
                    Command::Subscriber(addr) => {
                        if addr != self.address {
                            self.subscribers.add(addr);
                        }
                    }
                    Command::Unsubscriber(addr) => {
                        self.subscribers.remove(&addr);
                    }
                    _ => {
                        continue;
                    }
                },
                None => {
                    break;
                }
            }
        }
        println!("data: completed topic loop, sending message");
        println!("data: subscriber length: {:?}", self.subscribers.len());
        for sub in &self.subscribers.subscribers {
            println!("data: sending message to: {:?}", sub);
            // TODO: Ownership issues, reduce clone calls.
            let action = Command::Broadcast(sub.clone(), body.clone());
            let e = self.channel.send(action);
            if e.is_err() {
                log::error!("channel is unavailable, it is possible the thread crashed.")
            }
        }
        println!("data: function exited");
        return Ok(());
    }

    /// In the future this should be replaced by an automatic Drop
    /// implementation, currently a manual "unsubscribe" function is
    /// required to inform other users about the change. It simply
    /// sends an Unsubscribe action to each subscriber.
    pub fn unsubscribe(&mut self) {
        for sub in &self.subscribers.subscribers {
            let action = Command::Drop(sub.clone());
            let e = self.channel.send(action);
            if e.is_err() {
                log::error!("channel is unavailable, it is possible the thread crashed.")
            }
        }
    }

    /// Shorthand function to get the Address of a Topic.
    pub fn address(&self) -> Address {
        self.address.clone()
    }
}

impl Drop for Topic {
    fn drop(&mut self) {
        println!("dropping topic: {:?}", self.address);
        for sub in self.subscribers.clone().into_iter() {
            let command = Command::Drop(sub);
            let _ = self.channel.send(command);
        }
    }
}

impl Simple {
    pub fn new(address: Address, channel: Channel<Command>) -> Self {
        Self { address, channel }
    }
}

impl SubscriberBucket {
    /// Creates a new SubscriberBucket. Currently there are no limits
    /// or other properties so the Bucket is simply an unlimited
    /// Vec.
    pub fn new(subscribers: Vec<Address>) -> Self {
        Self { subscribers }
    }

    /// Will add a new Address to the table. Should the Address
    /// already exist in the Bucket nothing will change. The function
    /// can't fail or return an Error, nothing will happen
    pub fn add(&mut self, address: Address) {
        match self.get(&address) {
            Some(_) => {}
            None => self.subscribers.push(address),
        }
    }

    /// Returns a reference to a specific subscriber with a matching
    /// Address. There isn't really a reason for an end user to use
    /// this (but it is possible for unusual use cases). It will be
    /// called by the "add" function.
    pub fn get(&self, search: &Address) -> Option<&Address> {
        let index = self.subscribers.iter().position(|e| e == search);
        match index {
            Some(i) => self.subscribers.get(i),
            None => None,
        }
    }

    /// Drops a subscriber from the Bucket should an Unsubscribe event
    /// come in.
    pub fn remove(&mut self, target: &Address) {
        let index = self.subscribers.iter().position(|e| e == target);
        match index {
            Some(i) => {
                self.subscribers.remove(i);
            }
            None => {}
        }
    }

    /// Shorthand function for adding many Subscribers at once.
    pub fn add_bulk(&mut self, data: Vec<Address>) {
        data.iter().for_each(|x| self.add(x.clone()));
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

    /// Only adds a Simple if it doesn't exist yet, preventing
    /// duplicates.
    pub fn add(&mut self, simple: Simple) {
        // strange namespace issues
        if TopicBucket::find(&self, &simple.address).is_none() {
            self.topics.push(simple);
        }
    }

    /// Normal (custom) Bucket function for finding a Simple.
    pub fn find(&self, search: &Address) -> Option<&Simple> {
        let index = self.topics.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.topics.get(i),
            None => None,
        }
    }

    /// Normal (custom) Bucket function for finding a mut Simple.
    pub fn find_mut(&mut self, search: &Address) -> Option<&mut Simple> {
        let index = self.topics.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.topics.get_mut(i),
            None => None,
        }
    }

    /// Removes a Simple from the Bucket but won't fail if it doesn't
    /// exist.
    pub fn remove(&mut self, target: &Address) {
        let index = self.topics.iter().position(|e| &e.address == target);
        match index {
            Some(i) => {
                self.topics.remove(i);
            }
            None => {}
        }
    }

    /// Checks if an item exists in the list.
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

impl Iterator for TopicBucket {
    type Item = Simple;

    fn next(&mut self) -> Option<Self::Item> {
        self.topics.pop()
    }
}
