//! # TCP
//!
//! TCP interface for connecting to the other nodes. (The handlers
//! should get modularized in the future, currently almost everything
//! is hard coded.)

use crate::config::Signaling;
use crate::error::Error;
use crate::node::{Address, Center, Link, Node};
use crate::router::Safe;
use crate::transaction::{Transaction, Wire};
use crate::util::{self, Channel};
use std::cell::RefCell;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

/// Represents the TCP listener and exposes certain functions to
/// interact with the outside world. They are mostly just wrappers
/// around the underlying TCP modules.
pub struct Listener {
    center: Center,
    listener: TcpListener,
    connections: RefCell<ConnectionBucket>,
    channel: Channel<Transaction>,
    limit: usize,
    table: Safe,
    cache: Cache,
    signaling: Signaling,
}

struct Connection {
    address: Address,
    channel: Channel<Action>,
}

struct Handler {
    channel: Channel<Action>,
    socket: TcpStream,
    cache: Cache,
}

/// TODO: Reduce dependance on dedicated channel enums.
#[derive(Clone, Debug, PartialEq)]
enum Action {
    Message(Wire),
    Shutdown,
}

/// A cache of recent Transaction. Since each message might get
/// received multiple times, to avoid processing it more than once a
/// cache is introduced, that stores all recent messages. It has a
/// maximum number of elemets, once that size has been reached the
/// oldest elements will get dropped. This doesn't guarantee each
/// event will only be handled once but it should prevent any
/// duplication under good network conditions. Should a message be
/// delayed by a lot it still possible it gets processed more than
/// once.
#[derive(Clone)]
struct Cache {
    /// All current Transactions in the cache. Instead of only storing
    /// the messages the entire transactions will get stored, which
    /// should make comparisons faster for larger objects. The array
    /// will be sorted by age on every update.
    elements: Arc<Mutex<Vec<[u8; 16]>>>,
    /// The maximum size of the cache in number of elements. Once the
    /// size has been reached the oldest element will get dropped to
    /// make space for new Transactions.
    limit: usize,
}

struct ConnectionBucket {
    pub connections: Vec<Connection>,
}

impl Connection {
    fn new(address: Address, socket: TcpStream, cache: Cache) -> (Self, Handler) {
        let (c1, c2) = Channel::new();
        let connection = Connection {
            address,
            channel: c1,
        };
        let handler = Handler {
            channel: c2,
            socket,
            cache,
        };
        (connection, handler)
    }

    /// Since there is no reason to use a blocking function on the
    /// Connection directly only the non-blocking function is exposed.
    fn try_recv(&self) -> Option<Action> {
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
        channel: Channel<Transaction>,
        limit: usize,
        table: Safe,
        signaling: Signaling,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind(center.link.to_string())?;
        listener.set_nonblocking(true)?;
        let listener = Self {
            center,
            listener,
            // TODO: Add param
            cache: Cache::new(100),
            connections: RefCell::new(ConnectionBucket::new()),
            channel,
            limit,
            table,
            signaling,
        };
        Ok(listener)
    }

    pub fn start(self) {
        thread::spawn(move || {
            // TODO: Error handler
            loop {
                // 1. Read from Channel (non-blocking)
                if let Some(t) = self.channel.try_recv() {
                    if t.target() == self.center.public {
                        let _ = self.channel.send(t);
                    } else {
                    }
                }

                // 2. Read from TCP listener
                match self.listener.accept() {
                    Ok((mut socket, _addr)) => {}
                    Err(_) => {
                        log::error!("unable to handle incoming TCP connection.");
                    }
                }

                // 3. Read from Connection channels
                {
                    let mut drop = false;
                    let mut addr = Address::random();

                    // 3. Read from each Connection Channel.
                    for conn in self.connections.borrow().connections.iter() {
                        if let Some(action) = conn.try_recv() {
                            match action {
                                Action::Message(wire) => {
                                    let t = Transaction::from_wire(&wire).unwrap();
                                    let _ = self.channel.send(t);
                                }
                                Action::Shutdown => {
                                    //self.connections.borrow_mut().remove(&addr);
                                    drop = true;
                                    addr = conn.address();
                                }
                            }
                        }
                    }

                    if drop {
                        self.connections.borrow_mut().remove(&addr);
                    }
                }
            }
        });
    }
}

impl Handler {
    fn spawn(mut self) {
        thread::spawn(move || {
            // Dedicated thread per socket.
            loop {
                // Incoming TCP
                if let Ok(wire) = Handler::message(&mut self.socket) {
                    if !self.cache.exists(&wire.uuid) {
                        self.cache.add(&wire.uuid);
                        let _ = self.channel.send(Action::Message(wire));
                    }
                }

                // Channel messages
                if let Some(action) = self.channel.try_recv() {
                    match action {
                        Action::Message(wire) => {
                            if !self.cache.exists(&wire.uuid) {
                                self.cache.add(&wire.uuid);
                                // message
                                let e = self.socket.write(&wire.as_bytes());
                                if e.is_err() {
                                    let _ = self.channel.send(Action::Shutdown);
                                    break;
                                }
                            }
                        }
                        Action::Shutdown => {
                            break;
                        }
                    }
                }
            }
        });
    }

    fn message(socket: &mut TcpStream) -> Result<Wire, Error> {
        let mut header = [0; 142];
        let header_length = socket.read(&mut header)?;
        if header_length != 142 {
            return Err(Error::Invalid(String::from(
                "Received invalid header data!",
            )));
        }
        if header == [0; 142] {
            // if the wire is empty it means a bootstrapping request,
            // but usually there is no reason to issue a bootstrapping
            // request on an active connection.
        }
        let length = util::get_length(&header);
        let body = read(socket, length)?;

        let wire: Vec<u8> = header
            .to_vec()
            .into_iter()
            .chain(body.into_iter())
            .collect();
        let wire = Wire::from_bytes(&wire)?;
        Ok(wire)
    }
}

fn read(stream: &mut TcpStream, length: usize) -> Result<Vec<u8>, Error> {
    let mut data = vec![0; length];
    stream.read_exact(&mut data)?;
    return Ok(data);
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

impl Cache {
    /// Creates a new empty cache with a fixed size limit. In the
    /// future it might be helpful to dynamically change the cache
    /// limit, currently that is not implemented.
    fn new(limit: usize) -> Self {
        Self {
            elements: Arc::new(Mutex::new(Vec::new())),
            limit,
        }
    }

    /// Adds a new element to the cache. If the cache is full the
    /// oldest element will get removed and the new element gets
    /// added.
    fn add(&self, uuid: &[u8; 16]) {
        let mut cache = self.elements.lock().unwrap();
        (*cache).push(uuid.clone());
        (*cache).truncate(self.limit);
    }

    /// Checks if a transaction is already in the cache.
    fn exists(&self, id: &[u8; 16]) -> bool {
        match self.find(id) {
            Some(_) => true,
            None => false,
        }
    }

    /// Returns a pointer to a transaction should the same uuid be
    /// stored in the cache. In the future the entire cache could get
    /// restructured to only keep track of uuids.
    fn find(&self, id: &[u8; 16]) -> Option<[u8; 16]> {
        let cache = self.elements.lock().unwrap();
        let index = (*cache).iter().position(|uuid| uuid == id);
        match index {
            Some(i) => {
                let elem = (*cache).get(i).unwrap();
                return Some(elem.clone());
            }
            None => None,
        }
    }
}
