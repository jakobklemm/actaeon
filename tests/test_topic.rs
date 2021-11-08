use actaeon::{self, config::Config, node::Center, Interface};

//#[test]
fn test_subscribe() {
    let port1 = 42450;
    let port2 = 42451;

    let lconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), port1);
    let lcenter = gen_center_near("127.0.0.1", port2);
    let linterface = Interface::new(lconfig, lcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));

    let rconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), port2);
    let rcenter = gen_center_far("127.0.0.1", port1);
    let rinterface = Interface::new(rconfig, rcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));

    let topic = Address::default();
    // the topic is guaranteed not to be on this node.
    let mut ltopic = linterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut rtopic = rinterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _ = ltopic.broadcast(vec![42]);
    let _ = rtopic.broadcast(vec![43]);

    let lret = ltopic.recv().unwrap();
    assert_eq!(lret.message.body.as_bytes(), vec![43]);

    let rret = rtopic.recv().unwrap();
    assert_eq!(rret.message.body.as_bytes(), vec![42]);

    let lsubs = ltopic.subscribers.clone();
    let rsubs = rtopic.subscribers.clone();

    assert_eq!(lsubs.into_iter().next(), Some(rcenter.public));
    assert_eq!(rsubs.into_iter().next(), Some(lcenter.public));
}

use sodiumoxide::crypto::box_;

#[test]
fn test_topic_multi() {
    let port1 = 42460;
    let port2 = 42461;

    let lconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), port1);
    let (_p1, s1) = box_::gen_keypair();
    let lcenter = Center::new(s1, "127.0.0.1".to_string(), port2);
    let linterface = Interface::new(lconfig, lcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let rconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), port2);
    let (_p1, s2) = box_::gen_keypair();
    let rcenter = Center::new(s2, "127.0.0.1".to_string(), port1);
    let rinterface = Interface::new(rconfig, rcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));

    let topic = Address::default();
    // the topic is guaranteed not to be on this node.
    let mut ltopic = linterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut rtopic = rinterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _ = ltopic.broadcast(vec![42]);
    let _ = rtopic.broadcast(vec![43]);

    let lret = ltopic.recv().unwrap();
    assert_eq!(lret.message.body.as_bytes(), vec![43]);

    let rret = rtopic.recv().unwrap();
    assert_eq!(rret.message.body.as_bytes(), vec![42]);

    for i in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = ltopic.broadcast(vec![i]);
        let rret = rtopic.recv().unwrap();
        assert_eq!(rret.message.body.as_bytes(), vec![i]);
    }

    for i in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = rtopic.broadcast(vec![i]);
        let lret = ltopic.recv().unwrap();
        assert_eq!(lret.message.body.as_bytes(), vec![i]);
    }
}

use actaeon::node::Address;
use actaeon::node::Link;
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;
use std::time::SystemTime;

fn gen_center_near(ip: &str, port: usize) -> Center {
    let secret = [1; 32];
    let secret = SecretKey::from_slice(&secret).unwrap();
    let public = [1; 32];

    Center {
        secret,
        public: Address::from_bytes(public),
        uptime: SystemTime::now(),
        link: Link::new(ip.to_string(), port),
    }
}

fn gen_center_far(ip: &str, port: usize) -> Center {
    let secret = [128; 32];
    let secret = SecretKey::from_slice(&secret).unwrap();
    let public = [128; 32];

    Center {
        secret,
        public: Address::from_bytes(public),
        uptime: SystemTime::now(),
        link: Link::new(ip.to_string(), port),
    }
}
