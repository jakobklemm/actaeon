use actaeon::handler::Listener;
use actaeon::message::Message;
use actaeon::node::{Address, Center, Node};
use actaeon::transaction::{Class, Transaction, Wire};
use actaeon::util::Channel;
use sodiumoxide::crypto::box_;
use std::io::Write;
use std::net::TcpStream;

#[test]
fn test_tcp_init() {
    // local
    let (w1, w2) = Channel::<Wire>::new();
    let (n1, n2) = Channel::<Node>::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42424);
    let listener = Listener::new(center, w1, n1, 10).unwrap();
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
    let recv_node = n2.recv().unwrap();
    assert_eq!(recv_wire, wire);
    assert_eq!(recv_node.as_bytes(), node.as_bytes());
}

#[test]
fn test_tcp_message() {
    let (w1, w2) = Channel::<Wire>::new();
    let (n1, n2) = Channel::<Node>::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42425);
    let listener = Listener::new(center, w1, n1, 10).unwrap();
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
    let mut conn = TcpStream::connect("127.0.0.1:42425").unwrap();
    let _ = conn.write(&wire.as_bytes());
    let _ = conn.write(&node.as_bytes());

    let _ = w2.recv();
    let _ = n2.recv();

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
    println!("test data: {:?}", wire);
    println!("length data: {:?}", wire.as_bytes().len());
    println!("body length data: {:?}", t.message.body.len());
    let _ = conn.write(&wire.as_bytes());
    let ret = w2.recv().unwrap();
    println!("received data: {:?}", ret);
    assert_eq!(ret, wire);
}

#[test]
fn test_tcp_random() {
    let (w1, w2) = Channel::<Wire>::new();
    let (n1, n2) = Channel::<Node>::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42426);
    let listener = Listener::new(center, w1, n1, 10).unwrap();
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
    let _ = n2.recv();

    // At this point the connection is general purpose and
    // bidirectional.

    for i in 0..1000 {
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
        assert_eq!(ret, wire);
    }
}
