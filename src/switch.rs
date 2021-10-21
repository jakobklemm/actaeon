//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::config::Config;
use crate::error::Error;
use crate::interface::Interface;
use crate::message::Message;
use crate::node::{Address, Center, Node};
use crate::router::Table;
use crate::signaling::{Action, ActionBucket};
use crate::tcp::Handler;
use crate::topic::{RecordBucket, TopicBucket};
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
    cache: RefCell<Cache>,
    /// Represents the TCP Handler, currently just one struct. Later
    /// this will be replaced by a trait Object.
    handler: Handler,
    /// Each Message will be sent out multiple times to ensure
    /// delivery, currently it is simply hard coded.
    replication: usize,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: RefCell<Table>,
    /// Holds a list of all currently active topics. The data is in a
    /// RefCell in order to make interactions in the Thread closure
    /// easier.
    topics: RefCell<TopicBucket>,
    /// Topics that aren't created / managed by the user, rather are
    /// part of the Kademlia system. Since the Addresses of Topics
    /// need to be fixed / known the location in the system can't be
    /// guaranteed. Instead on the correct nodes a record of that
    /// topic is kept. From there the actual distribution of messages
    /// takes place.
    records: RefCell<RecordBucket>,
    /// Another copy of the Center data used for generating messages.
    center: Center,
    /// Queue of actions for the signaling thread.
    queue: ActionBucket,
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
    pub fn new(center: Center, config: Config, limit: usize) -> Result<(Switch, Interface), Error> {
        let (c1, c2) = Channel::new();
        let cache = Cache::new(limit);
        let queue = ActionBucket::new();
        let switch = Switch {
            channel: c1,
            cache: RefCell::new(cache),
            handler: Handler::new(center.clone())?,
            replication: 3,
            table: RefCell::new(Table::new(limit, center.clone())),
            topics: RefCell::new(TopicBucket::new(center.clone())),
            records: RefCell::new(RecordBucket::new(center.clone())),
            center: center.clone(),
            queue: queue.clone(),
        };
        let interface = Interface::new(c2, config, center, queue);
        Ok((switch, interface))
    }

    pub fn start(mut self) -> Result<(), Error> {
        thread::spawn(move || loop {
            // TODO: Add auto bootstrap (if enabled?)
            // tcp messages
            match self.handler.read() {
                Some(wire) => {
                    // 0. Check if the message is known already.
                    if !self.cache.borrow().exists(wire.uuid) {
                        match wire.convert() {
                            Ok(t) => {
                                // 1. Add it to the cache.
                                self.cache.borrow_mut().add(t.clone());
                                log::info!("received new TCP message: {:?}", t);
                                let target = t.target();
                                let source = t.source();
                                // 2. Add the source to the table if it
                                // doesn't exist already (link state
                                // defaults to None), but only if the
                                // source isn't the Center.
                                if &source == &self.center.public {
                                    match self.table.borrow().find(&source) {
                                        Some(_) => {}
                                        None => {
                                            let node = Node::new(source.clone(), None);
                                            self.table.borrow_mut().add(node);
                                            // TODO: Add signaling lookup
                                        }
                                    }
                                }

                                // 3. Add the target node to the RT,
                                // but only if its not the Center.
                                if &target == &self.center.public {
                                    match self.table.borrow().find(&source) {
                                        Some(_) => {}
                                        None => {
                                            let node = Node::new(source.clone(), None);
                                            self.table.borrow_mut().add(node);
                                            // TODO: Add signaling lookup
                                        }
                                    }

                                    // 4. Handle messages directly for this Node.
                                    // Only some Classes can
                                    // reasonably be sent to a Node
                                    // directly, instead of to a
                                    // Topic. Any other case currently
                                    // just gets dropped.
                                    match t.class() {
                                        // 4.1 A Ping requires a return Pong message. It's
                                        // a new Message with the same
                                        // body (the action UUID).
                                        Class::Ping => {
                                            log::info!(
                                                "received Ping request: {} from: {:?}",
                                                t.uuid,
                                                t.source()
                                            );
                                            let message = Message::new(
                                                Class::Pong,
                                                self.center.public.clone(),
                                                target.clone(),
                                                t.message.body.as_bytes(),
                                            );
                                            let transaction = Transaction::new(message);
                                            let targets = self
                                                .table
                                                .borrow()
                                                .get_copy(&target, self.replication);
                                            for node in targets {
                                                if self
                                                    .handler
                                                    .send(transaction.to_wire(), node)
                                                    .is_err()
                                                {
                                                    // TODO: Add to lookup queue.
                                                    log::warn!("unable to reach node");
                                                    self.table.borrow_mut().status(&source, false);
                                                }
                                            }
                                        }

                                        // 4.2 Getting a Pong requires an update in the
                                        // Signaling queue, the
                                        // message body represents the
                                        // Action Uuid.
                                        Class::Pong => {
                                            log::info!("received Ping request reply: {}", t.uuid);
                                            let bytes = t.message.body.as_bytes();
                                            if bytes.len() == 16 {
                                                let mut uuid = [0; 16];
                                                for (i, j) in bytes.iter().enumerate() {
                                                    uuid[i] = *j;
                                                }
                                                let uuid = uuid::Uuid::from_bytes(uuid);
                                                self.queue.remove(uuid);
                                            }
                                        }

                                        // 4.3 Getting a Lookup requires Node details in
                                        // return as a Details
                                        // request. This contains the
                                        // closest Node to the
                                        // requested one in the body +
                                        // Link data.
                                        Class::Lookup => {
                                            log::info!("received lookup request: {}", t.uuid);
                                            // 4.3.1 Parse the message body (the node to query for)
                                            // into an Address.
                                            let query = t.message.body.as_bytes();
                                            let addr = Address::from_slice(&query);
                                            match addr {
                                                Ok(address) => {
                                                    // 4.3.2 Get the closest known node to that search query.
                                                    let details =
                                                        self.table.borrow().get_copy(&address, 1);
                                                    let result: Node;
                                                    // 4.3.3 If the RT is empty, return the center instead.
                                                    if details.len() == 0 {
                                                        let node = Node::new(
                                                            self.center.public.clone(),
                                                            Some(self.center.link.clone()),
                                                        );
                                                        result = node;
                                                    } else {
                                                        // 4.3.4 Compute the two distances, choose the closer one.
                                                        let node = details.first().unwrap().clone();
                                                        let center_distance =
                                                            &self.center.public ^ &address;
                                                        let table_distance =
                                                            &node.address ^ &address;
                                                        if center_distance >= table_distance {
                                                            let node = Node::new(
                                                                self.center.public.clone(),
                                                                Some(self.center.link.clone()),
                                                            );
                                                            result = node;
                                                        } else {
                                                            // TODO: Use the center over the query if the Link is None
                                                            result = node;
                                                        }
                                                    }
                                                    // 4.3.5 Construct a new message and Transaction, since a new ID is required.
                                                    // The body of the message is the serialized Node.
                                                    let message = Message::new(
                                                        Class::Details,
                                                        self.center.public.clone(),
                                                        target.clone(),
                                                        result.as_bytes(),
                                                    );
                                                    let transaction = Transaction::new(message);
                                                    // 4.3.6 Distribute the Message through the system.
                                                    let targets = self
                                                        .table
                                                        .borrow()
                                                        .get_copy(&target, self.replication);
                                                    for node in targets {
                                                        let send_target = node.address.clone();
                                                        if self
                                                            .handler
                                                            .send(transaction.to_wire(), node)
                                                            .is_err()
                                                        {
                                                            log::warn!(
                                                                "unable to reach node: {:?}",
                                                                send_target
                                                            );
                                                            self.table
                                                                .borrow_mut()
                                                                .status(&source, false);
                                                            let action =
                                                                Action::lookup(send_target);
                                                            self.queue.add(action);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log::warn!("received invalid lookup query: {} for: {:?}", e, query);
                                                }
                                            }
                                        }

                                        // 4.4 Return of a Lookup request. Contains the
                                        // Link of the requested Node
                                        // in the body.
                                        Class::Details => {
                                            // 4.4.1 Construct the Node from the body.
                                            let incoming =
                                                Node::from_bytes(t.message.body.as_bytes());
                                            // 4.4.2 Try to find the Node in the table.
                                            match self
                                                .table
                                                .borrow_mut()
                                                .find_mut(&incoming.address)
                                            {
                                                Some(node) => {
                                                    // 4.4.3 Replace the Node Link with incoming data.
                                                    node.link = incoming.link;
                                                }
                                                None => {
                                                    // 4.4.4 If no Node was found add the new one.
                                                    self.table.borrow_mut().add(incoming);
                                                }
                                            }
                                        }

                                        // 4.5 Requests the entire routing table to be transfered.
                                        // Since it most likely comes
                                        // from a new Node the Link
                                        // details have to be provided
                                        // in the body.
                                        Class::Bootstrap => {
                                            // 4.5.1 Construct the
                                            // body data, which
                                            // consists of the center
                                            // and the serialized RT.
                                            let mut data = Node::new(
                                                self.center.public.clone(),
                                                Some(self.center.link.clone()),
                                            )
                                            .as_bytes();
                                            data.append(&mut self.table.borrow().export());
                                            // 4.5.2 Create the Message & Transaction
                                            let message = Message::new(
                                                Class::Bulk,
                                                self.center.public.clone(),
                                                source.clone(),
                                                data,
                                            );
                                            let transaction = Transaction::new(message);
                                            // 4.5.3 Deliver the Transaction
                                            let targets = self
                                                .table
                                                .borrow()
                                                .get_copy(&source, self.replication);
                                            for node in targets {
                                                if self
                                                    .handler
                                                    .send(transaction.to_wire(), node)
                                                    .is_err()
                                                {
                                                    // TODO: Deactivate & add to lookup queue.
                                                    log::warn!("unable to reach node");
                                                    self.table.borrow_mut().status(&source, false);
                                                }
                                            }
                                        }

                                        // 4.6 Return of a Bootstrap request, containts
                                        // the used signaling node &
                                        // its entire RT in the Message Body.
                                        Class::Bulk => {
                                            t.message
                                                .body
                                                .clone()
                                                .as_bytes()
                                                .chunks(32)
                                                .map(|x| {
                                                    let mut bytes: [u8; 32] = [0; 32];
                                                    for (i, j) in x.iter().enumerate() {
                                                        bytes[i] = *j;
                                                    }
                                                    bytes
                                                })
                                                .map(|x| match Address::from_bytes(x) {
                                                    Ok(a) => Some(Node::new(a, None)),
                                                    Err(e) => {
                                                        log::warn!(
                                                            "received invalid Address data: {}",
                                                            e
                                                        );
                                                        None
                                                    }
                                                })
                                                .for_each(|x| match x {
                                                    Some(n) => self.table.borrow_mut().add(n),
                                                    None => {}
                                                });
                                        }

                                        // 4.7 Messages going from a user to the provider.
                                        Class::Record => {}

                                        // 4.8 Messages from a provider to the subscribers.
                                        Class::Action => {
                                            match self.topics.borrow().find(&target) {
                                                Some(topic) => {
                                                    if topic.channel.send(Command::User(t)).is_err()
                                                    {
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
                                        }
                                        _ => {}
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
                            let targets =
                                self.table.borrow().get_copy(&t.target(), self.replication);
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
                                self.cache.borrow_mut().empty();
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
                                let targets =
                                    self.table.borrow().get_copy(&address, self.replication);
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
                                let targets =
                                    self.table.borrow().get_copy(&t.target(), self.replication);
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
                                    self.cache.borrow_mut().empty();
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
                                    let targets =
                                        self.table.borrow().get_copy(&address, self.replication);
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
