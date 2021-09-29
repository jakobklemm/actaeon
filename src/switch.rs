//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::node::Center;
use crate::tcp::Handler;
use crate::transaction::Transaction;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Each message being sent between the Listener Thead and User Thread
/// is either a UserAction or a SwitchAction, only UserAction get
/// actually passed onto the user, others are internal types for
/// managing the thread and the channel.
pub enum SwitchCommand {
    /// mpsc element for internal communication, like shutting down
    /// the thread or any issues.
    SwitchAction(SwitchAction),
    /// Messages coming from the network intended for the user.
    UserAction(Transaction),
}

/// A collection of possible events for the channel. These are not
/// error messages, since they should be handled automatically, but in
/// the future this part of the system should be made more open by
/// using traits and giving the user the option to specify how to
/// handle all kinds of issues and messages.
pub enum SwitchAction {
    /// If the network connection has been terminated or failed. This
    /// only referres to the network connection, not the channel.
    Terminated,
    /// A heads up message should the cache be full. The system should
    /// handle this automatically, but it can be helpful to be
    /// informed.
    CacheFull,
}

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// The SwitchInterface will implement recv/0 and try_recv/0
    /// functions, which will internally receive messages from the
    /// channel.
    receiver: Receiver<SwitchCommand>,
    /// Other direction of communication
    sender: Sender<SwitchCommand>,
}

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    /// mpsc sender component to send incoming messages to the user.
    /// This will only be done for messages that are intended for the
    /// user, not forwarded messages in the kademlia system.
    sender: Sender<SwitchCommand>,
    /// Messages from the user
    receiver: Receiver<SwitchCommand>,
    /// New transactions that are intended for the user will be
    /// checked against the cache to see if they are duplicates.
    /// TODO: Define term for "messages intended for the user"
    cache: Cache,
    /// TODO
    handler: Handler,
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

impl Interface {
    pub fn try_recv(&self) -> Option<Transaction> {
        match self.receiver.try_recv() {
            Ok(action) => match action {
                SwitchCommand::SwitchAction(_s) => {
                    // TODO: Handle SwitchActions, allow for custom user behaviour
                    None
                }
                SwitchCommand::UserAction(t) => Some(t),
            },
            Err(_) => None,
        }
    }

    pub fn recv(&self) -> Option<Transaction> {
        match self.receiver.recv() {
            Ok(action) => match action {
                SwitchCommand::SwitchAction(_s) => {
                    // TODO: Handle SwitchActions, allow for custom user behaviour
                    None
                }
                SwitchCommand::UserAction(t) => Some(t),
            },
            Err(_) => None,
        }
    }

    pub fn send(&self) {}
}

impl Switch {
    /// Creates a new (Switch, Interface) combo, creating the Cache
    /// and staritng the channel.
    fn new(center: Center, limit: usize) -> Result<(Switch, Interface), Error> {
        let (sender1, receiver1) = mpsc::channel();
        let (sender2, receiver2) = mpsc::channel();
        let cache = Cache::new(limit);
        let handler = Handler::new(center)?;
        let switch = Switch {
            sender: sender1,
            receiver: receiver2,
            cache,
            handler,
        };
        let interface = Interface {
            sender: sender2,
            receiver: receiver1,
        };
        Ok((switch, interface))
    }

    /// TODO: Add tracing for invalid messages and sending errors that
    /// can't be returned.
    fn start(mut self) -> Result<(), Error> {
        thread::spawn(move || loop {
            match self.handler.read() {
                Some(wire) => match wire.convert() {
                    Ok(t) => {
                        let message = SwitchCommand::UserAction(t);
                        let _ = self.sender.send(message);
                    }
                    Err(_) => {
                        continue;
                    }
                },
                None => continue,
            }
            // Send message
            match self.receiver.try_recv() {
                Ok(_b) => {

                    continue;
                }
                Err(e) => {
                    println!("{:?}", e);
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
    pub fn new(limit: usize) -> Self {
        Self {
            elements: Vec::new(),
            limit,
        }
    }

    /// Takes a mutable reference to the cache and sorts the elements.
    /// Transaction implements Ord based on the "created" timestamp,
    /// which is used to sort the cache.
    pub fn sort(&mut self) {
        self.elements.sort()
    }

    /// Adds a new element to the cache. If the cache is full the
    /// oldest element will get removed and the new element gets
    /// added.
    pub fn add(&mut self, element: Transaction) {
        self.elements.push(element);
        self.sort();
        self.elements.truncate(self.limit);
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
