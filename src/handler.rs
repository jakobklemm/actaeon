//! # TCP
//!
//! TCP interface for connecting to the other nodes. (The handlers
//! should get modularized in the future, currently almost everything
//! is hard coded.)

use crate::error::Error;
use crate::node::Address;
use crate::node::{Center, Node};
use crate::router::Safe;
use crate::transaction::{Transaction, Wire};
use crate::util::{self, Channel};
use std::cell::RefCell;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Represents the TCP listener and exposes certain functions to
/// interact with the outside world. They are mostly just wrappers
/// around the underlying TCP modules.
pub struct Listener {
    listener: TcpListener,
    connections: RefCell<ConnectionBucket>,
    channel: Channel<HandlerAction>,
    limit: usize,
    table: Safe,
}

struct Connection {
    address: Address,
    channel: Channel<HandlerAction>,
}

struct Handler {
    channel: Channel<HandlerAction>,
    socket: TcpStream,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HandlerAction {
    Message(Wire),
    Incoming(Transaction),
    Shutdown,
}

struct ConnectionBucket {
    pub connections: Vec<Connection>,
}

impl Connection {
    fn new(address: Address, socket: TcpStream) -> (Self, Handler) {
        let (c1, c2) = Channel::new();
        let connection = Connection {
            address,
            channel: c1,
        };
        let handler = Handler {
            channel: c2,
            socket,
        };
        (connection, handler)
    }

    /// Since there is no reason to use a blocking function on the
    /// Connection directly only the non-blocking function is exposed.
    fn try_recv(&self) -> Option<HandlerAction> {
        self.channel.try_recv()
    }

    fn address(&self) -> Address {
        self.address.clone()
    }
}

impl Listener {
    /// Spaws a new TCP listener based on the link details of the
    /// center.
    pub fn new(
        center: Center,
        channel: Channel<HandlerAction>,
        limit: usize,
        table: Safe,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let listener = Self {
            listener,
            connections: RefCell::new(ConnectionBucket::new()),
            channel,
            // TODO: Temp for testing, replace with Channel to -->?
            limit,
            table,
        };
        Ok(listener)
    }

    pub fn start(self) {
        thread::spawn(move || loop {
            // 1. Read from Channel (non-blocking)
            if let Some(action) = self.channel.try_recv() {
                // TODO: Lookup targets from RT
                let targets: Vec<Node> = vec![];
                for node in targets {
                    let target = node.address.clone();
                    match self.connections.borrow().get(&target) {
                        Some(conn) => {
                            let _ = conn.channel.send(action.clone());
                        }
                        None => {
                            // TODO: Handle outgoing
                        }
                    }
                }
            }

            // 2. Read from TCP listener
            match self.listener.accept() {
                Ok((mut socket, _addr)) => {
                    log::info!("new incoming TCP connection!");
                    match Listener::handle_establish(&mut socket) {
                        Ok((wire, node)) => {
                            let address = node.address.clone();
                            // TODO: Handle error
                            let _ = self.channel.send(HandlerAction::Message(wire));
                            // TODO: Integrate RT
                            // Check if still space available.
                            if self.connections.borrow().len() >= self.limit {
                                // No space => drop conn.
                                // TODO: Try if requried.
                                continue;
                            }
                            let (connection, handler) = Connection::new(address, socket);
                            self.connections.borrow_mut().add(connection);
                            Handler::spawn(handler);
                        }
                        Err(e) => {
                            log::warn!("received invalid TCP data: {}", e);
                        }
                    }
                }
                Err(_) => {
                    log::error!("unable to handle incoming TCP connection.");
                }
            }

            // 3. Read from each Connection Channel.
            for conn in self.connections.borrow().connections.iter() {
                if let Some(action) = conn.try_recv() {
                    match action {
                        HandlerAction::Message(wire) => {
                            // TODO: Handle easy Kademlia cases.
                            // TODO: Handle error / thread crash.
                            let _ = self.channel.send(HandlerAction::Message(wire));
                        }
                        HandlerAction::Incoming(transaction) => {
                            // TODO: Handle transaction
                        }
                        HandlerAction::Shutdown => {
                            let addr = conn.address();
                            self.connections.borrow_mut().remove(&addr);
                        }
                    }
                }
            }
        });
    }

    fn handle_establish(socket: &mut TcpStream) -> Result<(Wire, Node), Error> {
        let header = Listener::handle_header(socket)?;
        let length = util::get_length(&header);
        let body = Listener::handle_body(socket, length)?;
        let wire: Vec<u8> = header
            .to_vec()
            .into_iter()
            .chain(body.into_iter())
            .collect();
        let wire = Wire::from_bytes(&wire)?;
        let node = Listener::handle_node(socket)?;
        Ok((wire, node))
    }

    fn handle_header(socket: &mut TcpStream) -> Result<[u8; 110], Error> {
        let mut header: [u8; 110] = [0; 110];
        if let Ok(length) = socket.read(&mut header) {
            if length < 110 {
                // TODO: Handle bootstrap or center lookup requests.
                return Err(Error::Invalid(String::from("received invalid data!")));
            }
            Ok(header)
        } else {
            Err(Error::Invalid(String::from("received invalid data!")))
        }
    }

    fn handle_body(socket: &mut TcpStream, length: usize) -> Result<Vec<u8>, Error> {
        let mut body: Vec<u8> = vec![0; length];
        if let Ok(_length) = socket.read_exact(&mut body) {
            return Ok(body);
        }
        Err(Error::Invalid(String::from("received invalid data!")))
    }

    fn handle_node(socket: &mut TcpStream) -> Result<Node, Error> {
        let mut header = [0; 34];
        let _ = socket.read(&mut header);
        let length = [header[0], header[1]];
        let length = util::integer(length);
        let mut link: Vec<u8> = vec![0; length.into()];
        let _ = socket.read_exact(&mut link);
        let mut node_bytes = Vec::new();
        node_bytes.append(&mut header.to_vec());
        node_bytes.append(&mut link);
        let node = Node::from_bytes(node_bytes)?;
        Ok(node)
    }
}

impl Handler {
    fn spawn(mut self) {
        thread::spawn(move || {
            // Dedicated thread per socket.
            loop {
                // Incoming TCP
                if let Ok(wire) = Handler::message(&mut self.socket) {
                    let _ = self.channel.send(HandlerAction::Message(wire));
                } else {
                }

                // Channel messages
                if let Some(action) = self.channel.try_recv() {
                    match action {
                        HandlerAction::Message(wire) => {
                            let e = self.socket.write(&wire.as_bytes());
                            if e.is_err() {
                                let _ = self.channel.send(HandlerAction::Shutdown);
                                break;
                            }
                        }
                        HandlerAction::Incoming(transaction) => {
                            // TODO: Handle transaction => distribute
                        }
                        HandlerAction::Shutdown => {
                            break;
                        }
                    }
                }
            }
        });
    }

    fn message(socket: &mut TcpStream) -> Result<Wire, Error> {
        let mut header = [0; 110];
        let header_length = socket.read(&mut header)?;
        if header_length != 110 {
            return Err(Error::Invalid(String::from(
                "Received invalid header data!",
            )));
        }
        println!("received header data: {:?}", header);
        let length = util::get_length(&header);
        let mut body: Vec<u8> = vec![0; length];
        socket.read_exact(&mut body)?;
        let wire: Vec<u8> = header
            .to_vec()
            .into_iter()
            .chain(body.into_iter())
            .collect();
        let wire = Wire::from_bytes(&wire)?;
        Ok(wire)
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
        let table = Safe::new(42, center.clone());
        let h = Listener::new(center, c1, 42, table).unwrap();
        assert_eq!(
            h.listener.local_addr().unwrap().ip().to_string(),
            String::from("127.0.0.1")
        );
    }
}
