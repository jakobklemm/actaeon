//! # Switch

pub mod adapter;
use self::adapter::Mode;
use crate::error::Error;
use crate::transaction::Transaction;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Each message being sent between the Listener Thead and User Thread
/// is either a UserAction or a SwitchAction, only UserAction get
/// actually passed onto the user, others are internal types for
/// managing the thread and the channel.
pub enum Command {
    /// mpsc element for internal communication, like shutting down
    /// the thread or any issues.
    SwitchAction(Action),
    /// Messages coming from the network intended for the user.
    UserAction(Transaction),
}

/// A collection of possible events for the channel. These are not
/// error messages, since they should be handled automatically, but in
/// the future this part of the system should be made more open by
/// using traits and giving the user the option to specify how to
/// handle all kinds of issues and messages.
pub enum Action {
    /// If the network connection has been terminated or failed. This
    /// only referres to the network connection, not the channel.
    Terminated,
    /// A heads up message should the cache be full. The system should
    /// handle this automatically, but it can be helpful to be
    /// informed.
    CacheFull,
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

/// Starting the switch will create both SwitchInterface and Switch
/// objects. The SwitchInterface will be passed up and to the user /
/// instance. From there the user can interact (receive messages) with
/// the listener. Currently the communication only works in one
/// direction, sending messages will currently not utilize the
/// dedicated thread. In the future this will get integrated into the
/// thread and also include a two-directional channel system.
pub struct SwitchInterface {
    /// The SwitchInterface will implement recv/0 and try_recv/0
    /// functions, which will internally receive messages from the
    /// channel.
    channel: Receiver<Command>,
}

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    /// mpsc sender component to send incoming messages to the user.
    /// This will only be done for messages that are intended for the
    /// user, not forwarded messages in the kademlia system.
    channel: Sender<Command>,
    /// New transactions that are intended for the user will be
    /// checked against the cache to see if they are duplicates.
    /// TODO: Define term for "messages intended for the user"
    recent: Cache,
    /// Since the system is suppose to be usable with different
    /// transport layers and protocols a trait object of Adapter is
    /// used to represent any adapter. By default the system will use
    /// a TcpListener.
    handler: Box<dyn adapter::Adapter + Send>,
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

impl Switch {
    /// Creates a new pair of Switch and SwitchInterface and returns
    /// them both. They need to be created at the same time, since
    /// they both require access to the same channel. There is no
    /// dedicated SwitchInterface::new, the interface has to be
    /// created through this funciton.
    pub fn new(
        limit: usize,
        adapter: Box<dyn adapter::Adapter + Send>,
    ) -> (Switch, SwitchInterface) {
        let (tx, rx) = mpsc::channel::<Command>();
        let interface = SwitchInterface { channel: rx };
        let cache = Cache::new(limit);
        let switch = Switch {
            recent: cache,
            handler: adapter,
            channel: tx,
        };
        (switch, interface)
    }

    /// Currently the behavior of this function is hard coded, in the
    /// future it should be possible to opt out of starting the thread
    /// or use a pool instead. This function will start a new thread
    /// and run the start function of the adapter there. It will also
    /// pass the entire object to the thread. TODO: Determine shutdown
    /// method and what to do with handle.
    pub fn start(mut self) -> Result<u8, Error> {
        todo!()
        // thread::spawn(move || {
        //     if let Ok(()) = self.handler.start() {
        //         let _ = self.handler.mode(Mode::Unblocking);
        //         loop {
        //             let wire = self.handler.accept().unwrap();
        //             self.recent.add(wire.convert());
        //             // TODO: Kademlia processing
        //         }
        //     } else {
        //         return Err(Error::System);
        //     }
        // });
        // Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::router::address::Address;
    use crate::transaction::{Class, Message};

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
            Address::new("abc"),
            Address::new("def"),
            "test".to_string().as_bytes().to_vec(),
        );
        let t = Transaction::new(message);
        return t;
    }
}
