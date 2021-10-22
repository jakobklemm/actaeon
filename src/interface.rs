//! # Interface

use crate::config::Config;
use crate::error::Error;
use crate::node::Address;
use crate::node::Center;
use crate::signaling::{ActionBucket, Signaling};
use crate::switch::Channel;
use crate::switch::{Command, SwitchAction, SystemAction};
use crate::topic::Topic;
use crate::transaction::Transaction;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// The SwitchInterface will implement recv/0 and try_recv/0
    /// functions, which will internally receive messages from the
    /// channel.
    channel: Channel,
    signaling: Signaling,
    /// Center used for getting message origins.
    pub center: Center,
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(channel: Channel, config: Config, center: Center, queue: ActionBucket) -> Self {
        Self {
            channel,
            center,
            signaling: Signaling::new(config.signaling, queue),
        }
    }

    pub fn try_recv(&self) -> Option<Transaction> {
        // TODO: Handle action message types
        match self.channel.try_recv() {
            Some(s) => match s {
                Command::User(t) => Some(t),
                Command::Switch(a) => {
                    log::info!("special switch action received: {:?}", a);
                    None
                }
                Command::System(_) => {
                    log::warn!("unknown action received");
                    None
                }
            },
            None => None,
        }
    }

    pub fn recv(&self) -> Option<Transaction> {
        // TODO: Handle action message types
        match self.channel.recv() {
            Some(s) => match s {
                Command::User(t) => Some(t),
                Command::Switch(a) => {
                    log::info!("special switch action received: {:?}", a);
                    None
                }
                Command::System(_) => {
                    log::warn!("unknown action received");
                    None
                }
            },
            None => None,
        }
    }

    /// Unless there is a good reason to do otherwise, the Message
    /// Class should always be Record.
    pub fn send(&self, transaction: Transaction) -> Result<(), Error> {
        self.channel.send(Command::User(transaction))
    }

    pub fn subscribe(&self, address: Address) -> Result<Topic, Error> {
        let (c1, c2) = Channel::new();
        let local = Topic::new(address.clone(), c1, Vec::new());
        let remote = Topic::new(address.clone(), c2, Vec::new());
        let e = self
            .channel
            .send(Command::System(SystemAction::Subscribe(remote)));

        if e.is_err() {
            log::error!("handler thread failed: {:?}", e.unwrap());
            return Err(Error::Connection(String::from(
                "handler thread not responding",
            )));
        } else {
            return Ok(local);
        }
    }

    pub fn terminate(&self) {
        let e = self.channel.send(Command::Switch(SwitchAction::Terminate));
        if e.is_err() {
            log::error!("error terminating thread: {:?}", e);
        }
    }

    pub fn bootstrap(&self) {
        self.signaling.clone().start()
    }
}
