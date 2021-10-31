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

pub use config::Config;
use config::Signaling as CSig;
pub use error::Error;
use handler::Listener;
use message::Message;
pub use node::{Address, Center, Node};
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
    /// Center used for getting message origins.
    pub center: Center,
    switch: Channel<InterfaceAction>,
}

pub enum InterfaceAction {
    Shutdown,
    Message(Transaction),
    Subscribe(Simple),
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(config: Config, center: Center) -> Result<Self, Error> {
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
        signaling.start();

        // return
        Ok(Self {
            center,
            switch: switch2,
        })
    }

    pub fn subscribe(self, addr: &Address) -> Topic {
        let (c1, c2) = Channel::new();
        let local = Topic::new(addr.clone(), c1, Vec::new());
        let remote = Simple::new(addr.clone(), c2);
        let _ = self.switch.send(InterfaceAction::Subscribe(remote));
        local
    }

    pub fn send(&self, transaction: Transaction) -> Result<(), Error> {
        let action = InterfaceAction::Message(transaction);
        self.switch.send(action)
    }

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

    pub fn recv(&self) -> Option<Transaction> {
        if let Some(action) = self.switch.recv() {
            match action {
                InterfaceAction::Message(t) => Some(t),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn message(&self, target: Address, body: Vec<u8>) -> Result<(), Error> {
        let message = Message::new(
            Class::Action,
            self.center.public.clone(),
            target,
            Address::default(),
            body,
        );
        let t = Transaction::new(message);
        let action = InterfaceAction::Message(t);
        self.switch.send(action)
    }
}
