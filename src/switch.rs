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
use crate::node::{Address, Center, Link, Node};
use crate::router::Table;
use crate::signaling::{Action, ActionBucket};
use crate::topic::Topic;
use crate::topic::{Record, RecordBucket, TopicBucket};
use crate::transaction::{Class, Transaction};
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
    /// mpsc sender component to send incoming messages to the user.
    /// This will only be done for messages that are intended for the
    /// user, not forwarded messages in the Kademlia system. Since
    /// both directions are needed, the two channels are abstracted
    /// through a dedicated object, which handles both.
    channel: Channel<Command>,
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

impl Switch {
    /// Creates a new (Switch, Interface) combo, creating the Cache
    /// and staritng the channel.
    pub fn new(center: Center, config: Config, limit: usize) -> Result<(Switch, Interface), Error> {
        let (c1, c2) = Channel::<Command>::new();
        let cache = Cache::new(limit);
        let queue = ActionBucket::new();
        let handler = Handler::new(center.clone())?;
        let switch = Switch {
            channel: c1,
            cache: RefCell::new(cache),
            handler: handler,
            replication: config.replication,
            table: RefCell::new(Table::new(limit, center.clone())),
            topics: RefCell::new(TopicBucket::new(center.clone())),
            records: RefCell::new(RecordBucket::new(center.clone())),
            center: center.clone(),
            queue: queue.clone(),
        };
        let interface = Interface::new(config, center).unwrap();
        Ok((switch, interface))
    }

    /// The switch is responsible for deciding where specific messages
    /// go based on their origin, target and type. It listens on
    /// almost all Channels in the system and can send messages to any
    /// sink.
    pub fn start(mut self) {
        thread::spawn(move || loop {});
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
    use crate::node::Link;

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

    #[test]
    fn test_parse_bulk() {
        let node = Node::new(gen_center().public.clone(), Some(gen_center().link));
        let mut node_bytes = Vec::new();
        vec![gen_node("abc"), gen_node("def")].iter().for_each(|x| {
            node_bytes.append(&mut x.address.as_bytes().to_vec());
        });

        let mut bytes = Vec::new();
        bytes.append(&mut node.as_bytes());
        bytes.append(&mut node_bytes);

        let mut center_address = [0; 32];
        let mut center_link_length: u8 = 0;
        let mut center_link: Vec<u8> = vec![center_link_length];
        let mut table: Vec<u8> = Vec::new();
        if bytes.len() <= 32 {
            log::warn!("received invalid Bulk data: {:?}", bytes);
        }

        for (i, j) in bytes.iter().enumerate() {
            if i <= 31 {
                center_address[i] = *j;
            } else if i == 32 {
                center_link_length = *j;
            } else if i > 32 && i < (32 + center_link_length).into() {
                center_link.push(*j);
            } else {
                table.push(*j);
            }
        }

        let center_address_parsed = Address::from_bytes(center_address).unwrap();
        let center_link_parsed = Link::from_bytes(center_link).unwrap();
        let nodes: Vec<Address> = table
            .chunks_exact(32)
            .map(|x| Address::from_slice(x).unwrap())
            .collect();

        assert_eq!(
            bytes.chunks_exact(32).next().unwrap(),
            gen_center().public.as_bytes().to_vec()
        );
        //assert_eq!(bytes[32] + 1, gen_center().link.as_bytes().len() as u8);
        assert_eq!(center_address_parsed, gen_center().public);
        assert_eq!(center_link_parsed, gen_center().link);
        assert_eq!(nodes.first().unwrap().clone(), gen_node("abc").address);
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

    fn gen_node(s: &str) -> Node {
        Node::new(Address::generate(s).unwrap(), None)
    }

    use crate::node::Center;
    use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

    fn gen_center() -> Center {
        let mut b = [0; 32];
        b[0] = 42;
        let s = SecretKey::from_slice(&b).unwrap();
        Center::new(s, String::from(""), 8080)
    }
}
