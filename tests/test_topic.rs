use actaeon::{
    self,
    config::Config,
    node::{Address, Center},
    Interface,
};
use sodiumoxide::crypto::box_;

#[tokio::test]
async fn test_topic_multi() {
    let port1 = 42460;
    let port2 = 42461;

    let lconfig = Config::new(20, 10, 1000, "127.0.0.1".to_string(), port1);
    let lcenter = gen_center_near("127.0.0.1", port2);
    let linterface = Interface::new(lconfig, lcenter.clone()).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let rconfig = Config::new(20, 10, 1000, "127.0.0.1".to_string(), port2);
    let rcenter = gen_center_far("127.0.0.1", port1);
    let rinterface = Interface::new(rconfig, rcenter.clone()).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let topic = Address::default();
    // the topic is guaranteed not to be on this node.
    let mut rtopic = rinterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut ltopic = linterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));

    for i in 0..23 {
        std::thread::sleep(std::time::Duration::from_millis(16));
        let _ = ltopic.broadcast(vec![i]);
        let rret = rtopic.recv().unwrap();
        assert_eq!(rret.message.body.as_bytes(), vec![i]);
    }

    for i in 0..55 {
        std::thread::sleep(std::time::Duration::from_millis(16));
        let _ = rtopic.broadcast(vec![i]);
        let rret = ltopic.recv().unwrap();
        assert_eq!(rret.message.body.as_bytes(), vec![i]);
    }
}

use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

fn gen_center_near(ip: &str, port: usize) -> Center {
    let secret = [1; 32];
    let secret = SecretKey::from_slice(&secret).unwrap();
    let public = [1; 32];

    let b = [1; 32];
    let s = SecretKey::from_slice(&b).unwrap();
    let base = Center::new(s, ip.to_string(), port);

    Center {
        secret,
        public: Address::from_bytes(public),
        ..base
    }
}

fn gen_center_far(ip: &str, port: usize) -> Center {
    let secret = [127; 32];
    let secret = SecretKey::from_slice(&secret).unwrap();
    let public = [127; 32];

    let b = [127; 32];
    let s = SecretKey::from_slice(&b).unwrap();
    let base = Center::new(s, ip.to_string(), port);

    Center {
        secret,
        public: Address::from_bytes(public),
        ..base
    }
}

#[tokio::test]
async fn test_topic_random() {
    let port1 = 42270;
    let port2 = 42271;

    let lconfig = Config::new(20, 10, 1000, "127.0.0.1".to_string(), port1);
    let (_, s1) = box_::gen_keypair();
    let lcenter = Center::new(s1, "127.0.0.1".to_string(), port2);
    let linterface = Interface::new(lconfig, lcenter.clone()).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    let (_, s2) = box_::gen_keypair();
    let rconfig = Config::new(20, 10, 1000, "127.0.0.1".to_string(), port2);

    let rcenter = Center::new(s2, "127.0.0.1".to_string(), port1);
    let rinterface = Interface::new(rconfig, rcenter.clone()).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let topic = Address::random();
    // the topic is guaranteed not to be on this node.
    let mut rtopic = rinterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut ltopic = linterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));

    for i in 0..23 {
        std::thread::sleep(std::time::Duration::from_millis(8));
        let _ = ltopic.broadcast(vec![i]);
        std::thread::sleep(std::time::Duration::from_millis(8));
        let rret = rtopic.recv().unwrap();
        assert_eq!(rret.message.body.as_bytes(), vec![i]);
    }

    for i in 0..55 {
        std::thread::sleep(std::time::Duration::from_millis(8));
        let _ = rtopic.broadcast(vec![i]);
        std::thread::sleep(std::time::Duration::from_millis(8));
        let rret = ltopic.recv().unwrap();
        assert_eq!(rret.message.body.as_bytes(), vec![i]);
    }
}
