//! # Node
//!
//! Datastructures and functions related to representing members of the
//! network and how to connect to them.
//!
//! The functionality is split into two core areas:
//!
//! - Address: The kademlia-like routing details for finding a node in the
//!   decentralized routing system.
//!
//! - Connection: How to establish a connection to a node using direct
//!   TCP/UDP connections. In the future this will have to be
//!   modularized further to allow for different transport layers and
//!   protocol. (TODO: Integrate into proxy / indirect system).
//!
//! In addition each node also contains other fields like timestamps
//! and (in the future) a cache of recent messages.

use crate::config::CenterConfig;
use crate::error::Error;
use crate::util;
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::{PublicKey, SecretKey};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::BitXor;
use std::time::SystemTime;

/// Represents a singe Node in the system. It simply stores the
/// (optional) connection details, the routing Address and a
/// timestamp. This does not represent the actual connection to any
/// node, simply information on how to connect to it (both directly
/// and indirectly). The data will get populated over time through a
/// dedicated lookup thread.
#[derive(Clone, Debug, Eq)]
pub struct Node {
    timestamp: SystemTime,
    pub address: Address,
    pub link: Option<Link>,
}

/// Config for self / this node, currently as part of the Node module,
/// might get restructured into a dedicated module in the future,
/// should it increase in scope. It has to be created before all other
/// nodes and stored in the interface.
#[derive(Clone)]
pub struct Center {
    /// The public key / address of this node / self, which gets
    /// automatically generated from the secret key.
    pub public: Address,
    /// The base of the entire object / center calculation. It has to
    /// be stored for encyption but should never be read by anybody
    /// except for the crypto module.
    pub secret: SecretKey,
    /// The time this node was started, used to compare values in the
    /// DRT.
    pub uptime: SystemTime,
    /// User provided (ip finder is planned through signaling)
    /// connection details.
    pub link: Link,
}

/// Routing address based on kademlia keys. Poly1305 public keys are
/// used as the actual addresses, on which distance metrics are
/// implemented.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Address {
    /// sodiumoxide poly1305 public key which is also used to find and
    /// identify nodes.
    pub key: PublicKey,
}

/// Since the term Connection is already used to represent an acitve
/// connection between two nodes the information on how to establish
/// this connection are grouped under the term "Link". Next to the two
/// obvious once, which are currently locked to TCP/IP like values,
/// the public IP addr and the port, there are also two internal
/// fields that represent wheather a node is actually reachable. A
/// simlpe boolean value is used to store the status and a counter
/// will be increased on every attempt, which is supposed to happen
/// periodically until the node has been reached or the number of
/// attempts exceeds a set maximum.
///
/// Currently only IPV4 is supported, but this will have to be updated
/// as soon as possible. Any given IP address must be publicly
/// reachable, proxy modes are not yet supported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    /// IPV4 connection details which will be used by the TCP system
    /// to establish a direct connections.
    pub ip: String,
    /// The port could be represented as just a u16 but is currently
    /// unlimited, since it does not get verified as an acutally
    /// possible port.
    pub port: usize,
    /// Stores wheather a node is acutally reachable, can be
    /// interpreted as a filter for "valid" / possible links and
    /// nodes. Changing it requires the node to be mutable, this might
    /// get replaced by interior mutability in the future.
    pub reachable: bool,
    /// Stores the nuber of attemps that have been made to connect to
    /// a node. Once it exceeds a limit the link / node will be
    /// discarded.
    attempts: usize,
}

impl Node {
    /// Creates a new Node with the current timestamp. The Link can be
    /// None but should be provided.
    pub fn new(address: Address, link: Option<Link>) -> Self {
        Self {
            address,
            timestamp: SystemTime::now(),
            link,
        }
    }

    /// Returns the link status of a node. Should no link be available
    /// it is treated as if the node is unavailable.
    pub fn is_reachable(&self) -> bool {
        match &self.link {
            Some(link) => link.reachable,
            None => false,
        }
    }

    /// Update the link status of a node even if there is no link
    /// available.
    pub fn update(&mut self, value: bool) {
        match &mut self.link {
            Some(link) => link.update(value),
            None => {}
        }
    }

    /// A shorthand for a (mostly useless) empty zero Node with an
    /// invalid timestamp.
    pub fn default() -> Node {
        let bytes = [0; 32];
        let address = Address::from_bytes(bytes);
        Node {
            address,
            link: None,
            timestamp: SystemTime::UNIX_EPOCH,
        }
    }

    /// Converts a Node into a sendable Vec.
    pub fn as_bytes(&self) -> Vec<u8> {
        match &self.link {
            Some(link) => {
                let mut link = link.as_bytes().to_vec();
                let mut data = util::compute_length(&link).to_vec();
                data.append(&mut self.address.as_bytes().to_vec());
                data.append(&mut link);
                return data;
            }
            None => {
                let mut data = vec![0, 0];
                data.append(&mut self.address.as_bytes().to_vec());
                return data;
            }
        }
    }

    /// Turns the bytes back into a Node object. Currently this
    /// function can't fail, if the given data is invalid the default
    /// (empty) Node gets returned.
    pub fn from_bytes(mut bytes: Vec<u8>) -> Result<Node, Error> {
        if bytes.len() < 32 {
            Err(Error::Invalid(String::from("node address is not valid")))
        } else if bytes.len() == 32 {
            let address = Address::from_slice(&bytes)?;
            Ok(Node::new(address, None))
        } else if bytes.len() == 34 {
            let addr = bytes.split_off(2);
            let addr = Address::from_slice(&addr)?;
            Ok(Node::new(addr, None))
        } else {
            let mut length = [0; 2];
            let mut addr = [0; 32];
            let mut link = Vec::new();
            for (i, j) in bytes.iter().enumerate() {
                if i <= 1 {
                    length[i] = *j;
                } else if i >= 2 && i <= 33 {
                    addr[i - 2] = *j;
                } else {
                    link.push(*j);
                }
            }
            let address = Address::from_bytes(addr);
            let link = Link::from_bytes(link)?;
            Ok(Node::new(address, Some(link)))
        }
    }

    /// Parses a convertet Vec of serialized nodes, most likely from a bootstrap
    /// response, into a Vec of actual Nodes.
    pub fn from_bulk(bytes: Vec<u8>) -> Vec<Node> {
        if bytes.len() < 34 {
            return Vec::new();
        } else if bytes.len() == 34 {
            if let Ok(node) = Node::from_bytes(bytes.clone()) {
                return vec![node];
            }
        }
        Node::recursive_parse(bytes)
    }

    /// Used to recursively walk through the bytes and convert them
    /// into Nodes. This has to be done, since the length of the Link
    /// can't be known at compile time and is encoded at
    /// serialization.
    fn recursive_parse(data: Vec<u8>) -> Vec<Node> {
        if data.len() == 0 {
            return Vec::new();
        }
        let mut len = [0; 2];
        let mut address = [0; 32];
        let mut link = Vec::new();
        let mut rest = Vec::new();
        for (i, j) in data.iter().enumerate() {
            if i < 2 {
                len[i] = *j;
            } else if i >= 2 && i <= 33 {
                address[i - 2] = *j;
            } else if i >= 34 && i <= (34 + util::integer(len) - 1) {
                link.push(*j);
            } else {
                rest.push(*j);
            }
        }
        let addr = Address::from_bytes(address);
        if let Ok(link) = Link::from_bytes(link) {
            let node = Node::new(addr, Some(link));
            let mut nodes = vec![node];
            nodes.append(&mut Node::recursive_parse(rest));
            return nodes;
        }
        return Node::recursive_parse(rest);
    }
}

impl Ord for Node {
    /// Node Ordering is implemented based on the timestamps. THe
    /// comparison could fail (for example if the system time is
    /// invalid / before UNIX), it will simply unwrap and panic.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .timestamp
            .elapsed()
            .unwrap()
            .cmp(&self.timestamp.elapsed().unwrap())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl Center {
    /// Creates the center from the provided secret key (the user is
    /// responsible for providing this). The public key and address
    /// get generated from the secret and the current time is stored
    /// for the router.
    pub fn new(secret: SecretKey, ip: String, port: usize) -> Self {
        Self {
            public: Address::new(secret.public_key()),
            secret,
            uptime: SystemTime::now(),
            link: Link::new(ip, port),
        }
    }

    /// Create a Center instance from the CenterConfig.
    pub fn from_config(config: CenterConfig) -> Result<Self, Error> {
        match config.secret {
            Some(bytes) => match SecretKey::from_slice(&bytes) {
                Some(key) => Ok(Self::new(key, config.ip, config.port)),
                None => Err(Error::Config(String::from("invalid config"))),
            },
            None => Err(Error::Config(String::from("invalid config"))),
        }
    }
}

impl Address {
    /// Create a new Address from a public key. (currently the only
    /// field so it could be created manually.) This is usally not
    /// meant to be used by the user / externally.
    pub fn new(public: PublicKey) -> Self {
        Self { key: public }
    }

    /// Most of the times addresses will be created from bytes coming
    /// over the network, this function can be used and won't fail if
    /// the length is guaranteed.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            key: PublicKey::from_slice(&bytes).unwrap(),
        }
    }

    /// Most of the times addresses will be created from bytes coming
    /// over the network, this function can be used, although it might
    /// fail if the key is invalid. Mostly the same as from_bytes/1
    /// but only takes a reference.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        if let Some(public) = PublicKey::from_slice(bytes) {
            Ok(Self { key: public })
        } else {
            Err(Error::Invalid(String::from("public key is invalid")))
        }
    }

    /// Convert an array of bytes into an array of Addresses. Any
    /// invalid ones will be dropped.
    pub fn from_bulk(data: Vec<u8>) -> Vec<Address> {
        let mut ret = Vec::new();
        data.chunks_exact(32)
            .for_each(|x| match Address::from_slice(x) {
                Ok(address) => ret.push(address),
                Err(e) => {
                    log::warn!("received invalid Addres data: {:?}", e);
                }
            });
        return ret;
    }

    /// Returns an array of bytes of the public key / address.
    /// Currently it does not return a slice or reference to the
    /// bytes, instead it creates a new array. This should make it
    /// easier to use it in order to create Wire objects, which might
    /// live longer than the node / address.
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes: [u8; 32] = [0; 32];
        let key = self.key.as_ref();
        for (i, j) in key.into_iter().enumerate() {
            bytes[i] = *j;
        }
        return bytes;
    }

    /// Instead of creating a new array of fixed length this simply
    /// returns a pointer to the bytes stored in the PublicKey as a
    /// pointer.
    pub fn as_slice(&self) -> &[u8] {
        &self.key.0[..]
    }

    /// If a "random" Address is required this can generate a public key
    /// from an input string by hashing it.
    pub fn generate(source: &str) -> Self {
        let bytes = blake3::hash(source.as_bytes()).as_bytes().to_owned();
        Address::from_bytes(bytes)
    }

    /// Since the bucket ID (first byte of distance) is dependant on
    /// the distance from the Center it has to be computed. Currently
    /// this function uses as_bytes()/0 on both addresses, which
    /// creates new arrays for both. In the future this will have to
    /// be replaced with two different methods to reduce the memory
    /// footprint.
    pub fn bucket(&self, center: &Center) -> usize {
        (self.as_bytes()[0] ^ center.public.as_bytes()[0]).into()
    }

    /// Generate a random Address
    pub fn random() -> Address {
        let mut bytes = [0; 32];
        for i in 0..31 {
            bytes[i] = rand::random::<u8>();
        }
        Address::from_bytes(bytes)
    }

    /// Generates a new "zero" Address with all bytes being 0.
    pub fn default() -> Address {
        Address::from_bytes([0; 32])
    }
}

impl BitXor for Address {
    type Output = [u8; 32];

    /// Instead of a custom "distance method" the XOR operation itself
    /// is implemented on addresses. This makes it easier to use in
    /// any situation. Since as_bytes/0 currently returns new bytes
    /// instead of pointers the conversion is done only once and a new
    /// array is returned as well.
    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut bytes: [u8; 32] = [0; 32];
        let source = rhs.as_bytes();
        let target = self.as_bytes();
        for i in 0..31 {
            bytes[i] = target[i] ^ source[i];
        }
        return bytes;
    }
}

impl BitXor for &Address {
    type Output = [u8; 32];

    /// Instead of a custom "distance method" the XOR operation itself
    /// is implemented on addresses. This makes it easier to use in
    /// any situation. Since as_bytes/0 currently returns new bytes
    /// instead of pointers the conversion is done only once and a new
    /// array is returned as well.
    fn bitxor(self, rhs: &Address) -> Self::Output {
        let mut bytes: [u8; 32] = [0; 32];
        let source = rhs.as_bytes();
        let target = self.as_bytes();
        for i in 0..31 {
            bytes[i] = target[i] ^ source[i];
        }
        return bytes;
    }
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

/// Since sometimes the user has to interact with Addresses directly,
/// it might be easier to use a trait object, so that a number of
/// different types can be used to create Addresses.
pub trait ToAddress {
    /// Returns a new Address created from the input.
    fn to_address(&self) -> Address;
}

impl ToAddress for String {
    fn to_address(&self) -> Address {
        let bytes = blake3::hash(self.as_bytes()).as_bytes().to_owned();
        Address::from_bytes(bytes)
    }
}

impl ToAddress for [u8; 32] {
    fn to_address(&self) -> Address {
        Address::from_bytes(*self)
    }
}

impl ToAddress for usize {
    fn to_address(&self) -> Address {
        let mut bytes = [0; 32];
        let conv = self.to_be_bytes();
        for (i, j) in conv.iter().enumerate() {
            bytes[i] = *j;
        }
        Address::from_bytes(bytes)
    }
}

impl Link {
    /// Creates new connection details (Link). It sets both the
    /// reachable and attempts values to teh default.
    pub fn new(ip: String, port: usize) -> Self {
        Self {
            ip,
            port,
            reachable: false,
            attempts: 0,
        }
    }

    /// Returns a new String of the connection details, usable by the
    /// TCP handler. (This still doesn't validtate the values, it
    /// simply concats them. There is no guarantee it will be usable
    /// by IpV4.)
    pub fn to_string(&self) -> String {
        let elements = [self.ip.clone(), self.port.to_string()];
        elements.join(":")
    }

    /// This single function can be used to both incease the count of
    /// the attempts and set it as true should it has been reached.
    /// The counter will currently not be reset if the status is true,
    /// this might help to sort out unreliable nodes.
    pub fn update(&mut self, status: bool) {
        self.attempts += 1;
        self.reachable = status;
    }

    /// Exports the link details to bytes that can be sent over the
    /// wire. Structure:
    /// IP Address data,
    /// Last 8 bytes: Port number
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let address = self.ip.as_bytes();
        let port = self.port.to_le_bytes();
        data.append(&mut address.to_vec());
        data.append(&mut port.to_vec());
        return data;
    }

    pub fn from_bytes(mut data: Vec<u8>) -> Result<Link, Error> {
        data.reverse();
        let mut address = data.split_off(8);
        data.reverse();
        address.reverse();
        let ip = String::from_utf8(address)?;
        let mut port_bytes = [0; 8];
        for (i, j) in data.iter().enumerate() {
            port_bytes[i] = *j;
        }
        let port = u64::from_le_bytes(port_bytes);
        Ok(Link::new(ip, port as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;

    #[test]
    fn test_address_from_bytes() {
        let (p, _s) = box_::gen_keypair();
        let real = Address::new(p.clone());
        let test = Address::from_bytes(p.0);
        assert_eq!(real.key.0, test.key.0);
    }

    #[test]
    fn test_center_new() {
        let (_, s) = box_::gen_keypair();
        let c = Center::new(s, String::from("abc"), 0);
        assert_ne!(c.public.as_bytes(), [0; 32]);
    }

    #[test]
    fn test_link_new() {
        let l = Link::new("127.0.0.1".to_string(), 42);
        assert_eq!(l.port, 42);
    }

    #[test]
    fn test_link_string() {
        let l = Link::new("127.0.0.1".to_string(), 42);
        assert_eq!(l.to_string(), String::from("127.0.0.1:42"));
    }

    #[test]
    fn test_address_xor() {
        let a1 = Address::generate("test1");
        let a2 = Address::generate("test2");
        assert_ne!(a1 ^ a2, [0; 32]);
    }

    #[test]
    fn test_address_xor_zero() {
        let a = Address::generate("test");
        assert_eq!(a.clone() ^ a, [0; 32]);
    }

    #[test]
    fn test_to_address_bytes() {
        let mut bytes = [0; 32];
        bytes[17] = 42;
        assert_eq!(bytes.clone().to_address(), Address::from_bytes(bytes),);
    }

    #[test]
    fn test_address_form_bytes_zero() {
        let b = [0; 32];
        let a = Address::from_bytes(b);
        let c = a.as_bytes();
        assert_eq!(b, c);
    }

    #[test]
    fn test_link_serialize() {
        let l = Link::new(String::from("127.0.0.1"), 12345);
        let b = l.as_bytes();
        let c = Link::from_bytes(b).unwrap();
        assert_eq!(l, c);
    }

    #[test]
    fn test_link_serialize_more() {
        for i in 100..1000 {
            let l = Link::new(i.to_string(), (i * 14) / 4);
            let b = l.as_bytes();
            let c = Link::from_bytes(b).unwrap();
            assert_eq!(l, c);
        }
    }

    #[test]
    fn test_node_serialize() {
        let link = Link::new(String::from("127.0.0.1"), 12345);
        let node = Node::new(Address::random(), Some(link));
        let serialized = node.as_bytes();
        let deserialized = Node::from_bytes(serialized).unwrap();
        assert_eq!(deserialized, node);
    }

    #[test]
    fn test_address_random() {
        assert_ne!(Address::random(), Address::random());
    }

    #[test]
    fn test_node_length() {
        let l = Link::new("192.168.1.42".to_string(), 2424);
        let node = Node::new(Address::random(), Some(l.clone()));
        let ser = node.as_bytes();
        let len = util::compute_length(&l.as_bytes());
        assert_eq!(ser[0..1], len[0..1]);
    }

    #[test]
    fn test_node_bulk() {
        let mut bytes = Vec::new();
        let nodes = vec![gen_node(5), gen_node(5), gen_node(5)];
        nodes.iter().for_each(|x| {
            bytes.append(&mut x.as_bytes());
        });

        let re = Node::from_bulk(bytes);
        assert_eq!(nodes, re);
    }

    #[test]
    fn test_node_bulk_random() {
        let mut bytes = Vec::new();
        let mut nodes = Vec::new();
        for i in 42..142 {
            nodes.push(gen_node(i));
        }
        nodes.iter().for_each(|x| {
            bytes.append(&mut x.as_bytes());
        });
        let re = Node::from_bulk(bytes);
        assert_eq!(nodes, re);
    }

    fn gen_node(len: usize) -> Node {
        let addr = Address::random();
        let mut ip = Vec::new();
        for _ in 0..len {
            ip.push(rand::random::<char>().to_string());
        }
        let ip = ip.join(".");
        let link = Link::new(ip, rand::random());
        Node::new(addr, Some(link))
    }
}
