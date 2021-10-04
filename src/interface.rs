//! # Interface

use crate::error::Error;
use crate::message::Message;
use crate::node::Address;
use crate::node::Center;
use crate::switch::Channel;
use crate::switch::{SwitchAction, SwitchCommand};
use crate::topic::Topic;
use crate::transaction::Class;
use crate::transaction::Transaction;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// The SwitchInterface will implement recv/0 and try_recv/0
    /// functions, which will internally receive messages from the
    /// channel.
    channel: Channel,
    /// Center used for getting message origins.
    center: Center,
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(channel: Channel, center: Center) -> Self {
        Self { channel, center }
    }

    pub fn try_recv(&self) -> Option<Transaction> {
        // TODO: Handle action message types
        match self.channel.try_recv() {
            Some(s) => match s {
                SwitchCommand::UserAction(t) => Some(t),
                SwitchCommand::SwitchAction(a) => {
                    log::info!("special switch action received: {:?}", a);
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
                SwitchCommand::UserAction(t) => Some(t),
                SwitchCommand::SwitchAction(a) => {
                    log::info!("special switch action received: {:?}", a);
                    None
                }
            },
            None => None,
        }
    }

    pub fn send(&self, m: Message) -> Result<(), Error> {
        let transaction = Transaction::new(m);
        self.channel.send(SwitchCommand::UserAction(transaction))?;
        Ok(())
    }

    pub fn subscribe(&self, address: Address) -> Result<Topic, Error> {
        let (c1, c2) = Channel::new();
        let local = Topic::new(address.clone(), c1);
        let remote = Topic::new(address.clone(), c2);
        match self
            .channel
            .send(SwitchCommand::SwitchAction(SwitchAction::Subscribe(remote)))
        {
            Ok(()) => {
                let message = Message::new(
                    Class::Subscribe,
                    self.center.public.clone(),
                    address,
                    Vec::new(),
                );
                let transaction = Transaction::new(message);

                let e = self.channel.send(SwitchCommand::UserAction(transaction));
                if e.is_err() {
                    log::error!("handler thread failed: {:?}", e);
                    return Err(Error::Connection(String::from(
                        "handler thread not responding",
                    )));
                }
            }
            Err(e) => {
                log::error!("handler thread failed: {:?}", e);
                return Err(Error::Connection(String::from(
                    "handler thread not responding",
                )));
            }
        }
        Ok(local)
    }

    pub fn terminate(&self) {
        let e = self
            .channel
            .send(SwitchCommand::SwitchAction(SwitchAction::Terminate));
        if e.is_err() {
            log::error!("error terminating thread: {:?}", e);
        }
    }
}
