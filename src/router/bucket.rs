//! # Router Bucket
//!
//! Each bucket stores a collection of upto =k= nodes, sorted by the time they were last seen.

use super::node::Node;

// The main element of a bucket is a Vector of nodes.
pub struct Bucket<'a> {
    pub nodes: Vec<Node<'a>>,
}

impl<'a> Bucket<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Self { nodes: vec![node] }
    }

    pub fn sort(&mut self) {
        let mut sorted: Vec<Node<'a>> = Vec::new();
    }
}
