//! # Switch
//!
//! The switch is responsible for handling connections and messages as
//! well as the cache on a dedicated thread. This module does not
//! implement the actual listening logic, since that component is
//! supposed to be more modularized. Instead it handles the thread and
//! the cache, each protocol then has its own module.

use crate::error::Error;
use crate::interface::InterfaceAction;
use crate::message::Message;
use crate::node::{Address, Center, Link, Node};
use crate::record::{Record, RecordBucket};
use crate::router::Table;
use crate::signaling::SignalingAction;
use crate::topic::{Command, Topic};
use crate::topic::{Simple, TopicBucket};
use crate::transaction::{Class, Transaction, Wire};
use crate::util::Channel;
use std::cell::RefCell;
use std::thread;

/// Currently the system requires a dedicated thread for the listening
/// server, which will autoamtically get started. The thread will hold
/// a Switch object and send messages through the channel.
pub struct Switch {
    listener: Channel<Transaction>,
    interface: Channel<InterfaceAction>,
    signaling: Channel<SignalingAction>,
    /// Each Message will be sent out multiple times to ensure
    /// delivery, currently it is simply hard coded.
    replication: usize,
    /// The main copy of the couting table, which will be maintained
    /// by this Thread. It will have to be wrapped in a Arc Mutex to
    /// allow for the Updater Thread.
    table: RefCell<Table>,
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
    records: RefCell<RecordBucket>,
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
        replication: usize,
        limit: usize,
    ) -> Result<Self, Error> {
        let switch = Switch {
            listener,
            interface,
            signaling,
            replication,
            table: RefCell::new(Table::new(limit, center.clone())),
            topics: RefCell::new(TopicBucket::new(center.clone())),
            records: RefCell::new(RecordBucket::new()),
            center: center,
        };
        Ok(switch)
    }

    /// The switch is responsible for deciding where specific messages
    /// go based on their origin, target and type. It listens on
    /// almost all Channels in the system and can send messages to any
    /// sink.
    pub fn start(mut self) {
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
                // 4. Listen on Handler Channel.
                if let Some(t) = self.listener.try_recv() {
                    let source = t.source();
                    let target = t.target();
                    if target == self.center.public {
                        // Handle: Ping, Pong, Lookup, Details, Action, Subscriber, Unsubscriber
                        // Error: Subscriber, Unsubscribe
                        match t.class() {
                            Class::Ping => {}
                            Class::Pong => {}
                            Class::Lookup => {}
                            Class::Details => {}
                            Class::Action => {}
                            Class::Subscribe => {}
                            Class::Unsubscribe => {}
                            Class::Subscriber => {
                                log::warn!("received message to invalid target: {:?}", t);
                            }
                            Class::Unsubscriber => {
                                log::warn!("received message to invalid target: {:?}", t);
                            }
                        }
                    } else {
                        // Forward: Ping, Pong, Lookup, Details, Action, Subscriber, Unsubscriber,
                        // Maybe Handle: Subscribe, Unsubscribe
                        match t.class() {
                            Class::Ping => {
                                let _ = self.listener.send(t);
                            }
                            Class::Pong => {
                                let _ = self.listener.send(t);
                            }
                            Class::Lookup => {
                                let _ = self.listener.send(t);
                            }
                            Class::Details => {
                                let _ = self.listener.send(t);
                            }
                            Class::Action => {
                                let _ = self.listener.send(t);
                            }
                            Class::Subscribe => {}
                            Class::Unsubscribe => {}
                            Class::Subscriber => {
                                let _ = self.listener.send(t);
                            }
                            Class::Unsubscriber => {
                                let _ = self.listener.send(t);
                            }
                        }
                    }
                }
            }
        });
    }
}
