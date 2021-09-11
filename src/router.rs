//! # Router
//!
//! The router is responsible for storing and sorting nodes as well as
//! providing callers with the information required to send messages
//! through the system.

use crate::error::Error;
use crate::node::{Address, Center, Node};

/// Binary tree structure holding the k-Buckets.
///
/// TODO: Replace direct root with interior mutability.
pub struct Table {
    root: Element,
    center: Center,
    limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Property {
    /// Might not be required since Near == ll: 0?
    side: Side,
    ll: u8,
    ul: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Side {
    Far,
    Near,
}

enum Element {
    Node(Point),
    Leaf(Leaf),
}

struct Point {
    property: Property,
    near: Box<Element>,
    far: Box<Element>,
}

/// Stores a maximum of "limit" nodes, sorted by age / time. The first
/// element in the array is the oldest one. This equals a Kademlia
/// k-Bucket. Since it is only used inside the binary routing tree it
/// simply called Leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Leaf {
    property: Property,
    nodes: Vec<Node>,
    limit: usize,
}

impl Point {
    fn new(property: Property, near: Leaf, far: Leaf) -> Self {
        Self {
            property,
            near: Box::new(Element::Leaf(near)),
            far: Box::new(Element::Leaf(far)),
        }
    }

    fn add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        if self.in_range_near(&node, center) {
            self.add_near(node, center)
        } else if self.in_range_far(&node, center) {
            self.add_far(node);
            Ok(())
        } else {
            Err(Error::Invalid(String::from("node not in range")))
        }
    }

    /// TODO: Reduce clone calls.
    fn add_near(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        match self.near.as_mut() {
            Element::Leaf(leaf) => {
                let res = leaf.try_add(node, center);
                // this function should only get called if the node is
                // in range, therefor try_add should only return
                // Error::Full.
                if res.is_err() {
                    let (near, far) = leaf.clone().split(center);
                    let new = Self {
                        property: self.property.clone(),
                        near: Box::new(Element::Leaf(near)),
                        far: Box::new(Element::Leaf(far)),
                    };
                    self.near = Box::new(Element::Node(new));
                    Ok(())
                } else {
                    Ok(())
                }
            }
            Element::Node(point) => point.add_near(node, center),
        }
    }

    fn add_far(&mut self, node: Node) {
        match self.far.as_mut() {
            Element::Leaf(leaf) => leaf.add(node),
            Element::Node(point) => point.add_far(node),
        }
    }

    fn in_range_near(&self, node: &Node, center: &Center) -> bool {
        match self.near.as_ref() {
            Element::Leaf(leaf) => leaf.in_range(node, center),
            Element::Node(point) => point.in_range_near(node, center),
        }
    }

    fn in_range_far(&self, node: &Node, center: &Center) -> bool {
        match self.far.as_ref() {
            Element::Leaf(leaf) => leaf.in_range(node, center),
            Element::Node(point) => point.in_range_far(node, center),
        }
    }

    fn get(&self, node: &Node, center: &Center, count: usize) -> Vec<Node> {
        let mut initial = self.find(node, center).nodes;
        if initial.len() < count {
            for offset in 0..255 {
                if node.address.as_bytes()[0] + offset == 255 {
                    let mut bytes = node.address.as_bytes();
                    bytes[0] = bytes[0] + offset;
                    let search_addr = Address::from_bytes(bytes);
                    if search_addr.is_err() {
                        continue;
                    }
                    let search_node = Node::new(search_addr.unwrap(), None);
                    let mut more = self.get(&search_node, center, count);
                    initial.append(&mut more);
                    if initial.len() > count {
                        break;
                    }
                }
                if node.address.as_bytes()[0] - offset == 255 {
                    let mut bytes = node.address.as_bytes();
                    bytes[0] = bytes[0] - offset;
                    let search_addr = Address::from_bytes(bytes);
                    if search_addr.is_err() {
                        continue;
                    }
                    let search_node = Node::new(search_addr.unwrap(), None);
                    let mut more = self.get(&search_node, center, count);
                    initial.append(&mut more);
                    if initial.len() > count {
                        break;
                    }
                }
            }
        };

        return initial;
    }

    /// returns the leaf a node should belong into.
    fn find(&self, node: &Node, center: &Center) -> Leaf {
        if self.in_range_far(node, center) {
            match self.far.as_ref() {
                Element::Leaf(leaf) => leaf.clone(),
                Element::Node(point) => point.find(node, center),
            }
        } else {
            match self.far.as_ref() {
                Element::Leaf(leaf) => leaf.clone(),
                Element::Node(point) => point.find(node, center),
            }
        }
    }
}

impl Property {
    /// Determins whether a node is in range of a bucket / the
    /// properties.
    fn in_range(&self, node: &Node, center: &Center) -> bool {
        // TODO: Fix clone?
        let index = (node.address.clone() ^ center.public.clone())[0];
        self.ll < index && self.ul > index
    }
}

impl Leaf {
    /// Creates a new empty bucket and stores the provided limit. It
    /// usually has to be declared as mutable, since sorting and
    /// adding require mutable references. This might get replaced by
    /// interior mutability in the future. It should be enough to just
    /// store the nodes in a RefCell, since everything else stays
    /// constant.
    fn new(limit: usize, property: Property) -> Self {
        Self {
            nodes: Vec::new(),
            property,
            limit,
        }
    }

    /// Sorts the nodes in the bucket with the existing Ord
    /// implementation. THe first element in the Vector will be the
    /// oldest one, the last the oldest.
    fn sort(&mut self) {
        self.nodes.sort();
    }

    /// Adds a node to a bucket. If the bucket is full, an error is
    /// returned. This function does not follow any of the kademlia
    /// rules and is intended to be used as a first step, before
    /// specifiying the behavior for full buckets. Currently a
    /// dedicated error type is used for situations like this:
    /// Error::Full. Should the node not belong in this bucket, the
    /// function will also fail.
    fn try_add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        // TODO: Don't clone?
        if !self.in_range(&node, &center) {
            return Err(Error::Invalid(String::from("mismatched bucket")));
        } else if self.len() == self.limit {
            return Err(Error::Full);
        } else {
            self.nodes.push(node);
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
    fn add(&mut self, node: Node) {
        if self.len() < self.limit {
            self.nodes.push(node);
            self.sort();
        } else {
            if let Some(first) = self.nodes.first_mut() {
                // instead of manually checking the status of the
                // oldest node it is assumed that it is updated by a
                // dedicated process.
                if !first.is_reachable() {
                    *first = node;
                }
                self.sort();
            }
        }
    }

    /// Turns one Leaf (Bucket) into to separate halves. It does't
    /// check for size or size. It takes ownership of the Leaf and
    /// returns two new ones as a tuple. The first one is Near, the
    /// second one is Far.
    fn split(mut self, center: &Center) -> (Self, Self) {
        let near_property = Property {
            side: Side::Near,
            ll: self.property.ll,
            ul: self.property.ul / 2,
        };

        let far_property = Property {
            side: Side::Far,
            ll: (self.property.ul / 2) + 1,
            ul: self.property.ul,
        };

        let mut near = Leaf::new(self.limit, near_property);
        let mut far = Leaf::new(self.limit, far_property);

        for i in self.nodes {
            if near.in_range(&i, center) {
                near.add(i);
            } else {
                far.add(i);
            }
        }

        return (near, far);
    }

    /// Wrapper around Property::in_range method.
    fn in_range(&self, node: &Node, center: &Center) -> bool {
        self.property.in_range(node, center)
    }

    /// TOOD: Add dedup by key.
    fn dedup(&mut self) {
        self.sort();
        self.nodes.dedup();
    }

    /// Wrapper around the length of the nodes array.
    fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;
    use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

    #[test]
    fn test_leaf_add() {
        let mut leaf = gen_leaf(20);
        let node = gen_node("test");
        let center = gen_center();
        let ret = leaf.try_add(node, &center);
        assert_eq!(ret.is_err(), false);
    }

    #[test]
    fn test_leaf_add_error() {
        let mut leaf = gen_leaf(1);
        let node = gen_node("test");
        let center = gen_center();
        leaf.try_add(node, &center).unwrap();
        let node = gen_node("test2");
        let ret = leaf.try_add(node, &center);
        assert_eq!(ret.is_err(), true);
    }

    #[test]
    fn test_leaf_add_error_lim() {
        let mut leaf = gen_leaf(1);
        let node = gen_node("test");
        let center = gen_center();
        leaf.try_add(node, &center).unwrap();
        let node = gen_node("test2");
        let ret = leaf.try_add(node, &center);
        assert_eq!(ret.is_err(), true);
    }

    #[test]
    fn test_leaf_add_disregard() {
        let mut leaf = gen_leaf(1);
        let node = gen_node("test");
        leaf.add(node);
        let node = gen_node("test2");
        leaf.add(node);
        assert_eq!(leaf.len(), 1);
    }

    #[test]
    fn test_leaf_add_error_range() {
        let mut leaf = gen_lim_leaf(10);
        let node = gen_node("test");
        let center = gen_center();
        let ret = leaf.try_add(node, &center);
        assert_eq!(ret.is_err(), true);
    }

    #[test]
    fn test_leaf_split() {
        let mut leaf = gen_leaf(2);
        let node = gen_node("test");
        leaf.add(node);
        let node = gen_node("test2");
        leaf.add(node);
        let mut cb: [u8; 32] = [0; 32];
        cb[0] = 42;
        let secret = SecretKey::from_slice(&cb).unwrap();
        let center = Center::new(secret, String::from(""), 42);

        let (l1, l2) = leaf.split(&center);
        assert_eq!(l1.len(), 0);
        assert_eq!(l2.len(), 2);
    }

    #[test]
    fn test_leaf_range() {
        let leaf = gen_lim_leaf(10);
        let node = gen_node("abc");
        let center = gen_center();
        assert_eq!(leaf.in_range(&node, &center), false);
    }

    #[test]
    fn test_leaf_dedup() {
        let mut leaf = gen_leaf(10);
        let node = gen_node("abc");
        leaf.add(node);
        let node = gen_node("abc");
        leaf.add(node);
        leaf.dedup();
        assert_eq!(leaf.len(), 1);
    }

    fn gen_leaf(l: usize) -> Leaf {
        let prop = Property {
            side: Side::Near,
            ll: 0,
            ul: 255,
        };
        Leaf::new(l, prop)
    }

    fn gen_lim_leaf(l: usize) -> Leaf {
        let prop = Property {
            side: Side::Near,
            ll: 0,
            ul: 0,
        };
        Leaf::new(l, prop)
    }

    fn gen_center() -> Center {
        let (_, s) = box_::gen_keypair();
        Center::new(s, "a".to_string(), 42)
    }

    fn gen_node(s: &str) -> Node {
        Node::new(Address::generate(s).unwrap(), None)
    }
}
