//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::handler::{HandlerAction, Listener};
use crate::interface::InterfaceAction;
use crate::message::Message;
use crate::node::{Address, Center, Link, Node};
use crate::router::Table;
use crate::signaling::SignalingAction;
use crate::topic::Topic;
use crate::topic::{Record, RecordBucket, TopicBucket};
use crate::transaction::{Class, Transaction, Wire};
use crate::util::Channel;
use std::cell::RefCell;
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
}

pub enum SystemAction {
    /// A new topic gets created on the user thread, the channel has
    /// to be sent to the handler thread.
    Subscribe(Topic),
    /// Usually coming from the Thread to the user informing them of
    /// a new subscriber.
    Subscriber(Vec<Address>),
    /// Removes the topic from the tables & distributes the update.
    Unsubscribe(Address),
    /// Almost the same as UserAction, but in the topic the center
    /// might be unknown, so just the body can be transfered.
    Send(Address, Vec<u8>),
}

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    listener: Channel<HandlerAction>,
    interface: Channel<InterfaceAction>,
    signaling: Channel<SignalingAction>,
    /// New transactions that are intended for the user will be
    /// checked against the cache to see if they are duplicates.
    /// TODO: Define term for "messages intended for the user"
    cache: RefCell<Cache>,
    /// Each Message will be sent out multiple times to ensure
    /// delivery, currently it is simply hard coded.
    replication: usize,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: RefCell<Table>,
    /// Holds a list of all currently active topics. The data is in a
    /// RefCell in order to make interactions in the Thread closure
    /// easier. A Topic means a non-"should be local" Address
    /// subscribed to by the user.
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

impl Switch {
    /// Creates a new (Switch, Interface) combo, creating the Cache
    /// and staritng the channel.
    pub fn new(
        listener: Channel<HandlerAction>,
        interface: Channel<InterfaceAction>,
        signaling: Channel<SignalingAction>,
        center: Center,
        replication: usize,
        limit: usize,
    ) -> Result<Self, Error> {
        let cache = Cache::new(limit);
        let switch = Switch {
            listener,
            interface,
            signaling,
            cache: RefCell::new(cache),
            replication,
            table: RefCell::new(Table::new(limit, center.clone())),
            topics: RefCell::new(TopicBucket::new(center.clone())),
            records: RefCell::new(RecordBucket::new(center.clone())),
            center: center,
        };
        Ok(switch)
    }

    /// The switch is responsible for deciding where specific messages
    /// go based on their origin, target and type. It listens on
    /// almost all Channels in the system and can send messages to any
    /// sink.
    pub fn start(mut self) {
        thread::spawn(move || {
            // There is currently no method of restarting each of the
            // threads, all of them simply consist of a while true
            // loop listening on a number of sources.
            loop {
                // 1. Listen on Interface Channel.
                if let Some(action) = self.interface.try_recv() {
                    match action {
                        InterfaceAction::Shutdown => {
                            log::info!("received shutdown command, terminating Switch.");
                            break;
                        }
                        InterfaceAction::Message(transaction) => {}
                        InterfaceAction::Subscribe(addr) => {}
                    }
                }
                // 2. Listen on Signaling Channel.
                // 3. Listen on Handler Channel.
            }
        });
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
