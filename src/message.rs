//! # Body
//!
//! The actual contents being sent over the network. This module
//! handles encoding and decoding, as well as crypto related
//! functions.

use crate::node::Address;
use crate::transaction::{Class, Seed};

/// Represents a single message, but not the Wire format. It will
/// mostly be accessed by the Transaction object.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Message {
    /// Type / Class of the message, ensures only messages intended
    /// for the user reach him.
    pub class: Class,
    /// Who sent the message (not this network packet, but the key of
    /// the node that initiated the message).
    pub source: Address,
    /// Receiver of the message, might be a never before seen Address.
    pub target: Address,
    /// Since each message is encrypted a nonce needs to be sent
    /// along. It will be used to parse the body and should not be
    /// read / used by the user. The poly1305 is represented as a
    /// dedicated struct to allow for easier conversion.
    pub seed: Seed,
    /// The actual data of the message, currently just represented as
    /// a vector of bytes. This might later get replaced by a trait
    /// object to allow for smarter custom data formats.
    pub body: Body,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Body {
    is_plain: bool,
    bytes: Vec<u8>,
}

impl Message {
    /// Construct a new message manually with all fields. This
    /// function requires all values to be already present and does no
    /// conversion on any of them, it simply combines them into the
    /// object.
    pub fn new(class: Class, source: Address, target: Address, seed: Seed, body: Vec<u8>) -> Self {
        Self {
            class,
            source,
            target,
            seed,
            // TODO: enc
            body: Body::new(body),
        }
    }
}

impl Body {
    /// Creates the body with the given bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            is_plain: true,
            bytes,
        }
    }

    /// Returns the bytes currently in the body without chaning
    /// encryption.
    pub fn as_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
