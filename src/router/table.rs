//! # Routing Table
//!
//! Binary tree of k-Buckets, inspired by Kademlia.

use super::node::Node;

pub struct Table<'a> {
    nodes: Vec<Node<'a>>,
    center: &'a Node<'a>,
    count: usize,
}

impl<'a> Table<'a> {
    pub fn new(center: &'a Node, count: usize) -> Table<'a> {
        Table {
            nodes: Vec::new(),
            center,
            count,
        }
    }

    pub fn extract(&self, new: &'a Node, count: usize) -> Bucket<'a> {
        let size = new.address.bytes[0];
        let filtered = self
            .nodes
            .into_iter()
            .filter(|x| x.address.bytes[0] == size)
            .collect();
        Bucket {
            nodes: filtered,
            size: size as usize,
            count: count,
        }
    }

    fn size(node: Node) -> usize {
        node.address.bucket()
    }
}

struct Bucket<'a> {
    nodes: Vec<Node<'a>>,
    size: usize,
    count: usize,
}

impl<'a> Bucket<'a> {
    fn new(nodes: Vec<Node<'a>>, size: usize, count: usize) -> Self {
        Self { nodes, size, count }
    }
}
