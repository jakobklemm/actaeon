//! # Router Node
//!
//! A node has some basic elements used for routing and sorting.

use super::address::Address;
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

// Each node requires connection data as well as Kademlia Routing Data.
// It also stores the last seen time.
#[derive(Debug)]
pub struct Node<'a> {
    ip: &'a str,
    port: u16,
    pub address: Address<'a>,
    last: SystemTime,
}

impl<'a> Node<'a> {
    pub fn new(ip: &'a str, port: u16, address: Address<'a>) -> Self {
        Self {
            ip,
            port,
            address,
            last: SystemTime::now(),
        }
    }

    // Returns the duration since the node was used last.
    pub fn since(&self) -> Duration {
        self.last.elapsed().unwrap()
    }

    pub fn refresh(&mut self) {
        self.last = SystemTime::now()
    }

    pub fn bucket(&self, center: &'a Node) -> usize {
        self.address.bucket(&center.address)
    }

    pub fn print(&self) -> String {
        format!("ip: {}, key: {}", self.ip, self.address.public.unwrap())
    }

    // Helper functions for tests and debugging.
    pub fn helper(s: &'a str) -> Node<'a> {
        let a = Address::new(s);
        Node {
            ip: "",
            port: 0,
            address: a,
            last: SystemTime::now(),
        }
    }
}

impl<'a> Ord for Node<'a> {
    // This function calculates the order based the time difference that they were last seen.
    // The function will panic should the time be invalid or the local system time is broken.
    // "Greater" means older.
    fn cmp(&self, other: &Self) -> Ordering {
        self.last
            .elapsed()
            .unwrap()
            .cmp(&other.last.elapsed().unwrap())
    }
}
impl<'a> PartialOrd for Node<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a> Eq for Node<'a> {}
impl<'a> PartialEq for Node<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.last == other.last
    }
}
