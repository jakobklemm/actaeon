//! # Utility
//!
//! Collection of non specific helpers & utility functions / objects.

use crate::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};

/// Bidirectional communcation wrapper around mspc channels.
#[derive(Debug)]
pub struct Channel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Channel<T> {
    /// Creates a new pair of Channels. Since two of them are always
    /// connected they have to be created together.
    pub fn new() -> (Self, Self) {
        let (s1, r1): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        let (s2, r2): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        (
            Self {
                sender: s1,
                receiver: r2,
            },
            Self {
                sender: s2,
                receiver: r1,
            },
        )
    }

    /// Sends a message through the Channel. This can fail if the
    /// remote socket is unavailable. Currently this error case is not
    /// handled.
    pub fn send(&self, message: T) -> Result<(), Error> {
        match self.sender.send(message) {
            Ok(()) => Ok(()),
            Err(_) => Err(Error::Connection(String::from("channel is not available"))),
        }
    }

    /// Like send this is also a wrapper around the mpsc try_recv
    /// method. Currently error are not getting handled and if the
    /// socket is unavailable None will be returned.
    pub fn try_recv(&self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }

    /// Like send this is also a wrapper around the mpsc recv method.
    /// Currently error are not getting handled and if the socket is
    /// unavailable None will be returned.
    pub fn recv(&self) -> Option<T> {
        match self.receiver.recv() {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_send() {
        let (c1, c2) = Channel::new();
        let _ = c1.send(42);
        assert_eq!(c2.recv(), Some(42));
    }
}
