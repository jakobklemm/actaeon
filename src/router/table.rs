//! # Routing Table
//!
//! Binary tree of k-Buckets, inspired by Kademlia.

use super::bucket::Bucket;
use super::node::Node;
use std::collections::HashMap;

pub struct Table {
    nodes: HashMap<usize, Bucket>,
    center: Node,
}

impl Table {
    pub fn new(center: Node, size: usize) -> Self {
        let mut nodes = HashMap::new();
        for n in 0..255 {
            nodes.insert(n, Bucket::new(size));
        }
        Self {
            nodes: nodes,
            center: center,
        }
    }

    pub fn add(&mut self, node: Node) {
        let bucket = self.nodes.get_mut(&node.bucket(&self.center)).unwrap();
        bucket.add(node);
    }

    pub fn run<F>(&self, node: &Node, count: usize, execute: F)
    where
        F: Fn(&Node),
    {
        // Get "count" nodes and execute F on them.
        let nodes = self.get(node, count);
        for i in nodes.iter() {
            execute(i);
        }
    }

    pub fn get(&self, node: &Node, count: usize) -> Vec<Node> {
        let node_bucket = node.bucket(&self.center);
        let bucket = self.nodes.get(&node_bucket).unwrap();
        if bucket.len() >= count {
            bucket.first(count)
        } else {
            self.get_all(node, count)
        }
    }

    pub fn get_all(&self, node: &Node, count: usize) -> Vec<Node> {
        let nodes = self.join();
        let mut sorted = Table::sort(nodes, node);
        sorted.truncate(count);
        sorted
    }

    pub fn join(&self) -> Vec<Node> {
        let mut nodes = Vec::new();
        for (_i, value) in &self.nodes {
            nodes.append(&mut value.all());
        }
        return nodes;
    }

    pub fn sort(all: Vec<Node>, node: &Node) -> Vec<Node> {
        let mut nodes = all;
        nodes.sort_by(|n, m| n.address.cmp(&m.address, &node.address));
        nodes
    }
}
