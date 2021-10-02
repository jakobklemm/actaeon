//! # Interface

use crate::error::Error;
use crate::message::Message;
use crate::node::Address;
use crate::node::Center;
use crate::switch::SwitchCommand;
use crate::topic::Topic;
use crate::transaction::Class;
use crate::transaction::Transaction;
use std::sync::mpsc::{Receiver, Sender};

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
    /// Center used for getting message origins.
    center: Center,
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

    pub fn send(&self, m: Message) -> Result<(), Error> {
        let transaction = Transaction::new(m);
        self.sender.send(SwitchCommand::UserAction(transaction))?;
        Ok(())
    }

    pub fn subscribe(&self, address: Address) -> Result<Topic, Error> {
        let message = Message::new(Class::Subscribe, self.center.public, address, Vec::new());
        let transaction = Transaction::new(message);
        self.sender.send(SwitchCommand::UserAction(transaction))?;
        Ok(Topic::new(address))
    }
}
