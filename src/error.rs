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
    Connection(String),
    /// If any of the signaling servers can't be reached. This could
    /// be a reason to exit or panic.
    Signaling(String),
    /// If the local config is not valid in any way. This might need
    /// to be expanded later to cover different kinds of config
    /// issues.
    Config(String),
    /// Should messages or transactions not be valid or corrupted.
    Invalid(String),
    /// Should there be any issues with the local system, for example
    /// permissions or issues with the local system time.
    System(String),
    /// If the listener is currently busy or unable to stop.
    Busy(String),
    /// Unknown error
    Unknown,
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(_err: std::io::Error) -> Self {
        Self::System(String::from("generic IO error"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Connection(s) => write!(f, "network connection failed: {}", s),
            Self::Signaling(s) => write!(f, "signaling server is unavailable: {}", s),
            Self::Config(s) => write!(f, "local configuration is not valid: {}", s),
            Self::Invalid(s) => write!(f, "message is not valid: {}", s),
            Self::System(s) => write!(f, "operating system error: {}", s),
            Self::Busy(s) => write!(f, "process is busy or unavailable: {}", s),
            Self::Unknown => write!(f, "unknown error"),
        }
    }
}
