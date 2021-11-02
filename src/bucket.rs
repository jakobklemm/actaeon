//! # Bucket
//!
//! Leafs of the binary routing tree responsible for storing the
//! actual nodes in the binary tree. (Nodes are in the leafes, not the
//! "nodes" of the binary tree.) The nodes are stored sorted by time,
//! not their keys. Within each bucket nodes share a common property,
//! they are assumed to be "equally distanced". This might not be
//! perfect for very large buckets (higher up in the tree) but will
//! make no difference for buckets deeper in the tree.
//!
//! TODO: Create generic Bucket implementation based on std HashMap.

use crate::error::Error;
use crate::node::{Address, Center, Node};

/// Stores a maximum of "limit" nodes, sorted by age / time. The first
/// element in the array is the oldest one. This equals a Kademlia
/// k-Bucket. The bucket will be used in the binary routing tree as
/// a leaf..
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bucket {
    /// Stores the nodes, gets sorted by time, from old to new.
    nodes: Vec<Node>,
    /// Maximum length of the nodes array.
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
            self.dedup();
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
            self.dedup();
        } else {
            if let Some(first) = self.nodes.first_mut() {
                // instead of manually checking the status of the
                // oldest node it is assumed that it is updated by a
                // dedicated process.
                if !first.is_reachable() {
                    *first = node;
                }
                self.sort();
                self.dedup();
            }
        }
    }

    /// Takes ownership of the Bucket and returns two new once, with
    /// the nodes distributed between the two based on their distance
    /// in comparison to the upper limit. The center has to be
    /// provided to calculate the distance, the "ul" parameter is the
    /// upper limit of the bucket. When spliting the root bucket the
    /// upper limit would be 255 and the two new buckets would have
    /// upper limits of 127 and 255. This function will do no
    /// validation of size and will return even if one of the buckets
    /// is empty.
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

    /// With a provided address it will return a pointer to the node
    /// matching that address. This is for exact matches, not for
    /// getting targets. It will only every return one optional node.
    pub fn find(&self, search: &Address) -> Option<&Node> {
        let index = self.nodes.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.nodes.get(i),
            None => None,
        }
    }

    /// With a provided address it will return a pointer to the node
    /// matching that address. This is for exact matches, not for
    /// getting targets. It will only every return one optional node.
    pub fn find_mut(&mut self, search: &Address) -> Option<&mut Node> {
        let index = self.nodes.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.nodes.get_mut(i),
            None => None,
        }
    }

    /// Returns upto "limit" number of nodes. It only return
    /// references to the nodes, since the nodes in the routing table
    /// are expected to live the longest.
    pub fn get(&self, limit: usize) -> Vec<&Node> {
        let mut targets = Vec::new();
        for i in &self.nodes {
            targets.push(i);
        }
        targets.sort();
        targets.truncate(limit);
        return targets;
    }

    pub fn remove(&mut self, target: &Address) -> Result<(), Error> {
        let index = self.nodes.iter().position(|e| &e.address == target);
        match index {
            Some(i) => {
                self.nodes.remove(i);
                Ok(())
            }
            None => Err(Error::Unknown),
        }
    }

    /// Simple wrapper around the limit field so that all fields can
    /// remain private. It gets used by the router capacity query to
    /// calculate the maximum size of the entire tree.
    pub fn capacity(&self) -> usize {
        self.limit
    }

    /// Since it is possible to add nodes to the tree or a bucket
    /// multiple times some cleanup might be required. This function
    /// removes all nodes with duplicate addresses.
    pub fn dedup(&mut self) {
        self.sort_by_address();
        self.nodes.dedup_by(|a, b| a.address == b.address);
    }

    /// Wrapper around the length of the nodes array.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Uses the Ord and Partial Ord implementation Address to sort
    /// the nodes based on that. This does not represent the distance
    /// sorting for Kademlia but is just a shortcut for easier
    /// handling.
    fn sort_by_address(&mut self) {
        self.nodes
            .sort_by(|a, b| a.address.partial_cmp(&b.address).unwrap());
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

    #[test]
    fn test_bucket_get() {
        let mut root = Bucket::new(20);
        root.add(gen_node("first"));
        root.add(gen_node("second"));
        root.add(gen_node("another"));
        let targets = root.get(2);
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_bucket_remove() {
        let mut root = Bucket::new(20);
        root.add(gen_node("first"));
        root.add(gen_node("second"));
        let target = gen_node("first").address;
        root.remove(&target).unwrap();
        assert_eq!(root.len(), 1);
    }

    #[test]
    fn test_bucket_remove_empty() {
        let mut root = Bucket::new(20);
        let target = gen_node("first").address;

        assert_eq!(root.remove(&target).is_err(), true);
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
