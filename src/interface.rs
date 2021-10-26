//! # Interface

use crate::config::{CenterConfig, Config};
use crate::error::Error;
use crate::node::Address;
use crate::node::Center;
use crate::signaling::{ActionBucket, Signaling};
use crate::switch::Switch;
use crate::switch::{Command, SwitchAction, SystemAction};
use crate::topic::Topic;
use crate::transaction::Transaction;
use crate::util::Channel;

/// Starting the switch will create both Interface and Switch objects.
/// The Interface will be passed up and to the user / instance. From
/// there the user can interact (receive messages) with the listener.
pub struct Interface {
    /// Center used for getting message origins.
    pub center: Center,
}

impl Interface {
    /// Creates a new Interface. This function is currently one of the
    /// core components of starting up the system. In the future this
    /// might have to be wrapped by a start function.
    pub fn new(config: Config, center: Center) -> Result<Self, Error> {
        Ok(Self { center })
    }
}
