//! # Router Address
//!
//! Kademlia style addresses to be used in the XOR distance metric.

// The actual address is calcuated through the hash of the public key of the node.
// This ensures the key is valid, otherwise the data would be wrongly encrypted.
// All fields are private since they should not be changable by the user, only with the key.

use openssl::sha;

#[derive(Clone, Debug)]
pub struct Address<'a> {
    bytes: [u8; 32],
    pub public: &'a str,
}

impl<'a> Address<'a> {
    pub fn new(public: &'a str) -> Self {
        let hash = sha::sha256(public.as_bytes());
        Self {
            bytes: hash,
            public: public,
        }
    }

    pub fn distance(&self, source: &Self) -> [u8; 32] {
        let mut d: [u8; 32] = [0; 32];
        for i in 0..(self.bytes.len()) {
            d[i] = self.bytes[i] ^ source.bytes[i];
        }
        return d;
    }
}
