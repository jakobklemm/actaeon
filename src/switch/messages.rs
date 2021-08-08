//! # Messages
//!
//! Message struct and type enums.

use super::router::address::Address;

// TODO: Add Id to each message to prevent duplicate executes.
pub struct Message {
    class: Class,
    source: Address,
    target: Address,
    body: [u8],
}

pub enum Class {
    Lookup,
    Action,
}

impl Class {
    fn new(class: u8) -> Self {
        match class {
            1 => Self::Lookup,
            2 => Self::Action,
        }
    }
}

impl Message {
    // 2x ID + 8 bit Class ID = 520 bit / 8 Byte Message
    pub fn new(data: &[u8]) {
        if data.len() <= 65 {
            // TODO: Discard invalid messages
            panic!("Message not valid!");
        }
        let class = data[0];
        let source = data[1..33];
        let target = data[33..65];
        let class = Class::new(class);
        let source = Address::from_message(source);
        let target = Address::from_message(target);
        let body = data[66..];
        Self {
            class,
            source,
            target,
            body,
        }
    }
}
