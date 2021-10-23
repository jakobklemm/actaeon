//! # TCP
//!
//! TCP interface for connecting to the other nodes. (The handlers
//! should get modularized in the future, currently almost everything
//! is hard coded.)

use crate::error::Error;
use crate::node::{Center, Node};
use crate::transaction::Wire;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

/// Represents the TCP listener and exposes certain functions to
/// interact with the outside world. They are mostly just wrappers
/// around the underlying TCP modules.
pub struct Handler {
    listener: TcpListener,
}

impl Handler {
    /// Spaws a new TCP listener based on the link details of the
    /// center.
    pub fn new(center: Center) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let handler = Self { listener };
        Ok(handler)
    }

    /// The main (and only) way to read data from the socket. At this
    /// point in the system there is no difference between try_read
    /// and read, this read function is always non-blocking.
    ///
    /// The current TCP implementation is by no means the most
    /// efficient way of handling the connections. For each Message
    /// that is send a dedicated TCP connection is created, all the
    /// bytes are sent and the connection is terminated.
    ///
    /// In the future this has to be improved in two ways: 1. Switch
    /// to UDP over TCP for all simple Messages. 2. Keep a separate
    /// list of active connections for common targets, that are likely
    /// to be reused frequently.
    pub fn read(&mut self) -> Option<Wire> {
        match self.listener.accept() {
            Ok((mut socket, _addr)) => {
                let mut bytes = Vec::new();
                match socket.read_to_end(&mut bytes) {
                    Ok(_len) => {
                        let wire = Wire::from_bytes(&bytes);
                        match wire {
                            Ok(w) => Some(w),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Sends a Message to the given Node. There are a number of ways
    /// this can fail, currently they are all handled together, this
    /// has to be improved. Most obviously it can fail if there is no
    /// Link data at all, or if the Node is unreachable (for any
    /// number of reasons).
    ///
    /// There is also a need to improve memory usage, since the Link
    /// details are cloned on conversion but the function takes
    /// ownership of the entire Node object.
    pub fn send(&self, data: Wire, node: Node) -> Result<(), Error> {
        if node.link.is_none() {
            // TODO: Add to node link refetch
            return Err(Error::Invalid(String::from("no link data found")));
        } else {
            let mut stream = TcpStream::connect(node.link.as_ref().unwrap().to_string())?;
            stream.write(&data.as_bytes())?;
            Ok(())
        }
    }

    /// Creates a new Handler.
    pub fn try_clone(&self) -> Result<Self, Error> {
        Ok(Self {
            listener: self.listener.try_clone()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;

    #[test]
    fn test_create_handler() {
        let (_, s) = box_::gen_keypair();
        let center = Center::new(s, String::from("127.0.0.1"), 42434);
        let h = Handler::new(center).unwrap();
        assert_eq!(
            h.listener.local_addr().unwrap().ip().to_string(),
            String::from("127.0.0.1")
        );
    }
}
