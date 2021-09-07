//! # Router
//!
//! The router is responsible for storing and sorting nodes as well as
//! providing callers with the information required to send messages
//! through the system.

use crate::error::Error;
use crate::node::{Address, Center, Node};

/// Binary tree structure holding the k-Buckets.
///
/// TODO: Replace direct root with interior mutability.
pub struct RoutingTable {
    root: Element,
    center: Center,
    limit: usize,
}

enum Element {
    Split(RTNode),
    Leaf(Bucket),
}

struct RTNode {
    split: Address,
    near: Box<Element>,
    far: Box<Element>,
}

/// Stores a maximum of "limit" nodes, sorted by age / time. The first
/// element in the array is the oldest one.
struct Bucket {
    nodes: Vec<Node>,
    limit: usize,
}

impl Bucket {
    /// Creates a new empty bucket and stores the provided limit. It
    /// usually has to be declared as mutable, since sorting and
    /// adding require mutable references. This might get replaced by
    /// interior mutability in the future.
    fn new(limit: usize) -> Self {
        Self {
            nodes: Vec::new(),
            limit,
        }
    }

    /// Sorts the nodes in the bucket with the existing Ord
    /// implementation. THe first element in the Vector will be the
    /// oldest one, the last the oldest.
    fn sort(&mut self) {
        self.nodes.sort();
    }

    fn add_center(&mut self, node: Node) -> Result<(), Error> {
        if self.len() <= self.limit {
            self.nodes.push(node);
            Ok(())
        } else {
            Err(Error::Full)
        }
    }

    /// Adds a new node the the existing bucket, in which the center
    /// is not. It roughly follows the Kademlia update rules:
    ///
    /// - If there is still space in the bucket, the node is simply
    /// appended.
    ///
    /// - If there is no space, the oldest node gets replaced, but
    /// only if it is currently not reachable. This part requires the
    /// nodes in the table to get checked by a dedicated process. No
    /// status checks are happening in the table.
    ///
    /// This function will not split buckets or create new, should the
    /// bucket be full the node is simply disregarded.
    fn add_other(&mut self, node: Node) {
        if self.len() < self.limit {
            self.nodes.push(node);
            self.sort();
        } else {
            if let Some(first) = self.nodes.first_mut() {
                // instead of manually checking the status of the
                // oldest node it is assumed that it is updated by a
                // dedicated process.
                if !first.is_reachable() {
                    *first = node;
                }
                self.sort();
            }
        }
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::node::Link;

    #[test]
    fn test_bucket_create() {
        let bucket = Bucket::new(42);
        assert_eq!(bucket.limit, 42);
    }

    #[test]
    fn test_bucket_sort() {
        let mut bucket = Bucket::new(42);
        let node1 = node("abc");
        let node2 = node("def");
        bucket.add_other(node2);
        bucket.add_other(node1.clone());
        bucket.sort();
        assert_eq!(bucket.nodes[0], node1);
        assert_eq!(bucket.len(), 2);
    }

    #[test]
    fn test_bucket_other_replace() {
        let mut bucket = Bucket::new(1);
        let node1 = node("abc");
        let node2 = node("def");
        bucket.add_other(node2.clone());
        bucket.add_other(node1.clone());
        bucket.sort();
        assert_eq!(bucket.nodes[0], node1);
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn test_bucket_other_ignore() {
        let mut bucket = Bucket::new(1);
        let node1 = node("abc");
        let mut link2 = Link::new(String::new(), 42);
        link2.update(true);
        let node2 = Node::new(Address::generate("").unwrap(), Some(link2));
        bucket.add_other(node2.clone());
        bucket.add_other(node1.clone());
        bucket.sort();
        assert_eq!(bucket.nodes[0], node2);
        assert_eq!(bucket.len(), 1);
    }

    fn node(s: &str) -> Node {
        Node::new(Address::generate(s).unwrap(), None)
    }
}
