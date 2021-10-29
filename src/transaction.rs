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
    pub uuid: Uuid,
    /// The time a message was received and processed, useful if
    /// non-blocking try_read/0 is used and outdated messages need to
    /// be disregarded.
    created: SystemTime,
    /// The actual message (not just the body but also connection data).
    pub message: Message,
}

/// The Transaction and Message data will be converted into "Wire" and
/// serialized. This struct contains fields from both objects and will
/// be decontructed at the receiving end.
///
/// Wire format:
/// 02 bytes: Length,
/// 04 bytes: Class,
/// 32 bytes: Source,
/// 32 bytes: Target,
/// 16 bytes: UUID,
/// 24 bytes: Nonce,
/// .. bytes: Body,
///
/// Minimum data size: 110 bytes (+ body).
#[derive(Debug, PartialEq, Clone)]
pub struct Wire {
    length: [u8; 2],
    pub uuid: [u8; 16],
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
    /// Return of Ping.
    Pong,
    /// Internal NodeID lookup.
    Lookup,
    /// Return value for Lookup calls.
    Details,
    /// Messages coming from a user to the target node.
    Action,
    /// Subscribe to another topic.
    Subscribe,
    /// Opposite of Subscribe.
    Unsubscribe,
    /// Informs subscribers about a new one.
    Subscriber,
    /// Informs subscribers about a unsubscribe message.
    Unsubscriber,
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
            length: self.len(),
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

    /// Returns the Class of the Message / Transaction.
    pub fn class(&self) -> Class {
        self.message.class.clone()
    }

    fn len(&self) -> [u8; 2] {
        self.message.len()
    }

    /// When a message comes from a user to the record location the
    /// source Address should not change from the original node (maybe
    /// this has to be updated in a future version by including a
    /// second source field (source + origin)), only the target has to
    /// be updated for each target. This directly returns a new
    /// Transaction with the updated target that can be delivered.
    pub fn redirect(&self, target: Address) -> Transaction {
        let mut transaction = self.clone();
        transaction.message.target = target;
        return transaction;
    }

    /// Easy way of creating a "mostly primitive" version of the core
    /// relevant fields of a Transaction. Can be used for working on
    /// the received data in other parts of the users applications
    /// without relying on Actaeon structures. It simply returns a
    /// touple of the source of a message with its body.
    pub fn export(&self) -> (Address, Vec<u8>) {
        (self.source(), self.message.body.clone().as_bytes())
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
            [0, 0, 0, 2] => Ok(Self::Pong),
            [0, 0, 1, 0] => Ok(Self::Lookup),
            [0, 0, 1, 1] => Ok(Self::Details),
            [0, 1, 0, 0] => Ok(Self::Subscribe),
            [0, 1, 0, 1] => Ok(Self::Unsubscribe),
            [0, 1, 0, 2] => Ok(Self::Subscriber),
            [0, 1, 0, 3] => Ok(Self::Unsubscriber),
            [1, 0, 0, 1] => Ok(Self::Action),
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
            Self::Pong => [0, 0, 0, 2],
            Self::Lookup => [0, 0, 1, 0],
            Self::Details => [0, 0, 1, 1],
            Self::Subscribe => [0, 1, 0, 0],
            Self::Unsubscribe => [0, 1, 0, 1],
            Self::Subscriber => [0, 1, 0, 2],
            Self::Unsubscriber => [0, 1, 0, 3],
            Self::Action => [1, 0, 0, 1],
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

        let mut length: [u8; 2] = [0; 2];
        let mut class: [u8; 4] = [0; 4];
        let mut source: [u8; 32] = [0; 32];
        let mut target: [u8; 32] = [0; 32];
        let mut uuid: [u8; 16] = [0; 16];
        let mut nonce: [u8; 24] = [0; 24];
        let mut body: Vec<u8> = Vec::new();

        for (i, j) in raw.iter().enumerate() {
            // bytes 0..1 = Length, len = 2, offset = 0
            if i <= 1 {
                length[i] = *j;
            }
            // bytes 2..5 = Class, len = 4, offset = 2
            else if i >= 2 && i <= 5 {
                class[i - 2] = *j;
            }
            // bytes 6..37 = Source, len = 32, offset = 6
            else if i >= 6 && i <= 37 {
                source[i - 6] = *j;
            }
            // bytes 38..69 = Target, len = 32, offset = 38
            else if i >= 38 && i <= 69 {
                target[i - 38] = *j;
            }
            // bytes 70..85 = UUID, len = 16, offset = 70
            else if i >= 70 && i <= 85 {
                uuid[i - 70] = *j;
            }
            // bytes 86..109 = Nonce, len = 24, offset = 86
            else if i >= 86 && i <= 109 {
                nonce[i - 86] = *j;
            } else {
                body.push(*j);
            }
        }

        Ok(Self {
            length,
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
        data.append(&mut self.length.to_vec());
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

        data.append(&mut [0, 8].to_vec());
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
