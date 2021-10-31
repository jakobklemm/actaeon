use actaeon::config::Signaling;
use actaeon::handler::Listener;
use actaeon::message::Message;
use actaeon::node::{Address, Center, Link, Node};
use actaeon::router::Safe;
use actaeon::transaction::{Class, Transaction};
use actaeon::util::Channel;
use sodiumoxide::crypto::box_;
use std::io::Write;
use std::io::*;
use std::net::TcpStream;

#[test]
fn test_subscribe() {
    let test_node = Node::new(
        Address::random(),
        Some(Link::new(String::from("example.com"), 45678)),
    );

    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let lcenter = Center::new(secret, String::from("127.0.0.1"), 42441);
    let target = lcenter.public.clone();
    let ltable = Safe::new(42, lcenter.clone());
    ltable.add(test_node.clone());
    let signaling = Signaling::new(String::from("127.0.0.1"), 42442);
    let llistener = Listener::new(lcenter.clone(), w1, 10, ltable.clone(), signaling).unwrap();
    let _ = llistener.start();

    std::thread::sleep(std::time::Duration::from_millis(25));

    // remote
    let (r1, r2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let rcenter = Center::new(secret, String::from("127.0.0.1"), 42442);
    let source = rcenter.public.clone();
    let rtable = Safe::new(42, rcenter.clone());
    let signaling = Signaling::new(String::from("127.0.0.1"), 42441);
    let rlistener = Listener::new(rcenter.clone(), r1, 10, rtable.clone(), signaling).unwrap();
    let _ = rlistener.start();

    std::thread::sleep(std::time::Duration::from_millis(25));

    let message = Message::new(Class::Action, source, target, Address::random(), vec![42]);
    let t = Transaction::new(message);
    let _ = r2.send(t.clone());
    let rt = w2.recv().unwrap();
    assert_eq!(t, rt);
}
