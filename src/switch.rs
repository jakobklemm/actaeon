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
use crate::router::Table;
use crate::signaling::SignalingAction;
use crate::topic::Topic;
use crate::topic::{Record, RecordBucket, TopicBucket};
use crate::transaction::{Class, Transaction, Wire};
use crate::util::Channel;
use std::cell::RefCell;
use std::thread;

/// Each message being sent between the Listener Thead and User Thread
/// is either a UserAction or a SwitchAction, only UserAction get
/// actually passed onto the user, others are internal types for
/// managing the thread and the channel.
pub enum Command {
    /// mpsc element for internal communication, like shutting down
    /// the thread or any issues.
    Switch(SwitchAction),
    /// Messages coming from the network intended for the user (or the
    /// other way around).
    User(Transaction),
    /// Internal messages with incomplete data, that will be completed
    /// on the Handler Thread.
    System(SystemAction),
}

/// A collection of possible events for the channel. These are not
/// error messages, since they should be handled automatically, but in
/// the future this part of the system should be made more open by
/// using traits and giving the user the option to specify how to
/// handle all kinds of issues and messages.
#[derive(Debug)]
pub enum SwitchAction {
    /// If the network connection has been terminated or failed. This
    /// only referres to the network connection, not the channel. It
    /// can also be used to signal termination to the thread.
    Terminate,
}

pub enum SystemAction {
    /// A new topic gets created on the user thread, the channel has
    /// to be sent to the handler thread.
    Subscribe(Topic),
    /// Usually coming from the Thread to the user informing them of
    /// a new subscriber.
    Subscriber(Vec<Address>),
    /// Removes the topic from the tables & distributes the update.
    Unsubscribe(Address),
    /// Almost the same as UserAction, but in the topic the center
    /// might be unknown, so just the body can be transfered.
    Send(Address, Vec<u8>),
}

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
            records: RefCell::new(RecordBucket::new(center.clone())),
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
                        InterfaceAction::Message(transaction) => {}
                        InterfaceAction::Subscribe(addr) => {}
                    }
                }
                // 2. Listen on Signaling Channel.
                // 3. Listen on Handler Channel.
            }
        });
    }
}
