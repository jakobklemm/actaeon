//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::interface::Interface;
use crate::node::Center;
use crate::router::Table;
use crate::tcp::Handler;
use crate::transaction::Transaction;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
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
    /// only referres to the network connection, not the channel. It
    /// can also be used to signal termination to the thread.
    Terminate,
    /// A heads up message should the cache be full. The system should
    /// handle this automatically, but it can be helpful to be
    /// informed. If it gets sent by the user the cache in the thread
    /// will be cleared.
    Cache,
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
    /// Represents the TCP Handler, currently just one struct. Later
    /// this will be replaced by a trait Object.
    handler: Arc<Mutex<Handler>>,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: Table,
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
    pub fn new(center: Center, limit: usize) -> Result<(Switch, Interface), Error> {
        let (sender1, receiver1) = mpsc::channel();
        let (sender2, receiver2) = mpsc::channel();
        let cache = Cache::new(limit);
        let handler = Handler::new(center.clone())?;
        let switch = Switch {
            sender: sender1,
            receiver: receiver2,
            cache,
            handler: Arc::new(Mutex::new(handler)),
            table: Table::new(limit, center.clone()),
        };
        let interface = Interface {
            sender: sender2,
            receiver: receiver1,
            center: center,
        };
        Ok((switch, interface))
    }

    /// TODO: Add tracing for invalid messages and sending errors that
    /// can't be returned.
    pub fn start(mut self) -> Result<(), Error> {
        thread::spawn(move || loop {
            // tcp messages
            // TODO: handle errors
            match self.handler.lock().unwrap().read() {
                Some(wire) => match wire.convert() {
                    Ok(t) => {
                        self.cache.add(t.clone());
                        let message = SwitchCommand::UserAction(t);
                        let _ = self.sender.send(message);
                    }
                    Err(_) => {
                        continue;
                    }
                },
                None => continue,
            }

            // user messages
            match self.receiver.try_recv() {
                Ok(data) => {
                    match data {
                        SwitchCommand::UserAction(t) => {
                            let targets = self.table.get(&t.target(), 5);
                            // on tcp fail update the RT (and fail the sending?)
                            for i in targets {
                                // TODO: handle errors
                                let _ = self.handler.lock().unwrap().send(t.to_wire(), i);
                            }
                        }
                        SwitchCommand::SwitchAction(action) => match action {
                            SwitchAction::Terminate => {
                                break;
                            }
                            SwitchAction::Cache => {
                                self.cache.empty();
                            }
                        },
                    }
                }
                Err(_) => {
                    continue;
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

    fn empty(&mut self) {
        self.elements = Vec::new();
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
