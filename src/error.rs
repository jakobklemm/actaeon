//! # Error
//!
//! Internal error enum

use std::fmt;

/// Collection of error types of the entire system. There is currently
/// no kind of smart conversion and this error do not interact with
/// other crates or the built in types.
#[derive(Clone, Debug)]
pub enum Error {
    /// If the network is experiencing issues connecting to other
    /// nodes.
    Connection,
    /// If any of the signaling servers can't be reached. This could
    /// be a reason to exit or panic.
    Signaling,
    /// If the local config is not valid in any way. This might need
    /// to be expanded later to cover different kinds of config
    /// issues.
    Config,
    /// Should messages or transactions not be valid or corrupted.
    Invalid,
    /// Should there be any issues with the local system, for example
    /// permissions or issues with the local system time.
    System,
    /// If the listener is currently busy or unable to stop.
    Busy,
    /// Unknown error
    Unknown,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Connection => write!(f, "Network connection failed!"),
            Self::Signaling => write!(f, "Signaling server is unavailable!"),
            Self::Config => write!(f, "Local configuration is not valid!"),
            Self::Invalid => write!(f, "Message is not valid!"),
            Self::System => write!(f, "Operating system error!"),
            Self::Busy => write!(f, "Process is busy or unavailable!"),
            Self::Unknown => write!(f, "Unknown error!"),
        }
    }
}
