//! # Bucket
//!
//! Leafs of the binary routing tree.

use crate::error::Error;
use crate::node::{Center, Node};

/// Stores a maximum of "limit" nodes, sorted by age / time. The first
/// element in the array is the oldest one. This equals a Kademlia
/// k-Bucket. Since it is only used inside the binary routing tree it
/// simply called Leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bucket {
    nodes: Vec<Node>,
    limit: usize,
}

impl Bucket {
    /// Creates a new empty bucket and stores the provided limit. It
    /// usually has to be declared as mutable, since sorting and
    /// adding require mutable references. This might get replaced by
    /// interior mutability in the future. It should be enough to just
    /// store the nodes in a RefCell, since everything else stays
    /// constant.
    pub fn new(limit: usize) -> Self {
        Bucket {
            nodes: Vec::new(),
            limit,
        }
    }

    /// Sorts the nodes in the bucket with the existing Ord
    /// implementation. THe first element in the Vector will be the
    /// oldest one, the last the oldest.
    pub fn sort(&mut self) {
        self.nodes.sort();
    }

    /// Adds a node to a bucket. If the bucket is full, an error is
    /// returned. This function does not follow any of the kademlia
    /// rules and is intended to be used as a first step, before
    /// specifiying the behavior for full buckets. Currently a
    /// dedicated error type is used for situations like this:
    /// Error::Full. Should the node not belong in this bucket, the
    /// function will also fail.
    pub fn try_add(&mut self, node: Node) -> Result<(), Error> {
        if self.len() == self.limit {
            return Err(Error::Full);
        } else {
            self.nodes.push(node);
            return Ok(());
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
    pub fn add(&mut self, node: Node) {
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

    pub fn split(self, center: &Center, ul: u8) -> (Self, Self) {
        let mut near = Bucket::new(self.limit);
        let mut far = Bucket::new(self.limit);

        for i in self.nodes {
            let index = (i.address.clone() ^ center.public.clone())[0];
            if index < (ul / 2) {
                near.add(i);
            } else {
                far.add(i);
            }
        }

        (near, far)
    }

    /// TOOD: Add dedup by key.
    pub fn dedup(&mut self) {
        self.sort();
        self.nodes.dedup();
    }

    /// Wrapper around the length of the nodes array.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Address;
    use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

    #[test]
    fn test_leaf_add() {
        let mut bucket = gen_bucket(20);
        let node = gen_node("test");
        let ret = bucket.try_add(node);
        assert_eq!(ret.is_err(), false);
    }

    #[test]
    fn test_leaf_add_error() {
        let mut leaf = gen_bucket(1);
        let node = gen_node("test");
        leaf.try_add(node).unwrap();
        let node = gen_node("test2");
        let ret = leaf.try_add(node);
        assert_eq!(ret.is_err(), true);
    }

    #[test]
    fn test_bucket_add_error_lim() {
        let mut bucket = gen_bucket(1);
        let node = gen_node("test");
        bucket.try_add(node).unwrap();
        let node = gen_node("test2");
        let ret = bucket.try_add(node);
        assert_eq!(ret.is_err(), true);
    }

    #[test]
    fn test_bucket_add_disregard() {
        let mut bucket = gen_bucket(1);
        let node = gen_node("test");
        bucket.add(node);
        let node = gen_node("test2");
        bucket.add(node);
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn test_bucket_dedup() {
        let mut bucket = gen_bucket(10);
        let node = gen_node("abc");
        bucket.add(node);
        let node = gen_node("abc");
        bucket.add(node);
        bucket.dedup();
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn test_bucket_split_root() {
        let mut root = Bucket::new(20);
        root.add(gen_node("first"));
        root.add(gen_node("second"));
        root.add(gen_node("another"));
        let center = gen_center();
        let (near, far) = root.split(&center, 255);
        assert_eq!(near.len(), 2);
        assert_eq!(far.len(), 1);
    }

    fn gen_bucket(l: usize) -> Bucket {
        Bucket::new(l)
    }

    fn gen_node(s: &str) -> Node {
        Node::new(Address::generate(s).unwrap(), None)
    }

    fn gen_center() -> Center {
        let mut b = [0; 32];
        b[0] = 42;
        let s = SecretKey::from_slice(&b).unwrap();
        Center::new(s, String::from(""), 8080)
    }
}
