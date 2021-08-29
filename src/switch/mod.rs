//! # Switch

pub mod adapter;
use crate::transaction::Transaction;
use std::sync::mpsc::{Receiver, Sender};

/// Each message being sent between the Listener Thead and User Thread
/// is either a UserAction or a SwitchAction, only UserAction get
/// actually passed onto the user, others are internal types for
/// managing the thread and the channel.
pub enum Command {
    /// mpsc element for internal communication, like shutting down
    /// the thread or any issues.
    SwitchAction(Action),
    /// Messages coming from the network intended for the user.
    UserAction(Transaction),
}

/// A collection of possible events for the channel. These are not
/// error messages, since they should be handled automatically, but in
/// the future this part of the system should be made more open by
/// using traits and giving the user the option to specify how to
/// handle all kinds of issues and messages.
pub enum Action {
    /// If the network connection has been terminated or failed. This
    /// only referres to the network connection, not the channel.
    Terminated,
    /// A heads up message should the cache be full. The system should
    /// handle this automatically, but it can be helpful to be
    /// informed.
    CacheFull,
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
struct Cache {
    /// All current Transactions in the cache. Instead of only storing
    /// the messages the entire transactions will get stored, which
    /// should make comparisons faster for larger objects. The array
    /// will be sorted by age on every update.
    elements: Vec<Transaction>,
    /// The maximum size of the cache in number of elements. Once the
    /// size has been reached the oldest element will get dropped to
    /// make space for new Transactions.
    limit: usize,
}

pub struct SwitchInterface {
    channel: Receiver<Command>,
}

pub struct Switch {
    channel: Sender<Command>,
    recent: Cache,
    handler: Box<dyn adapter::Adapter>,
}
