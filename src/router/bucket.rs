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

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn all(&self) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        for i in &self.nodes {
            nodes.push(i);
        }
        nodes
    }
}

impl<'a> Iterator for Bucket<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Node<'a>> {
        self.nodes.reverse();
        let element = self.nodes.pop();
        self.nodes.reverse();
        return element;
    }
}

#[test]
fn take_none() {
    let mut b = Bucket::new(20);
    b.add(Node::helper("abc"));
    b.add(Node::helper("def"));
    let sel = b.first(0);
    assert_eq!(sel.len(), 0);
}
