//! Engine
//!
//! Trait for user implementation.

use crate::error::Error;

// Temporary placeholder, move to dedicated message & transaction structs
pub struct Transaction {}

pub trait Engine {
    fn receive(&self, message: String);
    fn send(&self, message: String) -> Result<Transaction, Error>;
}
