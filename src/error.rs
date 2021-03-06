//! # Error
//!
//! Internal error enum

use std::fmt;
use std::net::AddrParseError;
use std::string::FromUtf8Error;
use std::sync::mpsc::SendError;

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
    /// If the cache, table, bucket, etc. is full.
    Full,
    /// Unknown error
    Unknown,
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(_err: std::io::Error) -> Self {
        Self::System(String::from("generic IO error"))
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_err: FromUtf8Error) -> Self {
        Self::System(String::from("address is not valid"))
    }
}

impl From<AddrParseError> for Error {
    fn from(_err: AddrParseError) -> Self {
        Self::Invalid(String::from("ip address is not a valid ipv4 format"))
    }
}

impl From<()> for Error {
    fn from(_err: ()) -> Self {
        Self::Invalid(String::from("data is invalid"))
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_err: SendError<T>) -> Self {
        Self::Connection(String::from("channel is no longer available"))
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
            Self::Full => write!(f, "item limit has been reached"),
            Self::Unknown => write!(f, "unknown error"),
        }
    }
}
