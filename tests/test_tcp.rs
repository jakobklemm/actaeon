use actaeon::handler::{HandlerAction, Listener};
use actaeon::message::Message;
use actaeon::node::{Address, Center, Node};
use actaeon::router::Safe;
use actaeon::transaction::{Class, Transaction, Wire};
use actaeon::util::Channel;
use sodiumoxide::crypto::box_;
use std::io::Write;
use std::net::TcpStream;

#[test]
fn test_tcp_init() {
    // local
    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42424);
    let table = Safe::new(42, center.clone());
    let listener = Listener::new(center, w1, 10, table).unwrap();
    let _ = listener.start();

    // message
    let message = Message::new(
        Class::Action,
        Address::random(),
        Address::random(),
        String::from("test body").as_bytes().to_vec(),
    );
    let t = Transaction::new(message);
    let wire = t.to_wire();

    // [116, 101, 115, 116, 32, 98, 111, 100, 121]
    // [116, 101, 115, 116, 32, 98, 111, 100, 121]

    // remote
    let (_, secret) = box_::gen_keypair();
    let remote = Center::new(secret, String::from("8.8.8.8"), 12345);
    let link = remote.link.clone();
    let node = Node::new(remote.public.clone(), Some(link));
    let mut conn = TcpStream::connect("127.0.0.1:42424").unwrap();
    let _ = conn.write(&wire.as_bytes());
    let _ = conn.write(&node.as_bytes());

    // verify
    let recv_wire = w2.recv().unwrap();
    assert_eq!(recv_wire, HandlerAction::Message(wire));
}

#[test]
fn test_tcp_message() {
    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42425);
    let table = Safe::new(42, center.clone());
    let listener = Listener::new(center, w1, 10, table).unwrap();
    let _ = listener.start();

    // message
    let message = Message::new(
        Class::Action,
        Address::random(),
        Address::random(),
        String::from("test body").as_bytes().to_vec(),
    );
    let t = Transaction::new(message);
    let wire = t.to_wire();

    // [116, 101, 115, 116, 32, 98, 111, 100, 121]
    // [116, 101, 115, 116, 32, 98, 111, 100, 121]

    // remote
    let (_, secret) = box_::gen_keypair();
    let remote = Center::new(secret, String::from("8.8.8.8"), 12345);
    let link = remote.link.clone();
    let node = Node::new(remote.public.clone(), Some(link));
    let mut conn = TcpStream::connect("127.0.0.1:42425").unwrap();
    let _ = conn.write(&wire.as_bytes());
    let _ = conn.write(&node.as_bytes());

    let _ = w2.recv();

    // At this point the connection is general purpose and
    // bidirectional.

    let message = Message::new(
        Class::Action,
        Address::random(),
        Address::random(),
        "message".to_string().as_bytes().to_vec(),
    );
    let t = Transaction::new(message);
    let wire = t.to_wire();
    let _ = conn.write(&wire.as_bytes());
    let ret = w2.recv().unwrap();
    assert_eq!(ret, HandlerAction::Message(wire));
}

#[test]
fn test_tcp_random() {
    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42426);
    let table = Safe::new(42, center.clone());
    let listener = Listener::new(center, w1, 10, table).unwrap();
    let _ = listener.start();

    // message
    let message = Message::new(
        Class::Action,
        Address::random(),
        Address::random(),
        String::from("test body").as_bytes().to_vec(),
    );
    let t = Transaction::new(message);
    let wire = t.to_wire();

    // remote
    let (_, secret) = box_::gen_keypair();
    let remote = Center::new(secret, String::from("8.8.8.8"), 12345);
    let link = remote.link.clone();
    let node = Node::new(remote.public.clone(), Some(link));
    let mut conn = TcpStream::connect("127.0.0.1:42426").unwrap();
    let _ = conn.write(&wire.as_bytes());
    let _ = conn.write(&node.as_bytes());

    let _ = w2.recv();

    // At this point the connection is general purpose and
    // bidirectional.

    for i in 0..100 {
        let message = Message::new(
            Class::Action,
            Address::random(),
            Address::random(),
            i.to_string().as_bytes().to_vec(),
        );
        let t = Transaction::new(message);
        let wire = t.to_wire();
        let _ = conn.write(&wire.as_bytes());
        let ret = w2.recv().unwrap();
        assert_eq!(ret, HandlerAction::Message(wire));
    }
}

#[test]
fn test_tcp_outgoing() {
    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let lcenter = Center::new(secret, String::from("127.0.0.1"), 42427);
    let lnode = Node::new(lcenter.public.clone(), Some(lcenter.link.clone()));
    let ltable = Safe::new(42, lcenter.clone());
    let llistener = Listener::new(lcenter.clone(), w1, 10, ltable).unwrap();
    let _ = llistener.start();

    // remote
    let (r1, r2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let rcenter = Center::new(secret, String::from("127.0.0.1"), 42428);
    let rtable = Safe::new(42, rcenter.clone());
    rtable.add(lnode);
    let rlistener = Listener::new(rcenter.clone(), r1, 10, rtable).unwrap();
    let _ = rlistener.start();

    // message
    let message = Message::new(
        Class::Action,
        rcenter.public.clone(),
        lcenter.public.clone(),
        String::from("test body").as_bytes().to_vec(),
    );
    let t = Transaction::new(message);

    let _ = r2.send(HandlerAction::Incoming(t.clone()));
    let rett = w2.recv().unwrap();

    assert_eq!(rett, HandlerAction::Message(t.to_wire()));
}
