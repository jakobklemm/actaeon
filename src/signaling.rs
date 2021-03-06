//! # Signaling
//!
//! Responsible for Kademlia background tasks and bootstrapping the
//! Instance.

use crate::message::Message;
use crate::node::Address;
use crate::router::Safe;
use crate::transaction::{Class, Transaction};
use crate::util::Channel;
use std::cell::RefCell;
use std::thread;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Represents all the fields needed to run the Signaling thread.
pub struct Signaling {
    /// Connection to the Switch.
    channel: Channel<SignalingAction>,
    /// Time of the last action.
    last: SystemTime,
    /// The routing table to update the statup of Nodes.
    table: Safe,
    /// List of active actions.
    bucket: RefCell<ActionBucket>,
}

/// Instead of storing transactions this thread uses a custom type to
/// represent pending / ongoing operations.
#[derive(Eq, PartialEq, Clone)]
pub struct SignalingAction {
    /// What type of operation is ongoing.
    pub action: Type,
    /// What node was asked.
    pub target: Address,
    /// The uuid of the transaction.
    pub uuid: Uuid,
}

/// Collection of ongoing operations.
pub struct ActionBucket {
    actions: Vec<SignalingAction>,
}

/// Not the same as the Transaction Class, since not all Classes are
/// possible for Signaling actions.
#[derive(Eq, PartialEq, Clone)]
pub enum Type {
    Ping,
    Pong,
    Lookup,
    Details,
}

impl Signaling {
    /// Creates a new Signaling object without starting the thread.
    pub fn new(channel: Channel<SignalingAction>, table: Safe) -> Self {
        Self {
            channel,
            last: SystemTime::now(),
            table,
            bucket: RefCell::new(ActionBucket::new()),
        }
    }

    /// Starts the signaling thread.
    pub fn start(mut self) {
        thread::spawn(move || {
            loop {
                // 1. Try to read from Channel for new Actions.
                if let Some(action) = self.channel.try_recv() {
                    match action.action {
                        Type::Ping => {
                            // Unable to handle
                        }
                        Type::Pong => {
                            self.table.status(&action.target, true);
                            self.bucket.borrow_mut().remove(action.uuid);
                        }
                        Type::Lookup => {
                            self.bucket.borrow_mut().add(action);
                        }
                        Type::Details => {
                            // TODO: Add lookup result to RT
                            self.bucket.borrow_mut().remove(action.uuid);
                        }
                    }
                }

                // 2. Process an item from the Bucket.
                if self.last.elapsed().unwrap() >= Duration::new(60, 0) {
                    self.last = SystemTime::now();
                    if let Some(action) = self.bucket.borrow().get() {
                        let _ = self.channel.send(action.clone());
                    }
                }

                if self.bucket.borrow().len() == 0 {
                    let action = SignalingAction::new(Type::Lookup, Address::random());
                    self.bucket.borrow_mut().add(action);
                }
            }
        });
    }
}

impl SignalingAction {
    pub fn new(action: Type, target: Address) -> Self {
        Self {
            action,
            target,
            uuid: Uuid::new_v4(),
        }
    }

    /// Shorthand for creating a new Pong response.
    pub fn pong(address: Address, uuid: Uuid) -> Self {
        Self {
            action: Type::Pong,
            // Target is irrelevant, only the UUID matters.
            target: address,
            uuid,
        }
    }

    /// Shorthand for creating a new Details response.
    pub fn details(address: Address, uuid: Uuid) -> Self {
        Self {
            action: Type::Details,
            // Target is irrelevant, only the UUID matters.
            target: address,
            uuid,
        }
    }

    // Shorthand function for creating a lookup Action.
    pub fn lookup(target: Address) -> Self {
        Self {
            action: Type::Lookup,
            target,
            uuid: Uuid::new_v4(),
        }
    }

    /// Converts a SignalingAction into a Transaction.
    pub fn to_transaction(&self, center: &Address) -> Transaction {
        let class = match self.action {
            Type::Lookup => Class::Lookup,
            Type::Details => Class::Details,
            Type::Ping => Class::Ping,
            Type::Pong => Class::Pong,
        };
        let body = Vec::new();
        Transaction::new(Message::new(
            class,
            center.clone(),
            self.target.clone(),
            Address::default(),
            body,
        ))
    }
}

impl ActionBucket {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn get(&self) -> Option<&SignalingAction> {
        self.actions.first()
    }

    pub fn add(&mut self, action: SignalingAction) {
        let index = self.actions.iter().position(|e| e.uuid == action.uuid);
        if index.is_none() {
            self.actions.push(action)
        }
    }

    pub fn remove(&mut self, uuid: Uuid) {
        let index = self.actions.iter().position(|e| e.uuid == uuid);
        if index.is_none() {
            self.actions.remove(index.unwrap());
        }
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
}
