//! # Signaling
//!
//! Responsible for Kademlia background tasks and bootstrapping the
//! Instance.

use crate::node::Address;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct Signaling {
    servers: Vec<String>,
    queue: ActionBucket,
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
}

impl Signaling {
    pub fn new(servers: Vec<String>) -> Self {
        Self {
            servers,
            queue: ActionBucket::new(),
        }
    }

    pub fn start(self) {
        thread::spawn(move || {
            // TODO: Bootstrapping
            loop {
                // TODO: Queue processing
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
