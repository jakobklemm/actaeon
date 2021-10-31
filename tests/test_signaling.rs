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
fn test_manual_bootstrap() {
    // local
    let (w1, _) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 42435);
    let table = Safe::new(42, center.clone());
    let test_node = Node::new(
        Address::random(),
        Some(Link::new(String::from("8.8.8.8"), 789)),
    );
    table.add(test_node.clone());
    let another_node = Node::new(
        Address::random(),
        Some(Link::new(String::from("1.1.1.1"), 12345)),
    );
    table.add(another_node.clone());
    let center_node = Node::new(center.public.clone(), Some(center.link.clone()));
    let signaling = Signaling::new(String::from("127.0.0.1"), 12345);
    let listener = Listener::new(center, w1, 10, table, signaling).unwrap();
    let _ = listener.start();

    // remote
    let mut conn = TcpStream::connect("127.0.0.1:42435").unwrap();

    // init bootsrap
    let _ = conn.write(&[0; 142]);
    let mut len = [0; 2];
    let _ = conn.read(&mut len);
    let length = actaeon::util::integer(len);
    let mut nodes = vec![0; length];
    let _ = conn.read_exact(&mut nodes);
    let nodes = Node::from_bulk(nodes);
    assert_eq!(nodes, vec![test_node, another_node, center_node]);
}

#[test]
fn test_auto_bootstrap() {
    let test_node = Node::new(
        Address::random(),
        Some(Link::new(String::from("example.com"), 45678)),
    );

    let (w1, _) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let lcenter = Center::new(secret, String::from("127.0.0.1"), 42437);
    let rnode = Node::new(lcenter.public.clone(), Some(lcenter.link.clone()));
    let ltable = Safe::new(42, lcenter.clone());
    ltable.add(test_node.clone());
    let signaling = Signaling::new(String::from("127.0.0.1"), 42438);
    let llistener = Listener::new(lcenter.clone(), w1, 10, ltable.clone(), signaling).unwrap();
    let _ = llistener.start();

    std::thread::sleep(std::time::Duration::from_millis(25));

    // remote
    let (r1, _) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let rcenter = Center::new(secret, String::from("127.0.0.1"), 42438);
    let rtable = Safe::new(42, rcenter.clone());
    let signaling = Signaling::new(String::from("127.0.0.1"), 42437);
    let rlistener = Listener::new(rcenter.clone(), r1, 10, rtable.clone(), signaling).unwrap();
    let _ = rlistener.start();

    std::thread::sleep(std::time::Duration::from_millis(25));

    let found = rtable.get_copy(&Address::random(), 5);
    assert_eq!(found.len(), 2);
    assert_eq!(found, vec![test_node, rnode]);
}

#[test]
fn test_auto_messaging() {
    let (w1, w2) = Channel::new();
    let (_, secret) = box_::gen_keypair();
    let lcenter = Center::new(secret, String::from("127.0.0.1"), 42441);
    let target = lcenter.public.clone();
    let ltable = Safe::new(42, lcenter.clone());
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

    let message = Message::new(
        Class::Action,
        source.clone(),
        target.clone(),
        Address::random(),
        vec![42],
    );
    let t = Transaction::new(message);
    let _ = r2.send(t.clone());
    let rt = w2.recv().unwrap();
    assert_eq!(t, rt);

    let message = Message::new(
        Class::Action,
        target.clone(),
        source.clone(),
        Address::random(),
        vec![42],
    );
    let t = Transaction::new(message);
    let _ = w2.send(t.clone());
    let rt = r2.recv().unwrap();
    assert_eq!(t, rt);
}
