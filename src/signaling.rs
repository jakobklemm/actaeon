//! # Signaling
//!
//! Responsible for Kademlia background tasks and bootstrapping the
//! Instance.

use crate::config::Config;
use crate::message::Message;
use crate::node::Address;
use crate::transaction::{Class, Transaction};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

pub struct Signaling {
    server: String,
    port: usize,
    pub queue: ActionBucket,
    last: SystemTime,
}

#[derive(Eq, PartialEq)]
pub struct Action {
    created: SystemTime,
    last: SystemTime,
    action: Type,
    target: Address,
    uuid: Uuid,
}

#[derive(Clone)]
pub struct ActionBucket {
    actions: Arc<Mutex<Vec<Action>>>,
}

#[derive(Eq, PartialEq)]
pub enum Type {
    Ping,
    Lookup,
}

impl Signaling {
    pub fn new(config: Config, queue: ActionBucket) -> Self {
        Self {
            server: config.signaling,
            port: config.port,
            queue,
            last: SystemTime::now(),
        }
    }

    pub fn start(mut self) {
        thread::spawn(move || {
            // TODO: Bootstrapping
            loop {
                if self.queue.len() == 0 {
                    // Do a action every two minutes.
                    if self.last.elapsed().unwrap() >= Duration::new(120, 0) {
                        self.last = SystemTime::now();
                        // select a random node, get the closes few to it,
                        // look for Link None nodes and perform a lookup
                        // on them.
                    }
                }
            }
        });
    }
}

impl Action {
    pub fn new(action: Type, target: Address) -> Self {
        Self {
            created: SystemTime::now(),
            last: SystemTime::now(),
            action,
            target,
            uuid: Uuid::new_v4(),
        }
    }

    // Shorthand function for creating a lookup Action.
    pub fn lookup(target: Address) -> Self {
        Self {
            created: SystemTime::now(),
            last: SystemTime::now(),
            action: Type::Lookup,
            target,
            uuid: Uuid::new_v4(),
        }
    }
}

impl ActionBucket {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get(&self) {
        match self.actions.lock() {
            Ok(mut actions) => {
                actions.sort();
            }
            Err(e) => {
                log::error!("one of the threads might have crashed: {}", e);
            }
        }
    }

    pub fn add(&mut self, action: Action) {
        if let Ok(actions) = self.actions.lock() {
            let index = actions.iter().position(|e| e.uuid == action.uuid);
            if index.is_none() {
                self.actions.lock().unwrap().push(action)
            }
        } else {
            log::error!("one of the threads might have crashed.");
        }
    }

    pub fn sort(&mut self) {
        match self.actions.lock() {
            Ok(mut actions) => {
                actions.sort();
            }
            Err(e) => {
                log::error!("one of the threads might have crashed: {:?}", e);
            }
        }
    }

    pub fn remove(&mut self, uuid: Uuid) {
        if let Ok(mut actions) = self.actions.lock() {
            let index = actions.iter().position(|e| e.uuid == uuid);
            if index.is_none() {
                actions.remove(index.unwrap());
            }
        } else {
            log::error!("one of the threads might have crashed.");
        }
    }

    pub fn len(&self) -> usize {
        if let Ok(actions) = self.actions.lock() {
            actions.len()
        } else {
            0
            //log::error!("one of the threads might have crashed.");
        }
    }
}

impl PartialOrd for Action {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.last.cmp(&other.last))
    }
}

impl Ord for Action {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last.cmp(&other.last)
    }
}
