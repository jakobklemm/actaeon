//! A full test of the entire listener architecture.

use actaeon::config::Signaling;
use actaeon::handler::Listener;
use actaeon::node::Center;
use actaeon::router::Safe;
use actaeon::transaction::Transaction;
use actaeon::util::Channel;
use sodiumoxide::crypto::box_;

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
    let signaling = Signaling::new(String::from("127.0.0.1"), 12345);
    let rlistener = Listener::new(rcenter.clone(), r1, 10, rtable, signaling).unwrap();
    let _ = rlistener.start();
    return r2;
}
