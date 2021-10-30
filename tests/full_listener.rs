//! A full test of the entire listener architecture.

use actaeon::handler::Listener;
use actaeon::message::Message;
use actaeon::node::{Address, Center, Node};
use actaeon::router::Safe;
use actaeon::transaction::{Class, Transaction, Wire};
use actaeon::util::Channel;
use sodiumoxide::crypto::box_;
use std::io::Write;
use std::net::TcpStream;

#[test]
fn test_full_double() {
    let local = gen_listener(42433);
    let remote = gen_listener(42434);
}

fn gen_listener(port: usize) -> Channel<Transaction> {
    let (r1, r2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let rcenter = Center::new(secret, String::from("127.0.0.1"), port);
    let rtable = Safe::new(42, rcenter.clone());
    let rlistener = Listener::new(rcenter.clone(), r1, 10, rtable).unwrap();
    let _ = rlistener.start();
    return r2;
}
