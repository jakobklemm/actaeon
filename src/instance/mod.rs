//! # Instance
//!
//! The program needs one instance which stores the config, the routing table and the local storage engine.

use crate::config::Config;

// The Instance struct has public fields and is meant to be publicly.
pub struct Instance<'a> {
    pub config: &'a Config<'a>,
}

impl<'a> Instance<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config: &config }
    }
}
