//! # Interface

use crate::config::{CenterConfig, Config};
use crate::error::Error;
use crate::node::Address;
use crate::node::Center;
use crate::record::RecordBucket;
use crate::transaction::Transaction;
use crate::util::Channel;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// Center used for getting message origins.
    center: Center,
    records: RecordBucket,
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(config: Config, center: Center) -> Result<Self, Error> {
        let bucket = RecordBucket::new();
        Ok(Self {
            center,
            records: bucket,
        })
    }
}
