//! # Messages
//!
//! Message struct and type enums.

use crate::router::address::Address;

// TODO: Add Id to each message to prevent duplicate executes.
pub struct Message {
    pub class: Class,
    pub source: Address,
    pub target: Address,
    pub body: String,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Class {
    Lookup,
    Action,
}

impl Class {
    fn new(class: usize) -> Self {
        match class {
            1 => Self::Lookup,
            2 => Self::Action,
            _ => panic!("Error"),
        }
    }
}

impl Message {
    // 2x ID + 8 bit Class ID = 520 bit / 8 Byte Message
    pub fn deserialize(data: &[u8]) -> Self {
        if data.len() <= 65 {
            // TODO: Discard invalid messages
            panic!("Message not valid!");
        }
        let class = Class::new(1);
        let source = Address::new("abc");
        let target = Address::new("def");
        let body = String::new();
        Self {
            class,
            source,
            target,
            body,
        }
    }
}
