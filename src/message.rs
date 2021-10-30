//! # Messages
//!
//! The actuall message and body for sending over the network. This
//! module is also responsible for handling encryption and decryption
//! of messages. Currently bodies of messages are the only things
//! being encrypted in the system, in the future this might be
//! expanded to include more items.

use crate::error::Error;
use crate::node::Address;
use crate::node::Center;
use crate::transaction::Class;
use sodiumoxide::crypto::box_::{self, curve25519xsalsa20poly1305::Nonce};

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
    /// Each message corresponds to a topic or has 32 bytes of zero if
    /// its independant.
    pub topic: Address,
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

/// The sodiumoxide Nonce does not support the same functions as are
/// required here. Instead of duplicating this code throughout the
/// codebase a simple wrapper struct is used.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Seed(Nonce);

impl Message {
    /// Create a new message with a random nonce (meant to be called
    /// for new messages).
    pub fn new(
        class: Class,
        source: Address,
        target: Address,
        topic: Address,
        body: Vec<u8>,
    ) -> Self {
        Self {
            class,
            source,
            target,
            topic,
            seed: Seed::new(box_::gen_nonce()),
            body: Body::new(body),
        }
    }

    /// Manually create a new messages will all fields already known.
    pub fn create(
        class: Class,
        source: Address,
        target: Address,
        topic: Address,
        seed: Seed,
        body: Vec<u8>,
    ) -> Self {
        Self {
            class,
            source,
            target,
            topic,
            seed,
            body: Body::new(body),
        }
    }

    /// Encrypt the message using the secret of the center (this
    /// node) and the PublicKey of the target.
    pub fn encrypt(&mut self, center: &Center) {
        self.body.encrypt(&self.seed, &center, &self.target);
    }

    /// This function does not do the opposite of "encrypt". To get
    /// the plain text data after encrypting it the secret of the
    /// target node and the public key of the source node have to be
    /// used.
    pub fn decrypt(&mut self, center: &Center) -> Result<(), Error> {
        self.body.decrypt(&self.seed, &center, &self.source)
    }

    pub fn len(&self) -> [u8; 2] {
        self.body.len()
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

    /// Returns the bytes currently in the body without changing the
    /// encryption.
    pub fn as_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Actual body encryption called by the Message::encrypt. It will
    /// only act if the body is currently plaintext, otherwise it
    /// won't do anything. It does not check if the keys or values are
    /// valid, no it is possible to loose data by encrypting it.
    fn encrypt(&mut self, seed: &Seed, center: &Center, target: &Address) {
        if self.is_plain {
            let enc = box_::seal(&self.bytes, &seed.0, &target.key, &center.secret);
            self.bytes = enc;
            self.is_plain = false;
        }
    }

    /// Actual body decryption called by the Message::decrypt. It will
    /// only act if the body is currently encrypted, otherwise it will
    /// fail. It can also fail if the provided keys aren't valid. If
    /// the data is valid the body of the message will get changed,
    /// otherwise an error will be returned.
    fn decrypt(&mut self, seed: &Seed, center: &Center, source: &Address) -> Result<(), Error> {
        if !self.is_plain {
            let dec = box_::open(&self.bytes, &seed.0, &source.key, &center.secret)?;
            self.bytes = dec;
            self.is_plain = true;
            Ok(())
        } else {
            Err(Error::Invalid(String::from("not encrypted")))
        }
    }

    pub fn len(&self) -> [u8; 2] {
        let len = self.bytes.len();
        let ins = len % 255;
        let sig = len / 255;
        [sig as u8, ins as u8]
    }
}

impl Seed {
    /// Creates a new Seed from a provided sodiumoxide nonce.
    fn new(nonce: Nonce) -> Self {
        Self(nonce)
    }
    /// Creates a new Seed from incoming bytes. It should mostly be
    /// safe to unwrap, sodiumoxide should only fail if the length of
    /// the slice isn't correct (meaning 24 bytes).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if let Some(nonce) = Nonce::from_slice(bytes) {
            Ok(Self(nonce))
        } else {
            Err(Error::Invalid(String::from(
                "provided nonce bytes are invalid",
            )))
        }
    }

    /// The main reason why a wrapper around Nonce was needed: the
    /// wire methods all require structs but sodiumoxide by default
    /// only provides the option to convert into a slice. This
    /// function takes care of the conversion by creating a new array
    /// and populating it with elements from the Nonce.
    pub fn as_bytes(&self) -> [u8; 24] {
        let mut bytes: [u8; 24] = [0; 24];
        for (i, j) in self.0.as_ref().into_iter().enumerate() {
            bytes[i] = *j;
        }
        return bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;

    #[test]
    fn test_seed_parse() {
        let seed = box_::gen_nonce();
        let s = Seed::from_bytes(&seed.0).unwrap();
        assert_eq!(s.as_bytes(), seed.0[..]);
    }

    #[test]
    fn test_message_encrypt() {
        let mut m = Message::new(
            Class::Ping,
            Address::generate("a").unwrap(),
            Address::generate("b").unwrap(),
            Address::random(),
            Vec::new(),
        );
        let center = Center::new(box_::gen_keypair().1, String::from(""), 0);
        m.encrypt(&center);
        assert_ne!(m.body.as_bytes().len(), 1);
    }
    #[test]
    fn test_message_decrypt() {
        let (theirpk, theirsk) = box_::gen_keypair();
        let (ourpk, oursk) = box_::gen_keypair();
        let mut m = Message::new(
            Class::Ping,
            Address::new(theirpk),
            Address::new(ourpk),
            Address::random(),
            [111, 42].to_vec(),
        );
        let theircenter = Center::new(theirsk, String::from(""), 0);
        let ourcenter = Center::new(oursk, String::from(""), 0);
        m.encrypt(&theircenter);
        m.decrypt(&ourcenter).unwrap();
        assert_eq!(m.body.as_bytes(), [111, 42]);
    }

    #[test]
    fn test_length_empty() {
        let body = Body::new(Vec::new());
        let len = body.len();
        assert_eq!(len, [0, 0]);
    }

    #[test]
    fn test_length_once() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let body = Body::new(data);
        let len = body.len();
        assert_eq!(len, [0, 10]);
    }

    #[test]
    fn test_length_full() {
        let mut data = Vec::new();
        for i in 0..255 {
            data.push(i);
        }

        for i in 0..255 {
            data.push(i);
        }

        data.push(42);
        let body = Body::new(data);
        let len = body.len();
        assert_eq!(len, [2, 1]);
    }
}
