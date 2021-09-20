//! # TCP
//!
//! TCP interface for connecting to the other nodes. (The handlers
//! should get modularized in the future, currently almost everything
//! is hard coded.)

use crate::error::Error;
use crate::node::Center;
use crate::transaction::Wire;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

pub struct Handler {
    listener: TcpListener,
    center: Center,
}

impl Handler {
    pub fn new(center: Center) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let handler = Self { listener, center };
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
}
