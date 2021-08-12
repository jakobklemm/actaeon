//! # Routing Table
//!
//! Binary tree of k-Buckets, inspired by Kademlia.

use super::bucket::Bucket;
use super::node::Node;
use std::collections::HashMap;

pub struct Table<'a> {
    nodes: HashMap<usize, Bucket<'a>>,
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
        let node_bucket = node.bucket(&self.center);
        let bucket = self.nodes.get(&node_bucket).unwrap();
        if bucket.len() >= count {
            bucket.first(count)
        } else {
            self.get_all(node, count)
        }
    }

    pub fn get_all(&self, node: &'a Node, count: usize) -> Vec<&'a Node> {
        let nodes = self.join();
        for i in nodes.iter() {
            tracing::info!("{:?}", i.address.public);
        }
        let mut sorted = Table::sort(nodes, node);
        sorted.truncate(count);
        sorted
    }

    pub fn join(&self) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        for (_i, value) in &self.nodes {
            nodes.append(&mut value.all());
        }
        return nodes;
    }

    pub fn sort(all: Vec<&'a Node>, node: &'a Node) -> Vec<&'a Node<'a>> {
        let mut nodes = all;
        nodes.sort_by(|n, m| n.address.cmp(&m.address, &node.address));
        nodes
    }
}
