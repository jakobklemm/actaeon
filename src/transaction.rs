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
use crate::message::{Message, Seed};
use crate::node::Address;
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

/// The Transaction and Message data will be converted into "Wire" and
/// serialized. This struct contains fields from both objects and will
/// be decontructed at the receiving end.
///
/// Wire format:
/// 04 bytes: Class,
/// 32 bytes: Source,
/// 32 bytes: Target,
/// 16 bytes: UUID,
/// 24 bytes: Nonce,
/// .. bytes: Body,
///
/// Minimum data size: 108 bytes (+ body).
#[derive(Debug)]
pub struct Wire {
    uuid: [u8; 16],
    class: [u8; 4],
    source: [u8; 32],
    target: [u8; 32],
    nonce: [u8; 24],
    body: Vec<u8>,
}

/// Each message has a type or function. Since "type" is a reserved
/// keyword this is referred to as "Class". In the future this will be
/// expanded to custom types using a trait. The class will be
/// serialized to a single byte and parsed using a simple lookup
/// table.
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Class {
    /// Internal IsAlive check
    Ping,
    /// Internal NodeID lookup
    Lookup,
    /// Messages for the user
    Action,
    /// Subscribe to another topic
    Subscribe,
    /// A new subscriber was added
    Subscribed,
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
        wire.convert()
    }

    /// Converts a Transaction into a Wire object. Currently this
    /// function uses clone on most fields in order to convert between
    /// the types without having to take ownership. In the future this
    /// might get changed or a second function will get added, which
    /// uses fewer allocations.
    pub fn to_wire(&self) -> Wire {
        Wire {
            uuid: *self.uuid.as_bytes(),
            class: self.message.class.as_bytes(),
            source: self.message.source.as_bytes(),
            target: self.message.target.as_bytes(),
            nonce: self.message.seed.as_bytes(),
            body: self.message.body.clone().as_bytes(),
        }
    }

    /// Convert a Transaction into bytes, to be sent over the wire.
    /// This is a shortcut without interfacing with the Wire struct
    /// directly.
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_wire().as_bytes()
    }

    /// Returns the Address of target of a message. This is simply a
    /// shorthand function for reading the correct field but it
    /// ensures privacy.
    pub fn target(&self) -> Address {
        self.message.target.clone()
    }

    /// Returns the Address of source of a message. This is simply a
    /// shorthand function for reading the correct field but it
    /// ensures privacy.
    pub fn source(&self) -> Address {
        self.message.source.clone()
    }

    /// This function returns the duration since the Transaction was
    /// created. While it should mostly be without problems, it can
    /// fail if the OS clock is unreliable.
    pub fn age(&self) -> Result<Duration, Error> {
        match self.created.elapsed() {
            Ok(time) => Ok(time),
            Err(_) => Err(Error::System(String::from("transaction time is invalid"))),
        }
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

impl PartialEq for Transaction {
    fn eq(&self, rhs: &Self) -> bool {
        self.uuid == rhs.uuid
    }
}

impl Eq for Transaction {}

impl Class {
    /// The class is serialized as a single byte, this function
    /// converts that to the object using a simple lookup table.
    fn from_bytes(raw: [u8; 4]) -> Result<Self, Error> {
        match raw {
            [0, 0, 0, 1] => Ok(Self::Ping),
            [0, 0, 0, 2] => Ok(Self::Lookup),
            [0, 0, 0, 3] => Ok(Self::Action),
            [0, 0, 0, 4] => Ok(Self::Subscribe),
            [0, 0, 0, 5] => Ok(Self::Subscribed),
            _ => Err(Error::Invalid(String::from("class serlaization invalid"))),
        }
    }

    /// Converts the Class enum into a single u8 byte. Currently the
    /// Class lookup table is duplicated in both functions, in the
    /// future it might be smarter to have a single table, should many
    /// more types be added.
    fn as_bytes(&self) -> [u8; 4] {
        match self {
            Self::Ping => [0, 0, 0, 1],
            Self::Lookup => [0, 0, 0, 2],
            Self::Action => [0, 0, 0, 3],
            Self::Subscribe => [0, 0, 0, 4],
            Self::Subscribed => [0, 0, 0, 5],
        }
    }
}

impl Wire {
    /// Convert raw bytes coming from the network into a Wire object.
    /// This will not parse them into a transaction, since sone
    /// decisions can already be made without it. It currently takes a
    /// Vector of bytes, in the future just referencing the array
    /// would be better.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, Error> {
        if raw.len() <= 81 {
            return Err(Error::Invalid(String::from("invalid number of bytes")));
        }

        let mut class: [u8; 4] = [0; 4];
        let mut source: [u8; 32] = [0; 32];
        let mut target: [u8; 32] = [0; 32];
        let mut uuid: [u8; 16] = [0; 16];
        let mut nonce: [u8; 24] = [0; 24];
        let mut body: Vec<u8> = Vec::new();

        for (i, j) in raw.iter().enumerate() {
            // bytes 0..3 = Class, len = 4, offset = 0
            if i <= 3 {
                class[i] = *j;
            }
            // bytes 4..35 = Source, len = 32, offset = 4
            else if i >= 4 && i <= 35 {
                source[i - 4] = *j;
            }
            // bytes 36..67 = Target, len = 32, offset = 36
            else if i >= 36 && i <= 67 {
                target[i - 36] = *j;
            }
            // bytes 68..83 = UUID, len = 16, offset = 68
            else if i >= 68 && i <= 83 {
                uuid[i - 68] = *j;
            }
            // bytes 84..107 = Nonce, len = 24, offset = 84
            else if i >= 84 && i <= 107 {
                nonce[i - 84] = *j;
            } else {
                body.push(*j);
            }
        }

        Ok(Self {
            class,
            source,
            target,
            uuid,
            nonce,
            body,
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
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.append(&mut self.class.to_vec());
        data.append(&mut self.source.to_vec());
        data.append(&mut self.target.to_vec());
        data.append(&mut self.uuid.to_vec());
        data.append(&mut self.nonce.to_vec());
        data.append(&mut self.body.clone());

        return data;
    }

    /// Turns a Wire Object into a Transaction. It constructs a new
    /// Message and Transaction from the data in Wire.
    pub fn convert(self) -> Result<Transaction, Error> {
        let class = Class::from_bytes(self.class)?;
        let source = Address::from_bytes(self.source)?;
        let target = Address::from_bytes(self.target)?;
        let seed = Seed::from_bytes(&self.nonce)?;
        let uuid = Uuid::from_bytes(self.uuid);
        let message = Message::create(class, source, target, seed, self.body);
        Ok(Transaction {
            uuid,
            created: SystemTime::now(),
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_parse() {
        assert_eq!(Class::from_bytes([0, 0, 0, 1]).unwrap(), Class::Ping);
    }

    #[test]
    fn test_class_bytes() {
        assert_eq!(
            Class::from_bytes([0, 0, 0, 1]).unwrap().as_bytes(),
            [0, 0, 0, 1]
        );
    }

    #[test]
    fn test_wire_from_bytes() {
        let data = generate_test_data();
        match Wire::from_bytes(&data) {
            Ok(wire) => {
                assert_eq!(wire.source, Address::generate("abc").unwrap().as_bytes());
                assert_eq!(wire.target, Address::generate("def").unwrap().as_bytes());
                assert_eq!(
                    wire.uuid,
                    Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf")
                        .unwrap()
                        .as_bytes()
                        .to_owned()
                );
                assert_eq!(wire.class, [0, 0, 0, 1]);
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
        assert_eq!(wire.convert().unwrap().message.class, Class::Ping);
    }

    #[test]
    fn test_wire_as_bytes() {
        let data = generate_test_data();
        let wire = Wire::from_bytes(&data).unwrap();
        assert_eq!(wire.as_bytes(), data);
    }

    #[test]
    fn test_transaction_new() {
        let m = Message::create(
            Class::Ping,
            Address::generate("a").unwrap(),
            Address::generate("b").unwrap(),
            Seed::from_bytes(&[0; 24]).unwrap(),
            Vec::new(),
        );
        assert_eq!(Transaction::new(m).to_wire().class, [0, 0, 0, 1]);
    }

    #[test]
    fn test_transaction_age() {
        let m = Message::create(
            Class::Action,
            Address::generate("a").unwrap(),
            Address::generate("b").unwrap(),
            Seed::from_bytes(&[0; 24]).unwrap(),
            Vec::new(),
        );
        let t = Transaction::new(m);
        let d = t.age().unwrap();
        assert_eq!(d > Duration::from_secs(0), true);
    }

    #[test]
    fn test_transaction_as_bytes() {
        let data = generate_test_data();

        let m = Message::create(
            Class::Ping,
            Address::generate("abc").unwrap(),
            Address::generate("def").unwrap(),
            Seed::from_bytes(&[0; 24]).unwrap(),
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
            t.message.source.as_bytes(),
            Address::generate("abc").unwrap().as_bytes()
        );
    }

    #[test]
    fn test_transaction_build() {
        let uuid = Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap();
        let time = SystemTime::now();
        let seed = Seed::from_bytes(&[0; 24]).unwrap();
        let message = Message::create(
            Class::Ping,
            Address::generate("abc").unwrap(),
            Address::generate("def").unwrap(),
            seed,
            "test".to_string().as_bytes().to_vec(),
        );
        let t = Transaction::build(uuid, time, message);
        let d = Transaction::from_bytes(&generate_test_data()).unwrap();
        assert_eq!(t.message, d.message);
    }

    fn generate_test_data() -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();

        data.append(&mut [0, 0, 0, 1].to_vec());

        let source = Address::generate("abc")
            .unwrap()
            .as_bytes()
            .to_owned()
            .to_vec();
        data.append(&mut source.clone());
        let target = Address::generate("def")
            .unwrap()
            .as_bytes()
            .to_owned()
            .to_vec();
        data.append(&mut target.clone());
        let uuid = Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap();
        data.append(&mut uuid.clone().as_bytes().to_vec());

        data.append(&mut [0; 24].to_vec());

        data.append(&mut "test".to_string().into_bytes());
        return data;
    }
}
