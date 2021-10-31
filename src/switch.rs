//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::message::Message;
use crate::node::{Address, Center, Node};
use crate::record::{Record, RecordBucket};
use crate::router::Safe;
use crate::signaling::{SignalingAction, Type};
use crate::topic::Command;
use crate::topic::TopicBucket;
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
    listener: Channel<Transaction>,
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
            topics: RefCell::new(TopicBucket::new(center.clone())),
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
                    match action {
                        InterfaceAction::Shutdown => {
                            log::info!("received shutdown command, terminating Switch.");
                            break;
                        }
                        InterfaceAction::Message(transaction) => {
                            let _ = self.listener.send(transaction);
                        }
                        InterfaceAction::Subscribe(simple) => {
                            let target = simple.address.clone();
                            self.topics.borrow_mut().add(simple);
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

                // 2. Listen on topics Chanel.
                for simple in &self.topics.borrow().topics {
                    let topic = simple.address.clone();
                    if let Some(command) = simple.channel.try_recv() {
                        match command {
                            Command::Drop(addr) => {
                                // The addr is of the user to send the
                                // unsubscribe to, not of the topic!
                                self.topics.borrow_mut().remove(&simple.address);
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
                            Command::Broadcast(addr, body) => {
                                let message = Message::new(
                                    Class::Unsubscribe,
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
                    match action.action {
                        Type::Ping => {
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
                    let target = t.target();
                    if target == self.center.public {
                        // Handle: Ping, Pong, Lookup, Details, Action, Subscriber, Unsubscriber
                        // Error: Subscriber, Unsubscribe
                        match t.class() {
                            Class::Ping => {
                                Switch::handle_ping(t, &self.listener, &self.center);
                            }
                            Class::Pong => {
                                Switch::handle_pong(t, &self.signaling);
                            }
                            Class::Lookup => {
                                Switch::handle_lookup(t, &self.listener, &self.center);
                            }
                            Class::Details => {
                                Switch::handle_details(t, &self.signaling, &self.table);
                            }
                            Class::Action => {
                                Switch::handle_action(t, &self.topics);
                            }
                            Class::Subscriber => {
                                Switch::handle_subscriber(t, &self.topics);
                            }
                            Class::Unsubscriber => {
                                Switch::handle_unsubscriber(t, &self.topics);
                            }
                            _ => {
                                log::warn!("received message to invalid target: {:?}", t);
                            }
                        }
                    } else {
                        // Forward: Ping, Pong, Details, Action, Subscriber, Unsubscriber,
                        // Maybe Handle: Subscribe, Unsubscribe, Lookup
                        match t.class() {
                            Class::Subscribe => {
                                Switch::handle_subscribe(t, &self.listener, &self.records);
                            }
                            Class::Unsubscribe => {
                                Switch::handle_unsubscribe(t, &self.listener, &self.records);
                            }
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

    fn handle_action(t: Transaction, topics: &RefCell<TopicBucket>) {
        if let Some(simple) = topics.borrow().find(&t.source()) {
            let command = Command::Message(t);
            let _ = simple.channel.send(command);
        }
    }

    fn handle_subscriber(t: Transaction, topics: &RefCell<TopicBucket>) {
        if let Some(simple) = topics.borrow().find(&t.topic()) {
            let addrs = Address::from_bulk(t.message.body.as_bytes());
            for sub in addrs {
                let action = Command::Subscriber(sub);
                let _ = simple.channel.send(action);
            }
        }
    }

    fn handle_unsubscriber(t: Transaction, topics: &RefCell<TopicBucket>) {
        if let Some(simple) = topics.borrow().find(&t.topic()) {
            let action = Command::Subscriber(t.source());
            let _ = simple.channel.send(action);
        }
    }

    fn handle_subscribe(t: Transaction, listener: &Channel<Transaction>, records: &RecordBucket) {
        let topic = t.topic();
        match records.get(&topic) {
            Some(record) => {
                records.subscribe(&topic, t.source());
                let mut subscribers = Vec::new();
                record
                    .subscribers
                    .iter()
                    .for_each(|x| subscribers.append(&mut x.as_bytes().to_vec()));
                subscribers.append(&mut t.source().as_bytes().to_vec());
                let message = Message::new(
                    Class::Subscriber,
                    t.source(),
                    t.source(),
                    topic.clone(),
                    subscribers.clone(),
                );
                let transaction = Transaction::new(message);
                let _ = listener.send(transaction);
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
                let record = Record::new(topic);
                records.add(record);
            }
        }
    }

    fn handle_unsubscribe(t: Transaction, listener: &Channel<Transaction>, records: &RecordBucket) {
        let topic = t.topic();
        match records.get(&topic) {
            Some(record) => {
                records.unsubscribe(&topic, &t.source());

                for sub in record.subscribers {
                    let message = Message::new(
                        Class::Subscriber,
                        t.source(),
                        sub,
                        topic.clone(),
                        Vec::new(),
                    );
                    let transaction = Transaction::new(message);
                    let _ = listener.send(transaction);
                }
            }
            None => {}
        }
    }
}
