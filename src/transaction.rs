//! # Transactions
//!
//! A transaction represents any action on the network. The object
//! contains metadata and the message sent over the wire. Incoming
//! messages will also be parsed into transactions.
//!
//! The difference between a message and a transaction is that getting
//! a transaction from one node to another might utilize multiple
//! messages. Each transaction will have a unique ID and duplicates
//! will be filtered out automatically. This should increase
//! redundancy and possible speed while mostly avoiding duplicate
//! actions or lost transactions. Some form of recent cache will be
//! required to check for duplicate messages.

use crate::error::Error;
use crate::router::address::Address;
use std::time::SystemTime;
use uuid::Uuid;

/// The main object users will be interacting with to handle messages
/// and events.
pub struct Transaction {
    /// Unique ID to avoid duplicate processing.
    uuid: Uuid,
    /// The time a message was received and processed, useful if
    /// non-blocking try_read/0 is used and outdated messages need to
    /// be disregarded.
    processed: SystemTime,
    /// The actual message (not just the body but also connection data).
    message: Message,
}

/// Represents a single message, bu not the Wire format. It will
/// mostly be accessed by the Transaction object.
pub struct Message {
    /// Type / Class of the message, ensures only messages intended
    /// for the user reach him.
    class: Class,
    /// Who sent the message (not this network packet, but the key of
    /// the node that initiated the message).
    source: Address,
    /// Receiver of the message, might be a never before seen Address.
    target: Address,
    /// The actual data of the message, currently just represented as
    /// an owned string. Might later get replaced by a trait object to
    /// allow for smarter custom data formats.
    body: String,
}

/// The Transaction and Message data will be converted into "Wire" and
/// serialized. This struct contains fields from both objects and will
/// be decontructed at the receiving end.
///
/// Wire format:
/// 1 byte: Class,
/// 32 bytes: Source,
/// 32 bytes: Target,
/// 16 bytes: UUID,
/// .. bytes: Body
///
/// Minimum data size: 81 bytes
pub struct Wire {
    uuid: Uuid,
    class: Class,
    source: Address,
    target: Address,
    body: String,
}

/// Each message has a type or function. Since "type" is a reserved
/// keyword this is referred to as "Class". In the future this will be
/// expanded to custom types using a trait. The class will be
/// serialized to a single byte and parsed using a simple lookup
/// table.
///
/// The current hard coded Classes:
/// 0: Ping => Node Alive Check, internal.
/// 1: Lookup => Node ID lookup, internal.
/// 2: Action => User messages, custom.
#[derive(Eq, PartialEq, Debug)]
pub enum Class {
    /// Internal IsAlive check
    Ping,
    /// Internal NodeID lookup
    Lookup,
    /// Messages for the user
    Action,
}

impl Class {
    /// The class is serialized as a single byte, this function
    /// converts that to the object using a simple lookup table.
    fn parse(raw: [u8; 1]) -> Self {
        match raw[0] {
            0 => Self::Ping,
            1 => Self::Lookup,
            2 => Self::Action,
            _ => panic!("Error"),
        }
    }
}

impl Wire {
    /// Convert raw bytes coming from the network into a Wire object.
    /// This will not parse them into a transaction, since sone
    /// decisions can already be made without it. It currently takes a
    /// Vector of bytes, in the future just referencing the array
    /// would be better.
    pub fn from_raw(raw: Vec<u8>) -> Result<Self, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parse_class() {
        assert_eq!(Class::parse([0]), Class::Ping);
    }
}
