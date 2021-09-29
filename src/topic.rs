//! # Topics
//!
//! The main component of the PubSub system. Topics can be used in two
//! ways: They can be stored on their designated target node in the
//! local database or they can represent that remote object for a user
//! to interact with. Each topic has (like most other elements of the
//! system) an Address, which can be generated randomly or from the
//! user.

use crate::error::Error;
use crate::node::{Address, ToAddress};
use std::time::SystemTime;

pub struct DataTopic {
    address: Address,
    subscribers: Vec<Address>,
    timestamp: SystemTime,
}

pub struct Topic {
    address: Address,
}

impl DataTopic {
    fn new(address: Address) -> Self {
        Self {
            address,
            subscribers: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }
}

impl Topic {
    pub fn new(address: Box<dyn ToAddress>) -> Result<Self, Error> {
        let address = address.to_address()?;
        Ok(Self { address })
    }
}
