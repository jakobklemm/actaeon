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

/// Computes the length of a slice and returns it in the system wide
/// two byte array.
pub fn length(data: &[u8]) -> [u8; 2] {
    let length = data.len();
    let sig: u8 = (length / 255) as u8;
    let ins: u8 = (length % 255) as u8;
    [sig, ins]
}

/// Converts the standard two byte length format into a usize.
pub fn integer(length: [u8; 2]) -> usize {
    (length[0] as usize * 255) + length[1] as usize
}

/// Most binary messages have their length as the first two bytes of
/// the array. This function computes the length based only on the
/// first two bytes.
pub fn get_length(data: &[u8]) -> usize {
    data[0] as usize * 255 + data[1] as usize
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

    #[test]
    fn test_length_simple() {
        let data = vec![0, 1, 244, 213];
        assert_eq!(length(&data), [0, 4]);
    }

    #[test]
    fn test_length_one() {
        let mut outer = Vec::new();
        for i in 0..255 {
            outer.push(i);
        }
        outer.push(42);
        let length = length(&outer);
        assert_eq!(length, [1, 1]);
    }

    #[test]
    fn test_length_full() {
        let mut outer = Vec::new();
        for _ in 0..254 {
            for j in 0..255 {
                outer.push(j);
            }
        }
        outer.push(42);
        let length = length(&outer);
        assert_eq!(length, [254, 1]);
    }

    #[test]
    fn test_length_back() {
        let data = vec![1, 2, 3, 4, 5, 6, 7];
        let len = data.len();
        assert_eq!(len, integer(length(&data)));
    }

    #[test]
    fn test_length_double_random() {
        for i in 0..1000 {
            let mut data = Vec::new();
            for j in 0..i {
                data.push((j % 255) as u8);
            }
            let real = data.len();
            let len = integer(length(&data));
            assert_eq!(real, len);
        }
    }
}
