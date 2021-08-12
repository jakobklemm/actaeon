//! # Bucket
//!
//! Collection of nodes, sorted by time with operations to modify the bucket.

use super::node::Node;

pub struct Bucket {
    nodes: Vec<Node>,
    size: usize,
}

impl Bucket {
    pub fn new(size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            size,
        }
    }

    pub fn add(&mut self, mut node: Node) {
        if &mut self.nodes.len() < &mut self.size {
            node.refresh();
            self.nodes.push(node);
            self.nodes.sort();
        }
        // else: Replace oldest existing node & check availability.
    }

    pub fn first(&self, size: usize) -> Vec<Node> {
        //self.nodes.iter().take(size).collect()
	let mut selected = self.nodes.clone();
        selected.truncate(size);
        selected
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn all(&self) -> Vec<Node> {
        let mut nodes = Vec::new();
        for i in self.nodes.clone().into_iter() {
            nodes.push(i);
        }
        nodes
    }
}

impl Iterator for Bucket {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.nodes.reverse();
        let element = self.nodes.pop();
        self.nodes.reverse();
        return element;
    }
}
