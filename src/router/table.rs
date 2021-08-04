//! # Routing Table
//!
//! Binary tree of k-Buckets, inspired by Kademlia.

use super::bucket::Bucket;
use super::node::Node;
use std::collections::HashMap;

pub struct Table<'a> {
    nodes: HashMap<usize, Bucket<'a>>,
    size: usize,
    center: Node<'a>,
}

impl<'a> Table<'a> {
    pub fn new(center: Node<'a>, size: usize) -> Table<'a> {
        let mut nodes = HashMap::new();
        for n in 0..255 {
            nodes.insert(n, Bucket::new(size));
        }
        Self {
            nodes: nodes,
            size: size,
            center: center,
        }
    }

    pub fn add(&mut self, node: Node<'a>) {
        let bucket = self.nodes.get_mut(&node.bucket(&self.center)).unwrap();
        bucket.add(node);
    }

    pub fn run<F>(&self, node: &'a Node, count: usize, execute: F)
    where
        F: Fn(&Node),
    {
        // Get "count" nodes and execute F on them.
        let nodes = self.get(node, count);
        for i in nodes.iter() {
            execute(i);
        }
    }

    pub fn get(&self, node: &'a Node, count: usize) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        let node_bucket = node.bucket(&self.center);
        let bucket = self.nodes.get(&node_bucket).unwrap();
        nodes.append(&mut bucket.first(count));
        return nodes;
    }
}
