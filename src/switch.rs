//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::interface::Interface;
use crate::message::Message;
use crate::node::{Address, Center, Node};
use crate::router::Table;
use crate::tcp::Handler;
use crate::topic::TopicBucket;
use crate::transaction::{Class, Transaction};
use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Each message being sent between the Listener Thead and User Thread
/// is either a UserAction or a SwitchAction, only UserAction get
/// actually passed onto the user, others are internal types for
/// managing the thread and the channel.
pub enum Command {
    /// mpsc element for internal communication, like shutting down
    /// the thread or any issues.
    Switch(SwitchAction),
    /// Messages coming from the network intended for the user (or the
    /// other way around).
    User(Transaction),
    /// Internal messages with incomplete data, that will be completed
    /// on the Handler Thread.
    System(SystemAction),
}

/// A collection of possible events for the channel. These are not
/// error messages, since they should be handled automatically, but in
/// the future this part of the system should be made more open by
/// using traits and giving the user the option to specify how to
/// handle all kinds of issues and messages.
#[derive(Debug)]
pub enum SwitchAction {
    /// If the network connection has been terminated or failed. This
    /// only referres to the network connection, not the channel. It
    /// can also be used to signal termination to the thread.
    Terminate,
    /// A heads up message should the cache be full. The system should
    /// handle this automatically, but it can be helpful to be
    /// informed. If it gets sent by the user the cache in the thread
    /// will be cleared.
    Cache,
}

pub enum SystemAction {
    Subscribe(Address),
    Unsubscribe(Address),
    Send(Address, Vec<u8>),
}

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    /// mpsc sender component to send incoming messages to the user.
    /// This will only be done for messages that are intended for the
    /// user, not forwarded messages in the Kademlia system. Since
    /// both directions are needed, the two channels are abstracted
    /// through a dedicated object, which handles both.
    channel: Channel,
    /// New transactions that are intended for the user will be
    /// checked against the cache to see if they are duplicates.
    /// TODO: Define term for "messages intended for the user"
    cache: Cache,
    /// Represents the TCP Handler, currently just one struct. Later
    /// this will be replaced by a trait Object.
    handler: Handler,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: RefCell<Table>,
    /// Holds a list of all currently active topics. The data is in a
    /// RefCell in order to make interactions in the Thread closure
    /// easier.
    topics: RefCell<TopicBucket>,
    /// Another copy of the Center data used for generating messages.
    center: Center,
}

/// A cache of recent Transaction. Since each message might get
/// received multiple times, to avoid processing it more than once a
/// cache is introduced, that stores all recent messages. It has a
/// maximum number of elemets, once that size has been reached the
/// oldest elements will get dropped. This doesn't guarantee each
/// event will only be handled once but it should prevent any
/// duplication under good network conditions. Should a message be
/// delayed by a lot it still possible it gets processed more than
/// once.
struct Cache {
    /// All current Transactions in the cache. Instead of only storing
    /// the messages the entire transactions will get stored, which
    /// should make comparisons faster for larger objects. The array
    /// will be sorted by age on every update.
    elements: Vec<Transaction>,
    /// The maximum size of the cache in number of elements. Once the
    /// size has been reached the oldest element will get dropped to
    /// make space for new Transactions.
    limit: usize,
}

/// TODO: Handle crash / restart cases.
#[derive(Debug)]
pub struct Channel {
    sender: Sender<Command>,
    receiver: Receiver<Command>,
}

impl Channel {
    /// Creates a new pair of Channels. Since two of them are always
    /// connected they have to be created together.
    pub fn new() -> (Self, Self) {
        let (s1, r1) = mpsc::channel();
        let (s2, r2) = mpsc::channel();
        (
            Self {
                sender: s1,
                receiver: r2,
            },
            Self {
                sender: s2,
                receiver: r1,
            },
        )
    }

    /// Sends a message through the Channel. This can fail if the
    /// remote socket is unavailable. Currently this error case is not
    /// handled.
    pub fn send(&self, command: Command) -> Result<(), Error> {
        match self.sender.send(command) {
            Ok(()) => Ok(()),
            Err(_) => Err(Error::Connection(String::from("channel is not available"))),
        }
    }

    /// Like send this is also a wrapper around the mpsc try_recv
    /// method. Currently error are not getting handled and if the
    /// socket is unavailable None will be returned.
    pub fn try_recv(&self) -> Option<Command> {
        match self.receiver.try_recv() {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }

    /// Like send this is also a wrapper around the mpsc recv method.
    /// Currently error are not getting handled and if the socket is
    /// unavailable None will be returned.
    pub fn recv(&self) -> Option<Command> {
        match self.receiver.recv() {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }
}

impl Switch {
    /// Creates a new (Switch, Interface) combo, creating the Cache
    /// and staritng the channel.
    pub fn new(center: Center, limit: usize) -> Result<(Switch, Interface), Error> {
        let (c1, c2) = Channel::new();
        let cache = Cache::new(limit);
        let switch = Switch {
            channel: c1,
            cache,
            handler: Handler::new(center.clone())?,
            table: RefCell::new(Table::new(limit, center.clone())),
            topics: RefCell::new(TopicBucket::new(center.clone())),
            center: center.clone(),
        };
        let interface = Interface::new(c2, center);
        Ok((switch, interface))
    }

    pub fn start(mut self) -> Result<(), Error> {
        thread::spawn(move || loop {
            // tcp messages
            match self.handler.read() {
                Some(wire) => {
                    if !self.cache.exists(wire.uuid) {
                        match wire.convert() {
                            Ok(t) => {
                                log::info!("received new TCP message");
                                log::trace!("new transaction: {:?}", t);
                                let target = t.target();
                                if self.topics.borrow().is_local(&target) {
                                    // Differentiate message classes.
                                    match t.class() {
                                        Class::Ping => {
                                            log::info!(
                                                "received Ping request: {} from: {:?}",
                                                t.uuid,
                                                t.target()
                                            );
                                            let message = Message::new(
                                                Class::Pong,
                                                self.center.public.clone(),
                                                target.clone(),
                                                Vec::new(),
                                            );
                                            let transaction = Transaction::new(message);
                                            let targets = self.table.borrow().get_copy(&target, 3);
                                            for node in targets {
                                                if self
                                                    .handler
                                                    .send(transaction.to_wire(), node)
                                                    .is_err()
                                                {
                                                    // TODO: Deactivate & add to lookup queue.
                                                    log::warn!("unable to reach node");
                                                }
                                            }
                                        }
                                        Class::Pong => {
                                            // TODO Send to Signaling and update queue.
                                            log::info!("received Ping request reply: {}", t.uuid);
                                        }
                                        Class::Subscribe => {
                                            match self.topics.borrow_mut().find_mut(&target) {
                                                Some(topic) => {
                                                    topic.subscribers.add(t.source());
                                                    let action = Command::System(
                                                        SystemAction::Subscribe(t.source()),
                                                    );
                                                    if topic.channel.send(action).is_err() {
                                                        self.topics.borrow_mut().remove(&target);
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                        Class::Unsubscribe => {
                                            match self.topics.borrow_mut().find_mut(&target) {
                                                Some(topic) => {
                                                    topic.subscribers.remove(&t.source());
                                                    let action = Command::System(
                                                        SystemAction::Unsubscribe(t.source()),
                                                    );
                                                    if topic.channel.send(action).is_err() {
                                                        self.topics.borrow_mut().remove(&target);
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                        Class::Lookup => {}
                                        Class::Details => {}
                                        Class::Action => {}
                                    }
                                    self.cache.add(t.clone());
                                    match self.topics.borrow().find(&target) {
                                        Some(topic) => {
                                            if topic.channel.send(Command::User(t)).is_err() {
                                                // If the topic channel is offline, it can be
                                                // assumed the topic in user space
                                                // has been derefed and can be
                                                // disregarded.
                                                //
                                                // TODO: Send unsubscribe message (how to get subscribers?)
                                            }
                                        }
                                        None => {}
                                    }
                                } else {
                                    let source = t.source();
                                    match self.table.borrow().find(&source) {
                                        Some(_) => {}
                                        None => {
                                            let node = Node::new(source, None);
                                            self.table.borrow_mut().add(node);
                                        }
                                    }
                                    // Kademlia
                                    let targets = self.table.borrow().get_copy(&target, 3);
                                    for node in targets {
                                        let target = node.address.clone();
                                        // TODO: Second messaging type
                                        // / send function for owned
                                        // valus.
                                        let e = self.handler.send(t.to_wire(), node);
                                        if e.is_err() {
                                            log::error!("handler thread sending error: {:?}", e);
                                            self.table.borrow_mut().status(&target, false);
                                        } else {
                                            self.table.borrow_mut().status(&target, true);
                                        }
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    } else {
                        // if the message exists in the cache it can
                        // be disregarded.
                    }
                }
                None => {}
            }

            // user messages
            match self.channel.try_recv() {
                Some(data) => {
                    match data {
                        Command::User(t) => {
                            log::info!("sending new message");
                            log::trace!("transaction: {:?}", t);
                            // TODO: number of targets
                            let targets = self.table.borrow().get_copy(&t.target(), 5);
                            log::trace!("found following targets: {:?}", targets);
                            // on tcp fail update the RT (and fail the sending?)
                            for node in targets {
                                // TODO: handle errors
                                let e = self.handler.send(t.to_wire(), node);
                                if e.is_err() {
                                    log::error!("tcp handler failed: {:?}", e);
                                }
                            }
                        }
                        Command::Switch(action) => match action {
                            SwitchAction::Terminate => {
                                log::info!("terminating handler thread");
                                break;
                            }
                            SwitchAction::Cache => {
                                log::info!("emptied the handler cache");
                                self.cache.empty();
                            }
                        },
                        Command::System(action) => match action {
                            SystemAction::Subscribe(_) => {
                                log::warn!("unable to process event without topic");
                            }
                            SystemAction::Unsubscribe(_) => {
                                log::warn!("unable to process event without topic");
                            }
                            SystemAction::Send(address, body) => {
                                let message = Message::new(
                                    Class::Action,
                                    self.center.clone().public,
                                    address.clone(),
                                    body,
                                );
                                let transaction = Transaction::new(message);
                                let targets = self.table.borrow().get_copy(&address, 3);
                                for node in targets {
                                    let e = self.handler.send(transaction.to_wire(), node);
                                    if e.is_err() {
                                        log::error!("unable to connect to node");
                                        // TODO: Update table and
                                        // deactivate the node,
                                        // reschedule for reattemtp.
                                        self.table.borrow_mut().status(&address, false);
                                    } else {
                                        self.table.borrow_mut().status(&address, true);
                                    }
                                }
                            }
                        },
                    }
                }
                None => {}
            }

            // topic messages
            for topic in &self.topics.borrow().topics {
                match topic.channel.try_recv() {
                    Some(data) => {
                        match data {
                            Command::User(t) => {
                                log::info!("sending new message");
                                log::trace!("transaction: {:?}", t);
                                // TODO: number of targets
                                let targets = self.table.borrow().get_copy(&t.target(), 5);
                                log::trace!("found following targets: {:?}", targets);
                                // on tcp fail update the RT (and fail the sending?)
                                for node in targets {
                                    // TODO: handle errors
                                    let e = self.handler.send(t.to_wire(), node);
                                    if e.is_err() {
                                        log::error!("tcp handler failed: {:?}", e);
                                    }
                                }
                            }
                            Command::Switch(action) => match action {
                                SwitchAction::Terminate => {
                                    log::info!("terminating handler thread");
                                    break;
                                }
                                SwitchAction::Cache => {
                                    log::info!("emptied the handler cache");
                                    self.cache.empty();
                                }
                            },
                            Command::System(action) => match action {
                                SystemAction::Subscribe(_address) => {
                                    log::warn!("unable to process event without topic");
                                }
                                SystemAction::Unsubscribe(_address) => {
                                    log::warn!("unable to process event without topic");
                                }
                                SystemAction::Send(address, body) => {
                                    let message = Message::new(
                                        Class::Action,
                                        self.center.public.clone(),
                                        address.clone(),
                                        body,
                                    );
                                    let transaction = Transaction::new(message);
                                    let targets = self.table.borrow().get_copy(&address, 3);
                                    for node in targets {
                                        let e = self.handler.send(transaction.to_wire(), node);
                                        if e.is_err() {
                                            log::error!("unable to connect to node");
                                            // TODO: Update table and
                                            // deactivate the node,
                                            // reschedule for reattemtp.
                                        }
                                    }
                                }
                            },
                        }
                    }
                    None => {}
                }
            }
        });
        Ok(())
    }
}

impl Cache {
    /// Creates a new empty cache with a fixed size limit. In the
    /// future it might be helpful to dynamically change the cache
    /// limit, currently that is not implemented.
    fn new(limit: usize) -> Self {
        Self {
            elements: Vec::new(),
            limit,
        }
    }

    /// Takes a mutable reference to the cache and sorts the elements.
    /// Transaction implements Ord based on the "created" timestamp,
    /// which is used to sort the cache.
    fn sort(&mut self) {
        self.elements.sort()
    }

    /// Adds a new element to the cache. If the cache is full the
    /// oldest element will get removed and the new element gets
    /// added.
    fn add(&mut self, element: Transaction) {
        self.elements.push(element);
        self.sort();
        self.elements.truncate(self.limit);
    }

    /// Clears the cache.
    fn empty(&mut self) {
        self.elements = Vec::new();
    }

    /// Checks if a transaction is already in the cache.
    fn exists(&self, id: [u8; 16]) -> bool {
        match self.find(id) {
            Some(_) => true,
            None => false,
        }
    }

    /// Returns a pointer to a transaction should the same uuid be
    /// stored in the cache. In the future the entire cache could get
    /// restructured to only keep track of uuids.
    fn find(&self, id: [u8; 16]) -> Option<&Transaction> {
        let index = self
            .elements
            .iter()
            .position(|t| t.uuid == uuid::Uuid::from_bytes(id));
        match index {
            Some(i) => self.elements.get(i),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::message::Message;
    use crate::node::Address;
    use crate::transaction::Class;

    #[test]
    fn test_cache_add() {
        let mut c = Cache::new(42);
        let t = transaction();
        c.add(t);
        assert_eq!(c.elements.len(), 1);
    }

    #[test]
    fn test_cache_sort() {
        let mut c = Cache::new(42);
        let first = transaction();
        let second = transaction();
        c.add(second);
        c.add(first.clone());
        c.sort();
        assert_eq!(c.elements[0], first);
    }

    #[test]
    fn test_cache_limit() {
        let mut c = Cache::new(1);
        let first = transaction();
        let second = transaction();
        c.add(second);
        c.add(first.clone());
        assert_eq!(c.elements.len(), 1);
    }

    #[test]
    fn test_channel() {
        let (c1, c2) = Channel::new();
        let t = transaction();
        let h = std::thread::spawn(move || {
            let _ = c1.send(Command::User(t));
        });
        h.join().unwrap();
        let m = c2.recv();
        assert_eq!(m.is_none(), false);
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
