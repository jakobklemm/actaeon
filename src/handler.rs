//! # TCP
//!
//! TCP interface for connecting to the other nodes. (The handlers
//! should get modularized in the future, currently almost everything
//! is hard coded.)

use crate::error::Error;
use crate::node::Address;
use crate::node::{Center, Node};
use crate::transaction::{Transaction, Wire};
use crate::util::Channel;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Represents the TCP listener and exposes certain functions to
/// interact with the outside world. They are mostly just wrappers
/// around the underlying TCP modules.
pub struct Listener {
    listener: TcpListener,
    connections: ConnectionBucket,
    channel: Channel<Wire>,
    limit: usize,
}

struct Connection {
    address: Address,
    channel: Channel<Wire>,
}

struct Handler {
    address: Address,
    channel: Channel<Wire>,
    stream: TcpStream,
}

struct ConnectionBucket {
    connections: Vec<Connection>,
}

impl Connection {
    fn new(address: Address, stream: TcpStream) -> (Self, Handler) {
        let (c1, c2) = Channel::<Wire>::new();
        let connection = Connection {
            address: address.clone(),
            channel: c1,
        };
        let handler = Handler {
            address,
            channel: c2,
            stream,
        };
        (connection, handler)
    }
}

impl Listener {
    /// Spaws a new TCP listener based on the link details of the
    /// center.
    pub fn new(center: Center, channel: Channel<Wire>, limit: usize) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let listener = Self {
            listener,
            connections: ConnectionBucket::new(),
            channel,
            limit,
        };
        Ok(listener)
    }

    pub fn start(mut self) -> Result<(), Error> {
        thread::spawn(move || loop {
            // 1. Read from Channel (non-blocking)
            // 2. Read from TCP listener
            match self.listener.accept() {
                Ok((mut socket, _addr)) => {
                    let mut header: [u8; 110] = [0; 110];
                    if let Ok(length) = socket.read(&mut header) {
                        if length < 110 {
                            // TODO: Handle bootstrap or center lookup requests.
                            continue;
                        }
                        if let Ok(wire) = Listener::handle_wire(&mut socket, header) {
                            // TODO: Handle crashed Channels
                            let _ = self.channel.send(wire);
                            if self.connections.len() >= self.limit {
                                // Connection pool is full, replacing
                                // an existing one isn't possible
                                // right now.
                                continue;
                            }
                            let resp = Listener::get_remote(&mut socket);
                        } else {
                            log::error!("unable to receive TCP data!");
                        }
                    } else {
                        log::error!("unable to receive TCP data!");
                    }
                }
                Err(_) => {}
            }
        });
        Ok(())
    }

    fn handle_wire(socket: &mut TcpStream, header: [u8; 110]) -> Result<Wire, Error> {
        let body_length: u64 = (header[0] * 255 + header[1]).into();
        let mut body: Vec<u8> = Vec::new();
        let mut taker = socket.take(body_length);
        if let Ok(length) = taker.read(&mut body) {
            if length as u64 != body_length {
                return Err(Error::Invalid(String::from("received invalid data!")));
            }
            let mut data: Vec<u8> = Vec::new();
            data.append(&mut header.to_vec());
            data.append(&mut body);
            return Wire::from_bytes(&data);
        }
        Err(Error::Invalid(String::from("received invalid data!")))
    }

    fn get_remote(socket: &mut TcpStream) -> Result<Node, Error> {
        // 6x 0 is not a possible
        let request = [0, 0, 0, 0, 0, 0];
        let res = socket.write(&request)?;
        if res != 6 {
            return Err(Error::Connection(String::from("unable to write data")));
        }
        let mut response: Vec<u8> = Vec::new();
        let mut length: [u8; 2] = [0, 0];
        response.append(&mut length.to_vec());
        if let Ok(read_length) = socket.read(&mut length) {
            if read_length != 2 {
                return Err(Error::Connection(String::from("unable to write data")));
            }
            let length = (length[0] * 255 + length[1]).into();
            let mut taker = socket.take(length);
            if let Ok(read) = taker.read(&mut response) {
                if read as u64 != length {
                    return Err(Error::Connection(String::from("unable to write data")));
                }
                let node = Node::from_bytes(response);
                Ok(node)
            } else {
                return Err(Error::Connection(String::from("unable to write data")));
            }
        } else {
            return Err(Error::Connection(String::from("unable to write data")));
        }
    }

    pub fn read(&mut self) -> Option<Wire> {
        match self.listener.accept() {
            Ok((mut socket, _addr)) => {
                let mut header: [u8; 110] = [0; 110];
                match socket.read(&mut header) {
                    Ok(len) => {
                        if len <= 110 {
                            return None;
                        } else {
                            let length = header[0] * 255 + header[1];
                            let mut body = Vec::new();
                            let mut handle = socket.take(length.into());
                            let _ = handle.read(&mut body);
                            let mut data = Vec::new();
                            data.append(&mut header.to_vec());
                            data.append(&mut body);
                            match Wire::from_bytes(&data) {
                                Ok(wire) => Some(wire),
                                Err(e) => {
                                    log::warn!("received invalid data: {}", e);
                                    None
                                }
                            }
                        }
                    }
                    Err(e) => None,
                }
            }
            Err(_) => {
                // TODO: Handle disconnect
                None
            }
        }
    }

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
}

impl ConnectionBucket {
    /// Creates a new SubscriberBucket. Currently there are no limits
    /// or other properties so the Bucket is simply an unlimited
    /// Vec.
    fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    fn add(&mut self, connection: Connection) {
        match self.get(&connection.address) {
            Some(_) => {}
            None => self.connections.push(connection),
        }
    }

    /// Returns a reference to a specific subscriber with a matching
    /// Address. There isn't really a reason for an end user to use
    /// this (but it is possible for unusual use cases). It will be
    /// called by the "add" function.
    fn get(&self, search: &Address) -> Option<&Connection> {
        let index = self.connections.iter().position(|e| &e.address == search);
        match index {
            Some(i) => self.connections.get(i),
            None => None,
        }
    }

    /// Drops a subscriber from the Bucket should an Unsubscribe event
    /// come in.
    fn remove(&mut self, target: &Address) {
        let index = self.connections.iter().position(|e| &e.address == target);
        match index {
            Some(i) => {
                self.connections.remove(i);
            }
            None => {}
        }
    }

    fn len(&self) -> usize {
        self.connections.len()
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
        let (c1, _) = Channel::new();
        let h = Listener::new(center, c1, 42).unwrap();
        assert_eq!(
            h.listener.local_addr().unwrap().ip().to_string(),
            String::from("127.0.0.1")
        );
    }
}
