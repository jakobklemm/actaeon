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
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// The main object users will be interacting with to handle messages
/// and events.
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Unique ID to avoid duplicate processing.
    uuid: Uuid,
    /// The time a message was received and processed, useful if
    /// non-blocking try_read/0 is used and outdated messages need to
    /// be disregarded.
    created: SystemTime,
    /// The actual message (not just the body but also connection data).
    message: Message,
}

/// Represents a single message, bu not the Wire format. It will
/// mostly be accessed by the Transaction object.
#[derive(Debug, Clone)]
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
    /// a vector of bytes. This might later get replaced by a trait
    /// object to allow for smarter custom data formats.
    body: Vec<u8>,
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
    body: Vec<u8>,
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
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Class {
    /// Internal IsAlive check
    Ping,
    /// Internal NodeID lookup
    Lookup,
    /// Messages for the user
    Action,
}

impl Transaction {
    /// Create a new transaction from an existing message. UUID and
    /// TimeStamp will be set automatically.
    pub fn new(message: Message) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            created: SystemTime::now(),
            message,
        }
    }

    pub fn build(uuid: Uuid, created: SystemTime, message: Message) -> Self {
        Self {
            uuid,
            created,
            message,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let wire = match Wire::from_bytes(&bytes) {
            Ok(data) => data,
            Err(e) => {
                return Err(e);
            }
        };
        Ok(wire.convert())
    }

    /// Converts a Transaction into a Wire object. Currently this
    /// function uses clone on most fields in order to convert between
    /// the types without having to take ownership. In the future this
    /// might get changed or a second function will get added, which
    /// uses fewer allocations.
    fn to_wire(&self) -> Wire {
        Wire {
            uuid: self.uuid,
            class: self.message.class.clone(),
            source: self.message.source.clone(),
            target: self.message.target.clone(),
            body: self.message.body.clone(),
        }
    }

    /// Convert a Transaction into bytes, to be sent over the wire.
    /// This is a shortcut without interfacing with the Wire struct
    /// directly.
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_wire().as_bytes()
    }

    /// This function returns the duration since the Transaction was
    /// created. While it should mostly be without problems, it can
    /// fail if the OS clock is unreliable.
    pub fn age(&self) -> Result<Duration, Error> {
        match self.created.elapsed() {
            Ok(time) => Ok(time),
            Err(_) => Err(Error::System),
        }
    }

    /// Computes the checksum of select fields of the Transaction.
    pub fn verify(&self) -> [u8; 32] {
        let mut data: Vec<u8> = Vec::new();
        data.append(&mut self.uuid.as_bytes().to_vec());
        data.append(&mut self.message.source.serialize().to_vec());
        data.append(&mut self.message.target.serialize().to_vec());
        data.append(&mut self.message.body.clone());
        *blake3::hash(data.as_slice()).as_bytes()
    }
}

impl Eq for Transaction {}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.verify() == other.verify()
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created.cmp(&other.created)
    }
}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl Class {
    /// The class is serialized as a single byte, this function
    /// converts that to the object using a simple lookup table.
    fn parse(raw: u8) -> Self {
        match raw {
            0 => Self::Ping,
            1 => Self::Lookup,
            2 => Self::Action,
            _ => panic!("Error"),
        }
    }

    /// Converts the Class enum into a single u8 byte. Currently the
    /// Class lookup table is duplicated in both functions, in the
    /// future it might be smarter to have a single table, should many
    /// more types be added.
    fn serialize(&self) -> u8 {
        match self {
            Self::Ping => 0,
            Self::Lookup => 1,
            Self::Action => 2,
        }
    }
}

impl Wire {
    /// Convert raw bytes coming from the network into a Wire object.
    /// This will not parse them into a transaction, since sone
    /// decisions can already be made without it. It currently takes a
    /// Vector of bytes, in the future just referencing the array
    /// would be better.
    fn from_bytes(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() <= 81 {
            return Err(Error::Invalid);
        }

        let class = raw[0];
        let mut source: [u8; 32] = [0; 32];
        let mut target: [u8; 32] = [0; 32];
        let mut uuid: [u8; 16] = [0; 16];
        let mut body: Vec<u8> = Vec::new();

        for (i, j) in raw.iter().enumerate() {
            if i == 0 {
                continue;
            } else if i >= 1 && i <= 32 {
                source[i - 1] = *j;
            } else if i >= 33 && i <= 64 {
                target[i - 33] = *j;
            } else if i >= 65 && i <= 80 {
                uuid[i - 65] = *j;
            } else {
                body.push(*j);
            }
        }

        let uuid = match Uuid::from_slice(&uuid) {
            Ok(uuid) => uuid,
            Err(_e) => {
                return Err(Error::Invalid);
            }
        };

        Ok(Self {
            class: Class::parse(class),
            source: Address::from_message(source),
            target: Address::from_message(target),
            uuid: uuid,
            body: body,
        })
    }

    /// Converts a Wire object into the actuall bytes to be sent over
    /// the wire. The function simply pushes the different elements
    /// onto a vector, the only important thing is the order of
    /// commands.
    ///
    /// Currently this function clones the body. It might be more
    /// performant to remove that, but it would requrie a mutable
    /// reference to the Wire object.
    /// TODO: Define mut / clone.
    fn as_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.push(self.class.serialize());
        data.append(&mut self.source.serialize().to_vec());
        data.append(&mut self.target.serialize().to_vec());
        data.append(&mut self.uuid.as_bytes().to_vec());
        data.append(&mut self.body.clone());

        return data;
    }

    /// Turns a Wire Object into a Transaction. It constructs a new
    /// Message and Transaction from the data in Wire.
    pub fn convert(self) -> Transaction {
        let message = Message::new(self.class, self.source, self.target, self.body);
        Transaction {
            uuid: self.uuid,
            created: SystemTime::now(),
            message,
        }
    }
}

impl Message {
    /// Construct a new message manually with all fields. This
    /// function requires all values to be already present and does no
    /// conversion on any of them, it simply combines them into the
    /// object.
    pub fn new(class: Class, source: Address, target: Address, body: Vec<u8>) -> Self {
        Self {
            class,
            source,
            target,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_parse() {
        assert_eq!(Class::parse(0), Class::Ping);
    }

    #[test]
    fn test_class_bytes() {
        assert_eq!(Class::parse(0).serialize(), 0);
    }

    #[test]
    fn test_wire_from_bytes() {
        let data = generate_test_data();
        match Wire::from_bytes(&data) {
            Ok(wire) => {
                assert_eq!(wire.source.serialize(), Address::new("abc").serialize());
                assert_eq!(wire.target.serialize(), Address::new("def").serialize());
                assert_eq!(
                    wire.uuid,
                    Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap()
                );
                assert_eq!(wire.class, Class::Ping);
                assert_eq!(wire.body, "test".to_string().as_bytes())
            }
            Err(_e) => {
                panic!("Message Invalid!")
            }
        }
    }

    #[test]
    fn test_wire_to_transaction() {
        let data = generate_test_data();
        let wire = Wire::from_bytes(&data).unwrap();
        assert_eq!(wire.convert().message.class, Class::Ping);
    }

    #[test]
    fn test_wire_as_bytes() {
        let data = generate_test_data();
        let wire = Wire::from_bytes(&data).unwrap();
        assert_eq!(wire.as_bytes(), data);
    }

    #[test]
    fn test_transaction_new() {
        let m = Message::new(
            Class::Action,
            Address::new("a"),
            Address::new("b"),
            Vec::new(),
        );
        assert_eq!(Transaction::new(m).to_wire().class.serialize(), 2);
    }

    #[test]
    fn test_transaction_age() {
        let m = Message::new(
            Class::Action,
            Address::new("a"),
            Address::new("b"),
            Vec::new(),
        );
        let t = Transaction::new(m);
        let d = t.age().unwrap();
        assert_eq!(d > Duration::from_secs(0), true);
    }

    #[test]
    fn test_transaction_as_bytes() {
        let data = generate_test_data();

        let m = Message::new(
            Class::Ping,
            Address::new("abc"),
            Address::new("def"),
            Vec::new(),
        );
        let t = Transaction::new(m).as_bytes();
        // Since the uuid is random only a few (random) bytes are compared.
        assert_eq!(data[0], t[0]);
        assert_eq!(data[22], t[22]);
        assert_eq!(data[55], t[55]);
    }

    #[test]
    fn test_transaction_from_bytes() {
        let data = generate_test_data();
        let t = Transaction::from_bytes(&data).unwrap();
        assert_eq!(
            t.message.source.serialize(),
            Address::new("abc").serialize()
        );
    }

    #[test]
    fn test_transaction_verify() {
        let d = Transaction::from_bytes(&generate_test_data()).unwrap();
        assert_ne!(d.verify()[0], 42);
    }

    #[test]
    fn test_transaction_build() {
        let uuid = Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap();
        let time = SystemTime::now();
        let message = Message::new(
            Class::Ping,
            Address::new("abc"),
            Address::new("def"),
            "test".to_string().as_bytes().to_vec(),
        );
        let t = Transaction::build(uuid, time, message);
        let d = Transaction::from_bytes(&generate_test_data()).unwrap();
        // This also checks Eq and PartialEq impls
        assert_eq!(t, d);
    }

    fn generate_test_data() -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.push(0);
        let source = Address::new("abc").serialize().to_owned();
        data.append(&mut source.clone());
        let target = Address::new("def").serialize().to_owned();
        data.append(&mut target.clone());
        let uuid = Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap();
        data.append(&mut uuid.clone().as_bytes().to_vec());

        data.append(&mut "test".to_string().into_bytes());
        return data;
    }
}
