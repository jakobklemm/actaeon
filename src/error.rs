//! # Error
//!
//! Internal error enum

use thiserror::Error;

/// Collection of error types of the entire system. There is currently
/// no kind of smart conversion and this error do not interact with
/// other crates or the built in types.
#[derive(Error, Debug)]
pub enum Error {
    /// If the network is experiencing issues connecting to other
    /// nodes.
    #[error("Connection failed!")]
    Connection,
    /// If any of the signaling servers can't be reached. This could
    /// be a reason to exit or panic.
    #[error("Signaling servers not available!")]
    Signaling,
    /// If the local config is not valid in any way. This might need
    /// to be expanded later to cover different kinds of config
    /// issues.
    #[error("Local config is invalid!")]
    Config,
    /// Should messages or transactions not be valid or corrupted.
    #[error("Message is invalid!")]
    Invalid,
    /// Should there be any issues with the local system, for example
    /// permissions or issues with the local system time.
    #[error("OS error!")]
    System,
    #[error("Unknown error!")]
    Unknown,
}
