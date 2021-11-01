//! # Switch
//!
//! The Switch is responsible for forwarding messages from all
//! components of the system to all others. It listens on multiple
//! Channels and decides the targets based on type, origin, topic or
//! target.

use crate::error::Error;
use crate::message::Message;
use crate::node::{Address, Center, Node};
use crate::record::{Record, RecordBucket};
use crate::router::Safe;
use crate::signaling::{SignalingAction, Type};
use crate::topic::{Command, TopicBucket};
use crate::transaction::{Class, Transaction};
use crate::util::Channel;
use crate::InterfaceAction;
use std::cell::RefCell;
use std::thread;
use std::time::SystemTime;

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    /// Channel to the Listener, sends and receives full Transactions,
    /// which get serialized on demand when they are being sent.
    listener: Channel<Transaction>,
    /// Channel for communicating with the User. The other end it held
    /// by the Interface and it can be used for subscribing, sending
    /// messages, sending actions. Should sending ever not be possible
    /// this thread as well as all other threads of the system should
    /// get shut down.
    interface: Channel<InterfaceAction>,
    signaling: Channel<SignalingAction>,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: Safe,
    /// Holds a list of all currently active topics. The data is in a
    /// RefCell in order to make interactions in the Thread closure
    /// easier. A Topic means a non-"should be local" Address
    /// subscribed to by the user.
    topics: RefCell<TopicBucket>,
    /// Topics that aren't created / managed by the user, rather are
    /// part of the Kademlia system. Since the Addresses of Topics
    /// need to be fixed / known the location in the system can't be
    /// guaranteed. Instead on the correct nodes a record of that
    /// topic is kept. From there the actual distribution of messages
    /// takes place.
    records: RecordBucket,
    /// Another copy of the Center data used for generating messages.
    center: Center,
}

impl Switch {
    /// Creates a new (Switch, Interface) combo, creating the Cache
    /// and staritng the channel.
    pub fn new(
        listener: Channel<Transaction>,
        interface: Channel<InterfaceAction>,
        signaling: Channel<SignalingAction>,
        center: Center,
        table: Safe,
        records: RecordBucket,
    ) -> Result<Self, Error> {
        let switch = Switch {
            listener,
            interface,
            signaling,
            table,
            topics: RefCell::new(TopicBucket::new()),
            records,
            center,
        };
        Ok(switch)
    }

    /// The switch is responsible for deciding where specific messages
    /// go based on their origin, target and type. It listens on
    /// almost all Channels in the system and can send messages to any
    /// sink.
    pub fn start(self) {
        thread::spawn(move || {
            // There is currently no method of restarting each of the
            // threads, all of them simply consist of a while true
            // loop listening on a number of sources.
            loop {
                // 1. Listen on Interface Channel.
                if let Some(action) = self.interface.try_recv() {
                    log::info!("received action from the user");
                    match action {
                        InterfaceAction::Shutdown => {
                            log::trace!("received shutdown request, terminating switch.");
                            break;
                        }
                        InterfaceAction::Message(transaction) => {
                            log::trace!("received complete message from the user");
                            let _ = self.listener.send(transaction);
                        }
                        InterfaceAction::Subscribe(simple) => {
                            log::trace!("received subscribe resquest from the user");
                            let target = simple.address.clone();
                            self.topics.borrow_mut().add(simple);
                            if self.table.should_be_local(&target) {
                                let message = Message::new(
                                    Class::Subscribe,
                                    self.center.public.clone(),
                                    target.clone(),
                                    target,
                                    Vec::new(),
                                );
                                let t = Transaction::new(message);

                                Switch::handle_subscribe(
                                    t,
                                    &self.listener,
                                    &self.records,
                                    &self.topics,
                                    &self.center,
                                )
                            } else {
                                let message = Message::new(
                                    Class::Subscribe,
                                    self.center.public.clone(),
                                    target.clone(),
                                    // the subscribe action corresponds to
                                    // no topic, but for simplicity the
                                    // target is used twice.
                                    target,
                                    Vec::new(),
                                );
                                let t = Transaction::new(message);
                                let _ = self.listener.send(t);
                            }
                        }
                    }
                }

                // 2. Listen on topics Chanel.
                for simple in &self.topics.borrow().topics {
                    let topic = simple.address.clone();
                    if let Some(command) = simple.channel.try_recv() {
                        log::info!("received message from topic");
                        match command {
                            Command::Drop(addr) => {
                                log::trace!("topic went out of scope");
                                // The addr is of the user to send the
                                // unsubscribe to, not of the topic!
                                self.topics.borrow_mut().remove(&simple.address);
                                let topic = simple.address.clone();
                                if self.table.should_be_local(&topic) {
                                    let message = Message::new(
                                        Class::Unsubscribe,
                                        self.center.public.clone(),
                                        topic.clone(),
                                        topic,
                                        Vec::new(),
                                    );
                                    let t = Transaction::new(message);

                                    Switch::handle_unsubscribe(
                                        t,
                                        &self.listener,
                                        &self.records,
                                        &self.topics,
                                        &self.center,
                                    )
                                } else {
                                    let message = Message::new(
                                        Class::Unsubscribe,
                                        self.center.public.clone(),
                                        addr,
                                        topic,
                                        Vec::new(),
                                    );
                                    let t = Transaction::new(message);
                                    let _ = self.listener.send(t);
                                }
                            }
                            Command::Broadcast(addr, body) => {
                                log::trace!("received broadcast from user");
                                let message = Message::new(
                                    Class::Action,
                                    self.center.public.clone(),
                                    addr,
                                    topic,
                                    body,
                                );
                                let t = Transaction::new(message);
                                let _ = self.listener.send(t);
                            }
                            _ => {}
                        }
                    }
                }

                // 3. Listen on Siganling Channel.
                if let Some(action) = self.signaling.try_recv() {
                    log::info!("received message from signaling thread");
                    match action.action {
                        Type::Ping => {
                            log::trace!("received signaling ping request");
                            let message = Message::new(
                                Class::Ping,
                                self.center.public.clone(),
                                action.target,
                                Address::default(),
                                Vec::new(),
                            );
                            let t = Transaction::build(action.uuid, SystemTime::now(), message);
                            let _ = self.listener.send(t);
                        }
                        Type::Lookup => {
                            log::trace!("received signaling lookup request");
                            let message = Message::new(
                                Class::Lookup,
                                self.center.public.clone(),
                                action.target,
                                Address::default(),
                                Vec::new(),
                            );
                            let t = Transaction::build(action.uuid, SystemTime::now(), message);
                            let _ = self.listener.send(t);
                        }
                        _ => {}
                    }
                }

                // 4. Listen on Handler Channel.
                if let Some(t) = self.listener.try_recv() {
                    log::info!("received message from listener");
                    let target = t.target();
                    if target == self.center.public {
                        log::trace!("handling incoming message locally");
                        // Handle: Ping, Pong, Lookup, Details, Action, Subscriber, Unsubscriber
                        // Error: Subscriber, Unsubscribe
                        match t.class() {
                            Class::Ping => {
                                log::trace!("incoming ping message");
                                Switch::handle_ping(t, &self.listener, &self.center);
                            }
                            Class::Pong => {
                                log::trace!("incoming pong message");
                                Switch::handle_pong(t, &self.signaling);
                            }
                            Class::Lookup => {
                                log::trace!("incoming lookup message");
                                Switch::handle_lookup(t, &self.listener, &self.center);
                            }
                            Class::Details => {
                                log::trace!("incoming details message");
                                Switch::handle_details(t, &self.signaling, &self.table);
                            }
                            Class::Action => {
                                log::trace!("incoming details message");
                                Switch::handle_action(t, &self.topics, &self.interface);
                            }
                            Class::Subscriber => {
                                log::trace!("incoming subscriber message");
                                Switch::handle_subscriber(t, &self.topics, &self.center);
                            }
                            Class::Unsubscriber => {
                                log::trace!("incoming unsubscriber message");
                                Switch::handle_unsubscriber(t, &self.topics);
                            }
                            _ => {
                                log::warn!("received message to invalid target: {:?}", t);
                            }
                        }
                    } else {
                        log::trace!("target is not local but this node might be responsible");
                        // Forward: Ping, Pong, Details, Action, Subscriber, Unsubscriber,
                        // Maybe Handle: Subscribe, Unsubscribe, Lookup
                        match t.class() {
                            Class::Subscribe => {
                                log::trace!("incoming subscribe message for local topic");
                                Switch::handle_subscribe(
                                    t,
                                    &self.listener,
                                    &self.records,
                                    &self.topics,
                                    &self.center,
                                );
                            }
                            Class::Unsubscribe => {
                                log::trace!("incoming unsubscribe message for local topic");
                                Switch::handle_unsubscribe(
                                    t,
                                    &self.listener,
                                    &self.records,
                                    &self.topics,
                                    &self.center,
                                );
                            }
                            // TODO: Handle lookup!
                            _ => {
                                let _ = self.listener.send(t);
                            }
                        }
                    }
                }
            }
        });
    }

    fn handle_ping(t: Transaction, channel: &Channel<Transaction>, center: &Center) {
        let node = Node::new(center.public.clone(), Some(center.link.clone()));
        let message = Message::new(
            Class::Details,
            center.public.clone(),
            t.source(),
            Address::default(),
            node.as_bytes(),
        );
        let transaction = Transaction::new(message);
        let _ = channel.send(transaction);
    }

    fn handle_pong(t: Transaction, channel: &Channel<SignalingAction>) {
        let _ = channel.send(SignalingAction::pong(t.source(), t.uuid));
    }

    fn handle_lookup(t: Transaction, listener: &Channel<Transaction>, center: &Center) {
        let node = Node::new(center.public.clone(), Some(center.link.clone()));
        let message = Message::new(
            Class::Details,
            center.public.clone(),
            t.source(),
            Address::default(),
            node.as_bytes(),
        );
        let transaction = Transaction::new(message);
        let _ = listener.send(transaction);
    }

    fn handle_details(t: Transaction, channel: &Channel<SignalingAction>, table: &Safe) {
        if let Ok(node) = Node::from_bytes(t.message.body.as_bytes()) {
            table.add(node);
            let action = SignalingAction::pong(t.source(), t.uuid);
            let _ = channel.send(action);
        } else {
            log::warn!("received invalid node details: {:?}", t);
        }
    }

    fn handle_action(
        t: Transaction,
        topics: &RefCell<TopicBucket>,
        interface: &Channel<InterfaceAction>,
    ) {
        if let Some(simple) = topics.borrow().find(&t.topic()) {
            let command = Command::Message(t);
            let _ = simple.channel.send(command);
        } else {
            let action = InterfaceAction::Message(t);
            let _ = interface.send(action);
        }
    }

    fn handle_subscriber(t: Transaction, topics: &RefCell<TopicBucket>, center: &Center) {
        if let Some(simple) = topics.borrow().find(&t.topic()) {
            let addrs = Address::from_bulk(t.message.body.as_bytes());
            for sub in addrs {
                if sub != center.public {
                    let action = Command::Subscriber(sub);
                    let _ = simple.channel.send(action);
                }
            }
        }
    }

    fn handle_unsubscriber(t: Transaction, topics: &RefCell<TopicBucket>) {
        if let Some(simple) = topics.borrow().find(&t.topic()) {
            let action = Command::Subscriber(t.source());
            let _ = simple.channel.send(action);
        }
    }

    fn handle_subscribe(
        t: Transaction,
        listener: &Channel<Transaction>,
        records: &RecordBucket,
        topics: &RefCell<TopicBucket>,
        center: &Center,
    ) {
        let topic = t.target();
        match records.get(&topic) {
            Some(record) => {
                records.subscribe(&topic, t.source());
                let mut subscribers = Vec::new();
                record
                    .subscribers
                    .iter()
                    .for_each(|x| subscribers.append(&mut x.as_bytes().to_vec()));
                subscribers.append(&mut t.source().as_bytes().to_vec());
                if topics.borrow().is_local(&topic) {
                    let message = Message::new(
                        Class::Subscriber,
                        t.topic(),
                        t.source(),
                        topic.clone(),
                        subscribers.clone(),
                    );
                    let transaction = Transaction::new(message);
                    Switch::handle_subscriber(transaction, topics, center)
                }
                for addr in record.subscribers {
                    let message = Message::new(
                        Class::Subscriber,
                        t.source(),
                        addr,
                        topic.clone(),
                        subscribers.clone(),
                    );
                    let transaction = Transaction::new(message);
                    let _ = listener.send(transaction);
                }
            }
            None => {
                let mut record = Record::new(topic.clone());
                record.subscribe(t.source());
                records.add(record);
            }
        }
    }

    fn handle_unsubscribe(
        t: Transaction,
        listener: &Channel<Transaction>,
        records: &RecordBucket,
        topics: &RefCell<TopicBucket>,
        center: &Center,
    ) {
        let topic = t.target();
        match records.get(&topic) {
            Some(record) => {
                let source = t.source();
                records.unsubscribe(&topic, &t.source());
                let mut subscribers = Vec::new();
                record.subscribers.iter().for_each(|x| {
                    if x != &source {
                        subscribers.append(&mut x.as_bytes().to_vec())
                    }
                });
                if topics.borrow().is_local(&topic) {
                    let message = Message::new(
                        Class::Unsubscriber,
                        t.topic(),
                        t.source(),
                        topic.clone(),
                        subscribers.clone(),
                    );
                    let transaction = Transaction::new(message);
                    Switch::handle_subscriber(transaction, topics, center)
                }
                for addr in record.subscribers {
                    if addr != source {
                        let message = Message::new(
                            Class::Unsubscriber,
                            t.source(),
                            addr,
                            topic.clone(),
                            subscribers.clone(),
                        );
                        let transaction = Transaction::new(message);
                        let _ = listener.send(transaction);
                    }
                }
            }
            None => {}
        }
    }
}
