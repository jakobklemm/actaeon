//! # Router
//!
//! The router is responsible for storing and sorting nodes as well as
//! providing callers with the information required to send messages
//! through the system.

use crate::node::{Address, Center, Node};

/// Binary tree structure holding the k-Buckets.
struct Table {
    root: Element,
    center: Center,
}

enum Element {
    Split(Subtree),
    Leaf(Bucket),
}

/// TODO: Naming?
struct Subtree {
    split: Address,
    near: Box<Element>,
    far: Box<Element>,
}

struct Bucket {
    nodes: Node,
    limit: usize,
}
