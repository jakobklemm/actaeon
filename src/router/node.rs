//! # Router Node
//!
//! A node has some basic elements used for routing and sorting.

use super::address::Address;
use std::cmp::Ord;
use std::time::{Duration, SystemTime};

// Each node requires connection data as well as Kademlia Routing Data.
// It also stores the last seen time.
pub struct Node<'a> {
    ip: &'a str,
    pub port: u16,
    address: Address<'a>,
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

    pub fn since(&self) -> Duration {
        self.last.elapsed().unwrap()
    }
}

/*
impl<'a> Ord for Node<'a> {}
impl<'a> PartialOrd for Node<'a> {}
impl<'a> Eq for Node<'a> {}
impl<'a> PartialEq for Node<'a> {}
*/
