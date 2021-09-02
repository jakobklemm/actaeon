//! # Connection
//!
//! The user can receive two different types of objects from the channel:
//!
//! - Transaction: A complete message / transaction from a different node.
//!   This is "fire and forget", meaning the node is no longer connected.
//!   There is also no way of knowing how the message reached the user
//!   since it might have been sent through the decentralized network.
//!
//! - Connection: A remote node can choose to connect directly to the
//!   local node and keep the connection alive. This is only possible for
//!   direct connections (for reliability reasons) but it allows for
//!   higher speeds, lower latencies and actual bi-directional
//!   communication. (TODO: End to end encryption for direect
//!   communication)
