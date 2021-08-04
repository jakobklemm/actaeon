//! # Bucket
//!
//! Collection of nodes, sorted by time with operations to modify the bucket.

use super::node::Node;

pub struct Bucket<'a> {
    nodes: Vec<Node<'a>>,
    size: usize,
}

impl<'a> Bucket<'a> {
    pub fn new(size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            size,
        }
    }

    pub fn add(&mut self, mut node: Node<'a>) {
        if &mut self.nodes.len() < &mut self.size {
            node.refresh();
            self.nodes.push(node);
            self.nodes.sort();
        }
        // else: Replace oldest existing node & check availability.
    }

    pub fn first(&self, size: usize) -> Vec<&'a Node> {
        self.nodes.iter().take(size).collect()
    }
}
