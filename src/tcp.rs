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

pub struct Handler {
    listener: TcpListener,
}

impl Handler {
    pub fn new(center: Center) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let handler = Self { listener };
        Ok(handler)
    }

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

    pub fn send(&self, data: Wire, node: Node) -> Result<(), Error> {
        if node.link.is_none() {
            Err(Error::Invalid(String::from("no link data found")))
        } else {
            let mut stream = TcpStream::connect(node.link.unwrap().to_string())?;
            stream.write(&data.as_bytes())?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::box_;

    #[test]
    fn test_create_handler() {
        let (_, s) = box_::gen_keypair();
        let center = Center::new(s, String::from("127.0.0.1"), 42424);
        let h = Handler::new(center).unwrap();
        assert_eq!(
            h.listener.local_addr().unwrap().ip().to_string(),
            String::from("127.0.0.1")
        );
    }
}
