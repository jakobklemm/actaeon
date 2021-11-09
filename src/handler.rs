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

#[derive(Debug)]
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

#[derive(Debug)]
struct ConnectionBucket {
    pub connections: Vec<Connection>,
    pub limit: usize,
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

    fn recv(&self) -> Option<Action> {
        self.channel.recv()
    }

    fn send(&self, wire: Wire) -> Result<(), Error> {
        self.channel.send(Action::Message(wire))
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
            // TODO: Add params
            cache: Cache::new(100),
            connections: RefCell::new(ConnectionBucket::new(10)),
            channel,
            limit,
            table,
            signaling,
        };
        Ok(listener)
    }

    pub fn start(self) {
        thread::spawn(move || {
            if let Ok((socket, node)) =
                Listener::bootstrap(self.signaling, &self.table, &self.center)
            {
                let (conn, handler) = Connection::new(node.address, socket, self.cache.clone());
                handler.spawn();
                self.connections.borrow_mut().add(conn);
            }
            println!("data: bootstrap completed!");
            // TODO: Error handler
            loop {
                // 1. Read from Channel (non-blocking)
                if let Some(t) = self.channel.try_recv() {
                    if t.target() == self.center.public {
                        let _ = self.channel.send(t);
                    } else {
                        let _ = Listener::distribute(
                            t,
                            &self.table,
                            &self.cache,
                            &mut self.connections.borrow_mut(),
                            &self.center,
                            self.limit,
                        );
                    }
                }

                // 2. Read from TCP listener
                match self.listener.accept() {
                    Ok((mut stream, _addr)) => {
                        if let Ok(node) = Handler::read_node(&mut stream) {
                            let _ = Handler::write_node(&mut stream, &self.center);
                            let addr = node.address.clone();
                            self.table.add(node);
                            let (conn, handler) = Connection::new(addr, stream, self.cache.clone());
                            handler.spawn();
                            self.connections.borrow_mut().add(conn);
                        }
                        // if any of the steps fail the connection gets dropped.
                    }
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
                                    if wire.is_empty() {
                                        let response = Wire::bootstrap(self.table.export());
                                        println!(
                                            "data: received bootstrap request, response: {:?}",
                                            response
                                        );
                                        let _ = conn.send(response);
                                    } else {
                                        let t = Transaction::from_wire(&wire).unwrap();
                                        let _ = self.channel.send(t);
                                    }
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

    fn distribute(
        t: Transaction,
        table: &Safe,
        cache: &Cache,
        conns: &mut ConnectionBucket,
        center: &Center,
        limit: usize,
    ) -> Result<(), Error> {
        println!("data: sending message: {:?}", t);
        let target = t.target();
        let targets = table.get_copy(&target, limit);
        if targets.len() == 0 {
            return Err(Error::System("no target nodes found".to_string()));
        }
        for node in targets {
            let addr = node.address.clone();
            if let Some(conn) = conns.get(&addr) {
                return conn.send(t.to_wire());
            } else {
                if conns.len() >= conns.limit {
                    return Listener::write(t.to_wire(), node, center);
                } else {
                    if let Ok(stream) = Listener::activate(t.to_wire(), node, center) {
                        let (conn, handler) = Connection::new(addr, stream, cache.clone());
                        handler.spawn();
                        conns.add(conn);
                    } else {
                        // TODO: Update RT, deactivate
                    }
                }
            }
        }
        Ok(())
    }

    fn write(wire: Wire, node: Node, center: &Center) -> Result<(), Error> {
        match node.link {
            Some(link) => {
                let mut stream = TcpStream::connect(link.to_string())?;
                let _ = Handler::write_node(&mut stream, center);
                let node = Handler::read_node(&mut stream)?;
                println!("data: received node from single write: {:?}", node);
                stream.write(&wire.as_bytes())?;
                return Ok(());
            }
            None => {
                return Err(Error::Connection(String::from("no link data exists")));
            }
        }
    }

    fn activate(wire: Wire, node: Node, center: &Center) -> Result<TcpStream, Error> {
        match node.link {
            Some(link) => {
                let mut stream = TcpStream::connect(link.to_string())?;
                let _ = Handler::write_node(&mut stream, center);
                let node = Handler::read_node(&mut stream)?;
                println!("data: received node from activaition: {:?}", node);
                stream.write(&wire.as_bytes())?;
                return Ok(stream);
            }
            None => {
                return Err(Error::Connection(String::from("no link data exists")));
            }
        }
    }

    fn bootstrap(
        signaling: Signaling,
        table: &Safe,
        center: &Center,
    ) -> Result<(TcpStream, Node), Error> {
        println!("data: running bootstrap!");
        let mut stream = TcpStream::connect(signaling.to_string())?;
        let _ = Handler::write_node(&mut stream, center);
        let node = Handler::read_node(&mut stream)?;
        let _ = stream.write(&[0; 142])?;
        let wire = Handler::read_wire(&mut stream)?;
        let nodes = Node::from_bulk(wire.body().to_vec());
        for node in nodes {
            table.add(node);
        }
        Ok((stream, node))
    }
}

impl Handler {
    fn spawn(mut self) {
        thread::spawn(move || {
            // Otherwise the read_wire will be blocking and only allow
            // one iteration for each incoming message.
            let _ = self.socket.set_nonblocking(true);
            // Dedicated thread per socket.
            loop {
                // Incoming TCP
                if let Ok(wire) = Handler::read_wire(&mut self.socket) {
                    if !self.cache.exists(&wire.uuid) || wire.is_empty() {
                        self.cache.add(&wire.uuid);
                        let _ = self.channel.send(Action::Message(wire));
                    }
                }

                // Channel messages
                if let Some(action) = self.channel.try_recv() {
                    match action {
                        Action::Message(wire) => {
                            if !self.cache.exists(&wire.uuid) || wire.is_empty() {
                                self.cache.add(&wire.uuid);
                                // message
                                println!("data: using existing connection",);
                                let message = wire.as_bytes();
                                let e = self.socket.write(&message);
                                if e.is_err() {
                                    let _ = self.channel.send(Action::Shutdown);
                                    println!("data: terminating thread!");
                                    break;
                                }
                            }
                        }
                        Action::Shutdown => {
                            println!("data: terminating thread!");
                            break;
                        }
                    }
                }
            }
        });
    }

    fn read_wire(stream: &mut TcpStream) -> Result<Wire, Error> {
        let mut header = [0; 142];
        match stream.read(&mut header) {
            Ok(read_len) => {
                if read_len != 142 {
                    return Err(Error::Connection("unable to read header bytes".to_string()));
                }
                let length = util::get_length(&header);
                let mut body = vec![0; length];
                stream.read_exact(&mut body)?;

                let mut message = Vec::new();
                message.append(&mut header.to_vec());
                message.append(&mut body);

                let wire = Wire::from_bytes(&message)?;
                Ok(wire)
            }
            Err(_) => {
                return Err(Error::Connection("unable to read header bytes".to_string()));
            }
        }
    }

    fn read_node(stream: &mut TcpStream) -> Result<Node, Error> {
        let mut header = [0; 34];
        let header_length = stream.read(&mut header)?;
        if header_length != 34 {
            return Err(Error::Connection("unable to read header bytes".to_string()));
        }
        let length = util::get_length(&header);
        let mut link = vec![0; length];
        stream.read_exact(&mut link)?;

        let addr = Address::from_slice(&header[2..])?;
        let link = Link::from_bytes(link)?;
        let node = Node::new(addr, Some(link));
        Ok(node)
    }

    fn write_node(stream: &mut TcpStream, center: &Center) -> Result<(), Error> {
        let node = Node::new(center.public.clone(), Some(center.link.clone()));
        stream.write(&node.as_bytes())?;
        Ok(())
    }
}

impl ConnectionBucket {
    /// Creates a new SubscriberBucket. Currently there are no limits
    /// or other properties so the Bucket is simply an unlimited
    /// Vec.
    fn new(limit: usize) -> Self {
        Self {
            connections: Vec::new(),
            limit,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::transaction::{Class, Transaction};

    #[test]
    fn test_connection_life() {
        let local = TcpListener::bind("127.0.0.1:45600").unwrap();
        let stream = TcpStream::connect("127.0.0.1:45600").unwrap();
        let addr = Address::random();

        let message = Message::new(
            Class::Action,
            Address::random(),
            Address::random(),
            Address::random(),
            vec![42],
        );

        let t = Transaction::new(message);

        let (conn, handler) = Connection::new(addr.clone(), stream, Cache::new(100));

        handler.spawn();

        let (mut s, _) = local.accept().unwrap();
        let _ = s.write(&t.as_bytes());

        assert_eq!(conn.recv().unwrap(), Action::Message(t.to_wire()));

        let message = Message::new(
            Class::Action,
            Address::random(),
            Address::random(),
            Address::random(),
            vec![43],
        );
        let t = Transaction::new(message);
        println!("data: sending data: {:?}", t.to_wire());
        let _ = conn.send(t.to_wire());

        let wire = Handler::read_wire(&mut s).unwrap();
        assert_eq!(wire, t.to_wire());
    }
}
