//! # Router Node
//!
//! A node has some basic elements used for routing and sorting.

use super::address::{Address, Connection};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

// Each node requires connection data as well as Kademlia Routing Data.
// It also stores the last seen time.
#[derive(Debug, Clone)]
pub struct Node {
    pub connection: Connection,
    pub address: Address,
    last: SystemTime,
}

impl Node {
    pub fn new(ip: &str, port: usize, public: &str) -> Self {
        let connection = Connection::new(ip, port as u16);
        let address = Address::new(public);
        Self {
            connection: connection,
            address: address,
            last: SystemTime::now(),
        }
    }

    pub fn compose(connection: Connection, address: Address) -> Self {
        Self {
            connection: connection,
            address: address,
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

    pub fn bucket(&self, center: &Node) -> usize {
        self.address.bucket(&center.address)
    }
}

impl Ord for Node {
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
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.last == other.last
    }
}
