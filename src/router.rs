//! # Router
//!
//! The router is responsible for storing and sorting nodes as well as
//! providing callers with the information required to send messages
//! through the system.
//! TODO: Add try_add testing.

use crate::bucket::Bucket;
use crate::error::Error;
use crate::node::{Address, Center, Node};

/// Binary tree structure holding the k-Buckets.
///
/// TODO: Pass Center everywhere or store 'distance' with each address
/// / node?
pub struct Table {
    root: Element,
    center: Center,
    limit: usize,
}

#[derive(Clone, Debug)]
struct Property {
    lower: u8,
    upper: u8,
}

#[derive(Clone, Debug)]
enum Element {
    Split(Split, Property),
    Leaf(Bucket, Property),
}

#[derive(Clone, Debug)]
struct Split {
    near: Box<Element>,
    far: Box<Element>,
}

impl Element {
    fn try_add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        match self {
            Self::Split(s, p) => {
                if !p.in_range(&node, &center) {
                    return Err(Error::Invalid(String::from("not in range")));
                }
                s.try_add(node, center)
            }
            Self::Leaf(b, p) => {
                if !p.in_range(&node, &center) {
                    return Err(Error::Invalid(String::from("not in range")));
                }
                b.try_add(node)
            }
        }
    }

    fn add(&mut self, node: Node, center: &Center) {
        match self {
            Self::Split(s, _) => s.add(node, center),
            Self::Leaf(b, _) => b.add(node),
        }
    }

    fn split(self, center: &Center) -> Option<Self> {
        match self {
            Self::Split(s, p) => return None,
            Self::Leaf(b, p) => {
                // Only "near" elements can be split.
                if p.lower != 0 {
                    return None;
                }
                let (near, far) = b.split(center, p.upper);
                let (near_p, far_p) = p.split();
                let split = Split {
                    near: Box::new(Self::Leaf(near, near_p)),
                    far: Box::new(Self::Leaf(far, far_p)),
                };
                Some(Self::Split(split, p))
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Split(s, _) => s.len(),
            Self::Leaf(b, _) => b.len(),
        }
    }

    fn in_range(&self, node: &Node, center: &Center) -> bool {
        match self {
            Self::Split(_, p) => p.in_range(node, center),
            Self::Leaf(_, p) => p.in_range(node, center),
        }
    }
}

impl Split {
    fn try_add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        if self.in_range_near(&node, center) {
            self.near.try_add(node, center)
        } else {
            self.far.try_add(node, center)
        }
    }

    fn add(&mut self, node: Node, center: &Center) {
        if self.in_range_near(&node, center) {
            self.near.add(node, center)
        } else {
            self.far.add(node, center)
        }
    }

    fn len(&self) -> usize {
        let mut length = self.far.len();
        match &*self.near {
            Element::Leaf(b, _) => length += b.len(),
            Element::Split(s, _) => length += s.len(),
        }
        return length;
    }

    fn in_range_near(&self, node: &Node, center: &Center) -> bool {
        self.near.in_range(node, center)
    }

    fn in_range_far(&self, node: &Node, center: &Center) -> bool {
        self.far.in_range(node, center)
    }
}

impl Property {
    /// TODO: Reduce clone calls.
    fn in_range(&self, node: &Node, center: &Center) -> bool {
        let index = (node.address.clone() ^ center.public.clone())[0];
        self.lower <= index && self.upper > index
    }

    fn split(&self) -> (Self, Self) {
        let lower = Self {
            lower: self.lower,
            upper: self.upper / 2,
        };
        let upper = Self {
            lower: (self.upper / 2) + 1,
            upper: self.upper,
        };
        (lower, upper)
    }

    fn is_near(&self) -> bool {
        self.lower == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

    #[test]
    fn test_property_in_range() {
        let p = Property {
            lower: 0,
            upper: 255,
        };
        let node = gen_node_near();
        let center = gen_center_near();
        assert_eq!(p.in_range(&node, &center), true);
        assert_eq!((node.address ^ center.public)[0], 0);
    }

    #[test]
    fn test_property_split_root() {
        let p = Property {
            lower: 0,
            upper: 255,
        };
        let (l, u) = p.split();
        assert_eq!(l.lower, 0);
        assert_eq!(l.upper, 127);
        assert_eq!(u.lower, 128);
        assert_eq!(u.upper, 255);
    }

    #[test]
    fn test_property_split_lower() {
        let p = Property {
            lower: 0,
            upper: 63,
        };
        let (l, u) = p.split();
        assert_eq!(l.lower, 0);
        assert_eq!(l.upper, 31);
        assert_eq!(u.lower, 32);
        assert_eq!(u.upper, 63);
    }

    #[test]
    fn test_property_near() {
        let p = Property {
            lower: 0,
            upper: 63,
        };
        let (l, u) = p.split();
        assert_eq!(l.is_near(), true);
        assert_eq!(u.is_near(), false);
    }

    #[test]
    fn test_element_split_root() {
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let buck = gen_bucket();
        let elem = Element::Leaf(buck, prop);
        let center = gen_center();
        let split = elem.split(&center).unwrap();
        match split {
            Element::Split(s, p) => {
                assert_eq!(p.upper, 255);
                assert_eq!(s.len(), 3);
                assert_eq!(s.near.as_ref().len(), 2);
            }
            Element::Leaf(_, _) => assert_eq!("invalid split", ""),
        }
    }

    #[test]
    fn test_element_split_far() {
        let prop = Property {
            lower: 128,
            upper: 255,
        };
        let buck = gen_bucket();
        let elem = Element::Leaf(buck, prop);
        let center = gen_center();
        let split = elem.split(&center).is_none();
        assert_eq!(split, true);
    }

    #[test]
    fn test_element_add_to_leaf() {
        let bucket = gen_bucket();
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let node = gen_node("added");
        let center = gen_center();
        elem.add(node, &center);
        assert_eq!(elem.len(), 4);
    }

    #[test]
    fn test_split_add_near_top() {
        let mut split = gen_split();
        let node = gen_node_near();
        let center = gen_center_near();
        split.add(node, &center);
        assert_eq!(split.len(), 1);
        assert_eq!(split.near.len(), 1);
        assert_eq!(split.far.len(), 0);
    }

    #[test]
    fn test_split_add_far_top() {
        let mut split = gen_split();
        let node = gen_node_far();
        let center = gen_center_near();
        let a = (node.address.clone() ^ center.public.clone())[0];
        split.add(node, &center);
        assert_eq!(split.len(), 1);
        assert_eq!(a, 255);
        assert_eq!(split.far.len(), 1);
    }

    #[test]
    fn test_split_add_deep() {
        let mut split = Split {
            near: Box::new(Element::Split(
                Split {
                    near: Box::new(Element::Leaf(
                        Bucket::new(20),
                        Property {
                            lower: 0,
                            upper: 63,
                        },
                    )),
                    far: Box::new(Element::Leaf(
                        Bucket::new(20),
                        Property {
                            lower: 64,
                            upper: 127,
                        },
                    )),
                },
                Property {
                    lower: 0,
                    upper: 127,
                },
            )),
            far: Box::new(Element::Leaf(
                Bucket::new(20),
                Property {
                    lower: 128,
                    upper: 255,
                },
            )),
        };
        assert_eq!(split.len(), 0);

        let center = gen_center_near();
        let node = gen_node_near();
        split.add(node, &center);
        assert_eq!(split.len(), 1);
        assert_eq!(split.near.as_ref().len(), 1);
        assert_eq!(split.far.as_ref().len(), 0);

        let node = gen_node_far();
        split.add(node, &center);
        assert_eq!(split.len(), 2);
        assert_eq!(split.near.as_ref().len(), 1);
        assert_eq!(split.far.as_ref().len(), 1);
    }

    #[test]
    fn test_split_in_range() {
        let split = gen_split();
        let center = gen_center_near();
        let node = gen_node_near();
        assert_eq!(split.in_range_near(&node, &center), true);
    }

    #[test]
    fn test_split_in_range_far() {
        let split = gen_split();
        let center = gen_center_near();
        let node = gen_node_far();
        assert_eq!(split.in_range_near(&node, &center), false);
    }

    fn gen_split() -> Split {
        let near = Bucket::new(20);
        let np = Property {
            lower: 0,
            upper: 127,
        };
        let near = Element::Leaf(near, np);
        let far = Bucket::new(20);
        let fp = Property {
            lower: 128,
            upper: 255,
        };
        let far = Element::Leaf(far, fp);
        Split {
            near: Box::new(near),
            far: Box::new(far),
        }
    }

    fn gen_bucket() -> Bucket {
        let mut root = Bucket::new(20);
        root.add(gen_node("first"));
        root.add(gen_node("second"));
        root.add(gen_node("another"));
        root
    }

    fn gen_node(s: &str) -> Node {
        Node::new(Address::generate(s).unwrap(), None)
    }

    fn gen_node_near() -> Node {
        let addr = Address::from_bytes([0; 32]).unwrap();
        Node::new(addr, None)
    }

    fn gen_node_far() -> Node {
        let addr = Address::from_bytes([255; 32]).unwrap();
        Node::new(addr, None)
    }

    fn gen_center() -> Center {
        let mut b = [0; 32];
        b[0] = 42;
        let s = SecretKey::from_slice(&b).unwrap();
        Center::new(s, String::from(""), 8080)
    }

    fn gen_center_near() -> Center {
        let secret = [0; 32];
        let secret = SecretKey::from_slice(&secret).unwrap();
        let public = [0; 32];

        let b = [0; 32];
        let s = SecretKey::from_slice(&b).unwrap();
        let base = Center::new(s, String::from(""), 8080);

        Center {
            secret,
            public: Address::from_bytes(public).unwrap(),
            ..base
        }
    }
}
