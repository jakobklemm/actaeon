//! # Actaeon
//!
//! Distributed PubSub and messaging protocol for decentralized real time
//! applications.
//!
//! The most important struct for the API is the `Interface`, which can be
//! used for sending and receiving messages as well as subscribing to a
//! `Topic`. From there most interactions with other users can be done
//! through a `Topic`.
//!
//! Example:
//! ``` rust
//! use actaeon::{
//!     config::Config,
//!     node::{Center, ToAddress},
//!     Interface,
//! };
//! use sodiumoxide::crypto::box_;
//!
//! fn main() {
//!     let config = Config::new(20, 1, 100, "example.com".to_string(), 4242);
//!     let (_, secret) = box_::gen_keypair();
//!     let center = Center::new(secret, String::from("127.0.0.1"), 1235);
//!
//!     let interface = Interface::new(config, center).unwrap();
//!
//!     let mut topic = interface.subscribe(&"example".to_string().to_address());
//!
//!     let _ = topic.broadcast("hello world".as_bytes().to_vec());
//! }
//! ```

pub mod bucket;
pub mod config;
pub mod error;
pub mod handler;
pub mod message;
pub mod node;
pub mod record;
pub mod router;
pub mod signaling;
pub mod switch;
pub mod topic;
pub mod transaction;
pub mod util;

use config::Config;
use config::Signaling as CSig;
use error::Error;
use handler::Listener;
use message::Message;
use node::Address;
pub use node::{Center, ToAddress};
use record::RecordBucket;
use router::Safe;
use signaling::Signaling;
use switch::Switch;
use topic::Simple;
pub use topic::Topic;
use transaction::Class;
pub use transaction::Transaction;
use util::Channel;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// Center used for getting message origins. Is currently public
    /// to allow the user to read the center Address.
    pub center: Center,
    /// Channel to communicate with the Switch. The Interface is only
    /// connected with the Switch and none of the other threads, even
    /// though it starts them.
    switch: Channel<InterfaceAction>,
}

/// Each module that wants to interact with the Switch has a custom
/// enum of possible cases. This is to avoid having to handle a lot of
/// impossible cases in the Switch loop.
pub enum InterfaceAction {
    /// Will shut down the Switch thread. (But nothing else).
    Shutdown,
    /// Send a complete Transaction to the Switch (and to the
    /// TcpHandler from there). The restrictions and rules described
    /// in the send function apply.
    Message(Transaction),
    /// Passes a new Simple (minified version of the Topic) to the
    /// Switch, from where the Subscribe info will be distributed
    /// through the system.
    Subscribe(Simple),
}

impl Interface {
    /// Currently there are no dedicated functions for creating and
    /// starting the components. Instead this function does both:
    ///
    /// - It creates all the internally shared components like the
    /// RecordBucket and the Table.
    ///
    /// - It creates all the thread objects required.
    ///
    /// - It starts all threads.
    ///
    /// Should any of the steps fail the entire function fails, which
    /// means the system is unable to start.
    pub fn new(config: Config, center: Center) -> Result<Self, Error> {
        // initialize
        let bucket = RecordBucket::new();
        let (switch1, switch2) = Channel::<InterfaceAction>::new();
        let (listener1, listener2) = Channel::<Transaction>::new();
        let (signaling1, signaling2) = Channel::<signaling::SignalingAction>::new();
        let table = Safe::new(config.replication, center.clone());
        let signaling = CSig::new(config.signaling, config.port);
        let listener = Listener::new(
            center.clone(),
            listener1,
            config.replication,
            table.clone(),
            signaling,
        )?;
        let switch = Switch::new(
            listener2,
            switch1,
            signaling1,
            center.clone(),
            table.clone(),
            bucket.clone(),
        )?;
        let signaling = Signaling::new(signaling2, table.clone());

        // startup
        listener.start();
        switch.start();
        //signaling.start();

        // return
        Ok(Self {
            center,
            switch: switch2,
        })
    }

    /// Creates a new Topic, both locally, on the Switch thread and
    /// (possilby) remotely. The local topic returned contains a list
    /// of subscribers (that will get updated and refreshed on demand)
    /// as well as a Channel to the Switch. From there any updates are
    /// processed.
    pub fn subscribe(self, addr: &Address) -> Topic {
        let (c1, c2) = Channel::new();
        let local = Topic::new(addr.clone(), c1, Vec::new(), self.center.public.clone());
        let remote = Simple::new(addr.clone(), c2);
        let _ = self.switch.send(InterfaceAction::Subscribe(remote));
        local
    }

    /// It is possible to ignore the entire PubSub architecture and
    /// just send messages to another user directly. For that the
    /// exact Address has to be known. From there a Transaction can be
    /// constructed and distributed through the system. This function
    /// is only recommended for specific, special reasons, otherwise
    /// the `message` function can be used.
    pub fn send(&self, transaction: Transaction) -> Result<(), Error> {
        let action = InterfaceAction::Message(transaction);
        self.switch.send(action)
    }

    /// Tries to read a message from the Interface Channel without
    /// blocking. It only returns a transaction if a Message event was
    /// received, any other type will be ignored.
    pub fn try_recv(&self) -> Option<Transaction> {
        if let Some(action) = self.switch.try_recv() {
            match action {
                InterfaceAction::Message(t) => Some(t),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Mostly the same as try_recv(), but it blocks until a Message
    /// event is available. Should it ever return None it is likely,
    /// that the Switch is no longer available.
    pub fn recv(&self) -> Option<Transaction> {
        loop {
            if let Some(action) = self.switch.recv() {
                match action {
                    InterfaceAction::Message(t) => {
                        return Some(t);
                    }
                    _ => {
                        continue;
                    }
                }
            } else {
                return None;
            }
        }
    }

    /// Constructs a new Transaction from the provided target and body
    /// and completes the missing values. The created Transaction will
    /// be distributed automatically.
    pub fn message(&self, target: Address, body: Vec<u8>) -> Result<(), Error> {
        let message = Message::new(
            Class::Action,
            self.center.public.clone(),
            target,
            Address::default(),
            body,
        );
        let action = InterfaceAction::Message(Transaction::new(message));
        self.switch.send(action)
    }
}
