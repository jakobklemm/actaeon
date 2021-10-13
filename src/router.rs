//! # Router
//!
//! The router is responsible for storing and sorting nodes as well as
//! providing callers with the information required to send messages
//! through the system.

use crate::bucket::Bucket;
use crate::error::Error;
use crate::node::{Address, Center, Node};

/// The entry and interaction point for the binary routing tree. It
/// holds the root of the tree and is mainly a nice interface for the
/// internals of the tree. Currently the tree is stored directly in
/// the struct, in the future this might have to get replaced by a
/// Mutex and Arc, once a dedicated processing thread has been added.
pub struct Table {
    /// The first element of the routing tree. It is the only one that
    /// does not have to be heap allocated, since it can be accessed
    /// directly.
    root: Element,
    /// Since many of the distance calculations require the Center, it
    /// is stored here and will be passed to the functions internally.
    center: Center,
}

/// In order to simplify and modularize the binary tree the Elements
/// don't store the necessary metadata themselves. Instead in each
/// Element the Properties will be stored separately. The Properties
/// describe the range of each Element, as expressed through the lower
/// and upper limit index. The limits determine which Nodes can be
/// stored in a specific element, meaning the first byte of the
/// distance of any Node for a given Element must be between the lower
/// and upper limit. The root Element will always have limits of 0 and
/// 255 since it covers the entire range. When the root element gets
/// split the Properties will also be split automatically, meaning the
/// two lower Elements will have limits of 0, 127 and 128, 255. If the
/// lower limit is zero the Element is "near", if it is anything but
/// zero it is "far". This simply describes what side of the tree any
/// element is on. Any element that would contain the center is
/// considered "near", all other elements are "far". Only "near"
/// Elements can get split, Nodes in "far" Elements will get replaced.
#[derive(Clone, Debug)]
struct Property {
    /// The lower limit of the Element, zero means the Element is "near"
    lower: u8,
    /// The upper limit of the Element maximum is 255, only the root
    /// and the first "far" split can have that.
    upper: u8,
}

/// The mail component of the binary routing tree. Each /node/ (binary
/// tree node not remote Nodes) is either a Split or a Leaf and both
/// are combined with properties. The two variants of the Enum
/// correspond to two different structs and the properties. Most
/// methods use recursive calculations to iterate through the entire
/// Split structure.
#[derive(Clone, Debug)]
enum Element {
    /// Element that represents a Split in the binary tree, meaning
    /// there are two more Elements below it. It holds the dedicated
    /// Split struct and properties.
    Split(Split, Property),
    /// Represents a /node/ (=Element) of the binary tree that has no
    /// more child Elements, meaning it is a Leaf. It holds a bucket
    /// for the actual nodes as well as the properties.
    Leaf(Bucket, Property),
}

/// Struct representing a Node in the routing table (binary tree) that
/// has two child nodes. Since the struct holds "recursive
/// definitions" of the Element struct the two subelements need to be
/// heap allocated (boxed). The "far" Element would not have to be
/// boxed or even an Element, since it can always only be a Leaf
/// (following the Kademlia rules). But in order to make the functions
/// more unified both sides are represented equally.
#[derive(Clone, Debug)]
struct Split {
    /// The Element containing the Center, here called "near".
    near: Box<Element>,
    /// The Element not containing the Center, here called "far". This
    /// always is a Leaf / Bucket.
    far: Box<Element>,
}

impl Table {
    /// Creates a new routing table and populates the root Element
    /// with an empty Leaf / Bucket covering the entire range limit (0
    /// to 255).
    pub fn new(limit: usize, center: Center) -> Table {
        Table {
            root: Element::Leaf(
                Bucket::new(limit),
                Property {
                    lower: 0,
                    upper: 255,
                },
            ),
            center,
        }
    }

    /// Attempts to add a node to the routing table. It will fail if
    /// the bucket it should go into is full and it won't change the
    /// structure of the table, meaning it won't split any Elements.
    /// But if the target Leaf is "far", meaning splitting the Element
    /// wouldn't have been an option anyways, it will replace the
    /// oldest, non reachable Node in the Table or disregard the new
    /// Node. There is no guarantee a new Node will actually get
    /// added. This follows the Kademlia rules of preferring old,
    /// available Nodes over new ones.
    pub fn try_add(&mut self, node: Node) -> Result<(), Error> {
        self.root.try_add(node, &self.center)
    }

    /// The main function for adding new Nodes two the Table. Like
    /// "try_add" it also doesn't guarantee a Node will be added for
    /// Nodes on the "far" side, but the structure of the Table will
    /// get changed for "near" nodes. If the new Node belongs into an
    /// Element at maximum capacity it will get split into two new
    /// Leaves.
    pub fn add(&mut self, node: Node) {
        self.root.add(node, &self.center);
    }

    /// Opposite of "try_add", will remove a Node with the matching
    /// Address from the table. Since that might make parts of the
    /// tree under used, the shape can get updated after removal.
    /// Should the Address not be in the Table this function will
    /// fail.
    pub fn remove(&mut self, address: &Address) -> Result<(), Error> {
        self.root.remove(address, &self.center)
    }

    /// Takes an Address and returns an optional Node if a Node with
    /// exactly that Address exists. This is not meant as a way of
    /// finding new targets for messages but for checking if a Node
    /// exists in the Table or fetching specific connection data.
    pub fn find(&self, address: &Address) -> Option<&Node> {
        self.root.find(address, &self.center)
    }

    /// Takes an Address and returns an optional Node if a Node with
    /// exactly that Address exists. This is not meant as a way of
    /// finding new targets for messages but for checking if a Node
    /// exists in the Table or fetching specific connection data.
    pub fn find_mut(&mut self, address: &Address) -> Option<&mut Node> {
        self.root.find_mut(address, &self.center)
    }

    /// Returns the closest nodes to the search address, no matter the
    /// shape of the binary tree. If there are less than the requested
    /// number of nodes in the tree only that amount will be returned,
    /// the function can't fail. The Nodes are not guaranteed to be
    /// ordered by size since each bucket internally is ordered by
    /// age. Overall the nodes should roughly be ordered, but that is
    /// not reliable. This function will be used for sending the
    /// actual messages to the k-closest Nodes.
    pub fn get(&self, address: &Address, limit: usize) -> Vec<&Node> {
        self.root.get(address, &self.center, limit)
    }

    /// Mostly the same as get but copies the found nodes instead of
    /// returning a pointer to them. Since the Node at some point will
    /// have to be copied for the TCP handler, this function makes the
    /// copy earlier.
    pub fn get_copy(&self, address: &Address, limit: usize) -> Vec<Node> {
        let mut nodes = Vec::new();
        let refs = self.get(address, limit);

        for n in refs {
            nodes.push(n.clone());
        }

        return nodes;
    }

    /// Returns the current maximum capacity of the tree. The capacity
    /// is the sum of all maximum sizes of all Leaves / Buckets. The
    /// absolute limit is 255 times the size of each bucket, since
    /// there are a maximum of 255 Buckets in the Table.
    pub fn capacity(&self) -> usize {
        self.root.capacity()
    }

    /// Change the link state of a Node in the Table. This function
    /// can both be used to change the state of the link and also to
    /// update the state after no change was found. This will update
    /// the internal counter for how many times attempts have been
    /// made to reach a Node.
    pub fn status(&mut self, address: &Address, status: bool) {
        match self.root.find_mut(address, &self.center) {
            Some(node) => node.update(status),
            None => (),
        }
    }

    /// Returns the total number of Nodes in the entire Table.
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// Return the Address of the Center. Shorthand for the public
    /// field.
    pub fn center(&self) -> Address {
        self.center.public.clone()
    }
}

impl Element {
    /// Add to an element if possible. If the far bucket is full a
    /// node will get replaced following kademlia rules. This function
    /// does not handle refreshing and validating.
    fn try_add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        match self {
            Self::Split(s, p) => {
                if !p.in_range(&node.address, &center) {
                    return Err(Error::Invalid(String::from("not in range")));
                }
                if p.is_near() {
                    s.try_add(node, center)
                } else {
                    s.add(node, center);
                    Ok(())
                }
            }
            Self::Leaf(b, p) => {
                if !p.in_range(&node.address, &center) {
                    return Err(Error::Invalid(String::from("not in range")));
                }
                b.try_add(node)
            }
        }
    }

    /// Adds the Node to the Element. If the "near" Element is already
    /// full it gets split and the Element gets added to the new
    /// Split.
    fn add(&mut self, node: Node, center: &Center) {
        match self {
            Self::Split(s, _) => s.add(node, center),
            Self::Leaf(b, p) => {
                if p.is_near() {
                    match b.try_add(node.clone()) {
                        Ok(()) => return,
                        Err(_) => {
                            // bucket is full => split it. unwrap is
                            // not an issue, the split only fails if
                            // the element is not near.
                            *self = self.clone().split(center).unwrap();
                            self.add(node, center);
                        }
                    }
                } else {
                    b.add(node)
                }
            }
        }
    }

    /// Removes a node from the Element. Currently the function can
    /// panic, should the split fail. Split gets called once always,
    /// but the shape of the tree will only change if required. If a
    /// split is required a new Element is generated and this current
    /// object is replaced with the new one.
    fn remove(&mut self, address: &Address, center: &Center) -> Result<(), Error> {
        if let None = self.find(address, center) {
            return Err(Error::Unknown);
        }
        match self {
            Self::Split(s, _) => {
                if s.is_final() {
                    let _ = s.remove(address, center);
                    // collaps gets only called if half the capacity
                    // if less than the length.
                    if (s.capacity() / 2) > s.len() {
                        if let Ok(e) = s.collapse() {
                            // the actual Element (self) gets replaced.
                            *self = e;
                        } else {
                            return Err(Error::Unknown);
                        }
                    }
                } else {
                    s.remove(address, center)?;
                }
            }
            Self::Leaf(b, _) => {
                b.remove(address)?;
            }
        }
        Ok(())
    }

    /// Takes ownership of an Element (Leaf) and splits into two new
    /// ones, which gets returned as a new Split Element. The center
    /// is required to calculate the distance. The new Elements will
    /// all have their properties calculated automatically.
    fn split(self, center: &Center) -> Option<Self> {
        match self {
            Self::Split(_, _) => return None,
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

    /// Returns a pointer to a Node if the provided Address exists in
    /// the Table.
    fn find(&self, search: &Address, center: &Center) -> Option<&Node> {
        if !self.in_range(search, center) {
            return None;
        }
        match self {
            Self::Split(s, _) => s.find(search, center),
            Self::Leaf(b, _) => b.find(search),
        }
    }

    /// Returns a pointer to a Node if the provided Address exists in
    /// the Table.
    fn find_mut(&mut self, search: &Address, center: &Center) -> Option<&mut Node> {
        if !self.in_range(search, center) {
            return None;
        }
        match self {
            Self::Split(s, _) => s.find_mut(search, center),
            Self::Leaf(b, _) => b.find_mut(search),
        }
    }

    /// Gets upto the "limit" number of nodes closest to the target
    /// address. => bottom up recursion
    fn get(&self, target: &Address, center: &Center, limit: usize) -> Vec<&Node> {
        match self {
            Self::Split(s, _) => s.get(target, center, limit),
            Self::Leaf(b, _) => b.get(limit),
        }
    }

    /// Calculates the count of all Nodes under this Element.
    fn len(&self) -> usize {
        match self {
            Self::Split(s, _) => s.len(),
            Self::Leaf(b, _) => b.len(),
        }
    }

    /// Returns the maximum size of all buckets under an element.
    fn capacity(&self) -> usize {
        let mut sum = 0;
        match self {
            Self::Split(s, _) => sum += s.capacity(),
            Self::Leaf(b, _) => sum += b.capacity(),
        }
        return sum;
    }

    /// Uses the properties of an Element to determine if an Address
    /// can be stored in this Element (or below it).
    fn in_range(&self, address: &Address, center: &Center) -> bool {
        match self {
            Self::Split(_, p) => p.in_range(&address, center),
            Self::Leaf(_, p) => p.in_range(&address, center),
        }
    }

    /// Returns true if the Element is a Leaf. This will be used by
    /// the collaps / remove functions to verify they are operating on
    /// the bottom most elements.
    fn is_leaf(&self) -> bool {
        match self {
            Self::Split(_, _) => false,
            Self::Leaf(_, _) => true,
        }
    }
}

impl Split {
    /// Recursive function that calls try_add on the "near" or "far"
    /// side the Node belongs to.
    fn try_add(&mut self, node: Node, center: &Center) -> Result<(), Error> {
        if self.near.in_range(&node.address, center) {
            self.near.try_add(node, center)
        } else {
            self.far.try_add(node, center)
        }
    }

    /// Recursive function that calls add on the "near" or "far" side
    /// the Node belongs to.
    fn add(&mut self, node: Node, center: &Center) {
        if self.near.in_range(&node.address, center) {
            self.near.add(node, center)
        } else {
            self.far.add(node, center)
        }
    }

    /// Recursive function that calls find on the correct side for the
    /// Address.
    fn find(&self, search: &Address, center: &Center) -> Option<&Node> {
        if self.near.in_range(search, center) {
            self.near.find(search, center)
        } else {
            self.far.find(search, center)
        }
    }

    /// Recursive function that calls find on the correct side for the
    /// Address.
    fn find_mut(&mut self, search: &Address, center: &Center) -> Option<&mut Node> {
        if self.near.in_range(search, center) {
            self.near.find_mut(search, center)
        } else {
            self.far.find_mut(search, center)
        }
    }

    /// Recursive function that finds the "limit" number of closest
    /// nodes to a given Address. It tries get all of them from the
    /// element the target is in but will use both sides if no target
    /// is available.
    fn get(&self, target: &Address, center: &Center, limit: usize) -> Vec<&Node> {
        let mut nodes = Vec::new();
        if self.near.in_range(&target, &center) {
            nodes.append(&mut self.near.get(target, center, limit));
            if nodes.len() >= limit {
                nodes.truncate(limit);
                return nodes;
            } else {
                nodes.append(&mut self.far.get(target, center, limit));
                nodes.truncate(limit);
                return nodes;
            }
        } else {
            nodes.append(&mut self.far.get(target, center, limit));
            if nodes.len() >= limit {
                nodes.truncate(limit);
                return nodes;
            } else {
                nodes.append(&mut self.near.get(target, center, limit));
                nodes.truncate(limit);
                return nodes;
            }
        }
    }

    /// Designated the remove call to the correct Element (near / far).
    fn remove(&mut self, address: &Address, center: &Center) -> Result<(), Error> {
        if self.near.in_range(address, center) {
            self.near.remove(address, center)
        } else {
            self.far.remove(address, center)
        }
    }

    /// Core method for updating the shape of the tree after removing
    /// Elements. It will allocate two new arrays, get all the
    /// Elements from the two Leaf Buckets and then create a new
    /// Bucket & Element with the combined array. It will check if the
    /// total length is exceeded and only fail not both of the
    /// Elements are Leafs.
    fn collapse(&self) -> Result<Element, Error> {
        let mut nodes = Vec::new();
        let lower;
        let upper;
        let limit;
        if let Element::Leaf(b, p) = &*self.near {
            nodes.append(&mut b.get(b.capacity()));
            lower = p.lower;
            limit = b.capacity();
        } else {
            return Err(Error::Unknown);
        }
        if let Element::Leaf(b, p) = &*self.far {
            nodes.append(&mut b.get(b.capacity()));
            upper = p.upper;
        } else {
            return Err(Error::Unknown);
        }
        if nodes.len() > limit {
            return Err(Error::Unknown);
        }
        let mut bucket = Bucket::new(limit);
        for i in nodes.into_iter() {
            bucket.add(i.clone());
        }
        let prop = Property { lower, upper };
        Ok(Element::Leaf(bucket, prop))
    }

    /// Sums up the length of all Elements below the Split recursivly.
    fn len(&self) -> usize {
        let mut length = self.far.len();
        match &*self.near {
            Element::Leaf(b, _) => length += b.len(),
            Element::Split(s, _) => length += s.len(),
        }
        return length;
    }

    /// Sums up the capacity of all Elements below the Split
    /// recursivly.
    fn capacity(&self) -> usize {
        let mut sum = self.near.capacity();
        sum += self.far.capacity();
        return sum;
    }

    fn is_final(&self) -> bool {
        self.near.is_leaf() && self.far.is_leaf()
    }
}

impl Property {
    /// Determines whether an address is within range of the given
    /// Property. It does this by calculating the XOR Distance between
    /// the Node and the Center. If the first significant byte falls
    /// within the range it will return true.
    fn in_range(&self, address: &Address, center: &Center) -> bool {
        let index = (address.clone() ^ center.public.clone())[0];
        self.lower <= index && self.upper > index
    }

    /// Splits the Property of an Element. Unlike the similar function
    /// for Elements this will not return one object or modify an
    /// existing one, instead it will return two dedicated properties
    /// as a tuple with the first one being the "near" Property and
    /// the last one being the "far" Property.
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

    /// Simply checks if the lower property is zero, which means the
    /// Element is "near".
    fn is_near(&self) -> bool {
        self.lower == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

    #[test]
    fn test_full_duplicate() {
        let b = gen_bucket();
        let p = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(b, p);
        let center = gen_center();

        for i in 0..40 {
            elem.add(gen_node(&i.to_string()), &center);
        }

        assert_eq!(elem.len(), 40);

        for i in 0..40 {
            let _ = elem.add(gen_node(&i.to_string()), &center);
        }

        assert_eq!(elem.len(), 40);
    }

    #[test]
    fn test_full_stress() {
        let b = gen_bucket();
        let p = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(b, p);
        let center = gen_center();

        for i in 0..1000 {
            elem.add(gen_node(&i.to_string()), &center);
        }

        for i in 100..1100 {
            let _ = elem.remove(&gen_node(&i.to_string()).address, &center);
        }

        for i in 0..1000 {
            elem.add(gen_node(&i.to_string()), &center);
        }

        for i in 100..1100 {
            let _ = elem.remove(&gen_node(&i.to_string()).address, &center);
        }

        let a = elem.len() <= elem.capacity();

        assert_eq!(a, true);
    }

    #[test]
    fn test_property_in_range() {
        let p = Property {
            lower: 0,
            upper: 255,
        };
        let node = gen_node_near();
        let center = gen_center_near();
        assert_eq!(p.in_range(&node.address, &center), true);
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
    fn test_element_try_add() {
        let bucket = Bucket::new(1);
        let prop = Property {
            lower: 0,
            upper: 63,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let node = gen_node_near();
        let center = gen_center_near();
        elem.add(node, &center);

        let node = gen_node_far();
        let s = elem.try_add(node, &center);
        assert_eq!(s.is_err(), true);
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
    fn test_element_split() {
        let bucket = Bucket::new(1);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let center = gen_center_near();
        let node = gen_node_near();
        elem.add(node, &center);

        let node = gen_node_far();
        elem.add(node, &center);

        assert_eq!(elem.len(), 2);
        match elem {
            Element::Split(s, _) => {
                assert_eq!(s.len(), 2);
                assert_eq!(s.near.len(), 1);
                assert_eq!(s.far.len(), 1);
            }
            Element::Leaf(_, _) => assert_eq!("split failed", ""),
        }
    }

    #[test]
    fn test_element_split_near() {
        let bucket = Bucket::new(1);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let center = gen_center_near();
        let node = gen_node_near();
        elem.add(node, &center);

        let node = gen_node_near();
        elem.add(node, &center);

        assert_eq!(elem.len(), 1);
        match elem {
            Element::Split(s, _) => {
                assert_eq!(s.len(), 1);
                assert_eq!(s.near.len(), 1);
                assert_eq!(s.far.len(), 0);
            }
            Element::Leaf(_, _) => assert_eq!("split failed", ""),
        }
    }

    #[test]
    fn test_element_find_top() {
        let bucket = Bucket::new(20);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let center = gen_center_near();

        let node = gen_node("searching");
        let searching = node.address.clone();
        elem.add(node, &center);

        assert_eq!(elem.len(), 1);
        let node = elem.find(&searching, &center).unwrap();
        assert_eq!(node.address, searching);
    }

    #[test]
    fn test_element_find_deep() {
        let split = Split {
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

        let props = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Split(split, props);
        let center = gen_center_near();

        let node = gen_node("searching");
        let searching = node.address.clone();
        elem.add(node, &center);

        assert_eq!(elem.len(), 1);
        let node = elem.find(&searching, &center).unwrap();
        assert_eq!(node.address, searching);
    }

    #[test]
    fn test_element_get_top() {
        let bucket = Bucket::new(20);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);
        let center = gen_center_near();

        let node = gen_node("searching");
        elem.add(node, &center);
        let node = gen_node("random");
        elem.add(node, &center);
        let node = gen_node("string");
        elem.add(node, &center);
        let node = gen_node("actaeon");
        elem.add(node, &center);
        let node = gen_node("data");
        elem.add(node, &center);

        let target = gen_node("target").address;
        let targets = elem.get(&target, &center, 5);
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_element_get_empty() {
        let bucket = Bucket::new(20);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let elem = Element::Leaf(bucket, prop);
        let center = gen_center_near();
        let target = gen_node("target").address;
        let targets = elem.get(&target, &center, 5);
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn test_element_get_deep() {
        let split = Split {
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

        let props = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Split(split, props);
        let center = gen_center_near();

        let node = gen_node("searching");
        elem.add(node, &center);
        let node = gen_node("random");
        elem.add(node, &center);
        let node = gen_node("string");
        elem.add(node, &center);
        let node = gen_node("actaeon");
        elem.add(node, &center);
        let node = gen_node("data");
        elem.add(node, &center);

        let node = gen_node("searching2");
        elem.add(node, &center);
        let node = gen_node("random2");
        elem.add(node, &center);
        let node = gen_node("string2");
        elem.add(node, &center);
        let node = gen_node("actaeon2");
        elem.add(node, &center);
        let node = gen_node("maybe use a loop for this?");
        elem.add(node, &center);

        let target = gen_node("target").address;
        let targets = elem.get(&target, &center, 5);
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_element_remove_root() {
        let bucket = Bucket::new(20);
        let prop = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Leaf(bucket, prop);

        let center = gen_center();

        let node = gen_node("test");
        elem.add(node, &center);

        assert_eq!(elem.len(), 1);

        let node = gen_node("test");
        let _ = elem.remove(&node.address, &center);
        assert_eq!(elem.len(), 0);
    }

    #[test]
    fn test_element_remove_deep() {
        let split = Split {
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

        let props = Property {
            lower: 0,
            upper: 255,
        };
        let mut elem = Element::Split(split, props);
        let center = gen_center_near();

        let node = gen_node("searching");
        elem.add(node, &center);
        let node = gen_node("random");
        elem.add(node, &center);
        let node = gen_node("string");
        elem.add(node, &center);
        let node = gen_node("actaeon");
        elem.add(node, &center);
        let node = gen_node("data");
        elem.add(node, &center);

        let node = gen_node("searching2");
        elem.add(node, &center);
        let node = gen_node("random2");
        elem.add(node, &center);
        let node = gen_node("string2");
        elem.add(node, &center);
        let node = gen_node("actaeon2");
        elem.add(node, &center);
        let node = gen_node("maybe use a loop for this?");
        elem.add(node, &center);

        let target = gen_node("random");
        assert_eq!(elem.len(), 10);
        let _ = elem.remove(&target.address, &center);
        assert_eq!(elem.len(), 9);
    }

    #[test]
    fn test_element_remove_collaps() {
        let split = Split {
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

        let props = Property {
            lower: 0,
            upper: 255,
        };

        let mut elem = Element::Split(split, props);
        let center = gen_center_near();

        for i in 0..40 {
            elem.add(gen_node(&i.to_string()), &center);
        }

        assert_eq!(elem.len(), 40);

        for i in 0..40 {
            let _ = elem.remove(&gen_node(&i.to_string()).address, &center);
        }

        assert_eq!(elem.len(), 0);

        if let Element::Leaf(b, _) = elem {
            assert_eq!(b.len(), 0);
        }
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
        assert_eq!(split.near.in_range(&node.address, &center), true);
    }

    #[test]
    fn test_split_collaps_working() {
        let mut split = gen_split();
        let center = gen_center_near();
        let node = gen_node("first");
        split.add(node, &center);
        let node = gen_node("second");
        split.add(node, &center);
        let node = gen_node_far();
        split.add(node, &center);
        let node = gen_node_near();
        split.add(node, &center);
        assert_eq!(split.len(), 4);
        let e = split.collapse().unwrap();
        assert_eq!(e.len(), 4);
    }

    #[test]
    fn test_split_in_range_far() {
        let split = gen_split();
        let center = gen_center_near();
        let node = gen_node_far();
        assert_eq!(split.near.in_range(&node.address, &center), false);
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
