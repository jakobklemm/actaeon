//! # Records
//!
//! Represent a PubSub Topic this Node is responsible for. Currently
//! this thread only has a common hashmap impl., in the future this
//! will have to be extended with a dedicated thread and a file system
//! interaction.

use crate::node::Address;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents a Topic the Center Node is responsible for. The fields
/// are basically identical to a normal Topic but the Records aren't
/// meant to communicate with anybody directly. (comparable to an IPFS
/// provider record.)
#[derive(Clone)]
pub struct Record {
    /// The Address of the Record, that should satisfy
    /// "should_be_local" with the current RT. At the moment there is
    /// no system of republishing them or invalidating them later
    /// since this implementation is (right now at least) only
    /// suitable for small clusters.
    pub address: Address,
    /// List of subscribers as a Vec of Addresses. The actual Link
    /// data will be fetched from the RT or messages will be
    /// distributed indirectly.
    pub subscribers: Vec<Address>,
}

/// Multi "threadable" collection of all locally registered Records.
/// TODO: Check if it has to be thread safe.
#[derive(Clone)]
pub struct RecordBucket(Arc<Mutex<HashMap<Address, Record>>>);

impl Record {
    /// Creates a new Record without subscribers.
    pub fn new(address: Address) -> Self {
        Self {
            address,
            subscribers: Vec::new(),
        }
    }

    /// Adds the provided Address to the list of subscribers.
    pub fn subscribe(&mut self, subscriber: Address) {
        if !self.contains(&subscriber) {
            self.subscribers.push(subscriber)
        }
    }

    /// Removes the provided Address to the list of subscribers.
    pub fn unsubscribe(&mut self, subscriber: &Address) {
        let index = self.subscribers.iter().position(|e| e == subscriber);
        match index {
            Some(i) => {
                self.subscribers.remove(i);
            }
            None => {}
        }
    }

    /// Checks if the Address (the Subscriber) exists in the list of
    /// subscribers.
    pub fn contains(&self, query: &Address) -> bool {
        self.subscribers.contains(query)
    }
}

impl RecordBucket {
    /// Creates a new RecordBucket. It contains thread safety and a
    /// Mutex, so it doesn't have to be wrappen again.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Adds a new record to the Bucket. An internal thread error will
    /// result in a panic as there is currently no proper method of
    /// globally restarting the core threads.
    pub fn add(&self, record: Record) {
        match self.0.lock() {
            Ok(mut records) => {
                records.insert(record.address.clone(), record);
            }
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread has encountered an error: {}",
                    e
                );
            }
        }
    }

    /// Removes a record to the Bucket. An internal thread error will
    /// result in a panic as there is currently no proper method of
    /// globally restarting the core threads.
    pub fn remove(&self, address: &Address) {
        match self.0.lock() {
            Ok(mut records) => {
                records.remove(address);
            }
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread has encountered an error: {}",
                    e
                );
            }
        }
    }

    /// Checks if a Record exists in the RecordBucket and returns a
    /// boolean. An internal thread error will
    /// result in a panic as there is currently no proper method of
    /// globally restarting the core threads.
    pub fn contains(&self, address: &Address) -> bool {
        match self.0.lock() {
            Ok(records) => records.contains_key(address),
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread
        has encountered an error: {}",
                    e
                );
                false
            }
        }
    }

    /// Returns a copy of the Record if it exists. An internal thread
    /// error will result in a panic as there is currently no proper
    /// method of globally restarting the core threads.
    pub fn get(&self, address: &Address) -> Option<Record> {
        match self.0.lock() {
            Ok(records) => match records.get(address) {
                Some(record) => Some(record.clone()),
                None => None,
            },
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread has encountered an error: {}",
                    e
                );
                None
            }
        }
    }

    /// Since getting a mutable reference to the Record isn't possible
    /// outside the lock, direct functions on the RecordBucket can be
    /// used. They take in the Address of the Record as their first
    /// argument and the Address of the new Subscriber as their
    /// second. An internal thread error will result in a panic as
    /// there is currently no proper method of globally restarting the
    /// core threads.
    pub fn subscribe(&self, record: &Address, subscriber: Address) {
        match self.0.lock() {
            Ok(mut records) => match (*records).get_mut(&record) {
                Some(record) => {
                    (*record).subscribe(subscriber);
                }
                None => {}
            },
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread has encountered an error: {}",
                    e
                );
            }
        }
    }

    /// Since getting a mutable reference to the Record isn't possible
    /// outside the lock, direct functions on the RecordBucket can be
    /// used. They take in the Address of the Record as their first
    /// argument and the Address of the new Subscriber as their
    /// second. An internal thread error will result in a panic as
    /// there is currently no proper method of globally restarting the
    /// core threads.
    pub fn unsubscribe(&self, record: &Address, subscriber: &Address) {
        match self.0.lock() {
            Ok(mut records) => match (*records).get_mut(&record) {
                Some(record) => {
                    (*record).unsubscribe(subscriber);
                }
                None => {}
            },
            Err(e) => {
                log::warn!(
                    "unable to lock thread, another thread has encountered an error: {}",
                    e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_empty() {
        let bucket = RecordBucket::new();
        let query = Address::random();
        assert_eq!(bucket.get(&query).is_none(), true);
    }

    #[test]
    fn test_bucket_add() {
        let bucket = RecordBucket::new();
        let addr = Address::random();
        let record = Record::new(addr.clone());
        bucket.add(record);
        assert_eq!(bucket.contains(&addr), true);
        assert_eq!(bucket.contains(&Address::random()), false);
    }

    #[test]
    fn test_bucket_subscribe() {
        let bucket = RecordBucket::new();
        let record_addr = Address::random();
        let record = Record::new(record_addr.clone());
        bucket.add(record);
        let subscriber = Address::random();
        bucket.subscribe(&record_addr, subscriber.clone());
        let record = bucket.get(&record_addr);
        assert_eq!(record.is_none(), false);
        assert_eq!(record.unwrap().contains(&subscriber), true);
    }

    #[test]
    fn test_bucket_unsubscribe() {
        let bucket = RecordBucket::new();
        let record_addr = Address::random();
        let record = Record::new(record_addr.clone());
        bucket.add(record);
        let subscriber = Address::random();
        bucket.subscribe(&record_addr, subscriber.clone());
        bucket.unsubscribe(&record_addr, &subscriber);
        let record = bucket.get(&record_addr);
        assert_eq!(record.unwrap().contains(&subscriber), false);
    }
}
