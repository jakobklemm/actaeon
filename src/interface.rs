//! # Interface

use crate::config::{CenterConfig, Config};
use crate::error::Error;
use crate::node::Address;
use crate::node::Center;
use crate::record::RecordBucket;
use crate::topic::{Simple, Topic};
use crate::transaction::Transaction;
use crate::util::Channel;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// Center used for getting message origins.
    center: Center,
    records: RecordBucket,
    switch: Channel<InterfaceAction>,
}

pub enum InterfaceAction {
    Shutdown,
    Message(Transaction),
    Subscribe(Simple),
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(config: Config, center: Center) -> Result<Self, Error> {
        let bucket = RecordBucket::new();
        let (c1, c2) = Channel::new();
        Ok(Self {
            center,
            records: bucket,
            switch: c1,
        })
    }

    pub fn subscribe(self, addr: Address) -> Topic {
        let (c1, c2) = Channel::new();
        let local = Topic::new(addr.clone(), c1, Vec::new());
        let remote = Simple::new(addr, c2);
        let _ = self.switch.send(InterfaceAction::Subscribe(remote));
        local
    }
}
