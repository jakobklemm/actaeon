use actaeon::{self, config::Config, node::Center, Interface};

#[test]
fn test_subscribe() {
    let lconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), 42445);
    let lcenter = gen_center_near("127.0.0.1", 42446);

    let linterface = Interface::new(lconfig, lcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let rconfig = Config::new(20, 1, 100, "127.0.0.1".to_string(), 42446);
    let rcenter = gen_center_far("127.0.0.1", 42445);

    let rinterface = Interface::new(rconfig, rcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    println!("data Local: {:?}", lcenter.public.clone());
    println!("data Remote: {:?}", rcenter.public.clone());

    let topic = Address::default();
    println!("topic data: {:?}", topic);
    // the topic is guaranteed not to be on this node.
    let mut ltopic = linterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    println!("data: - - -- - - -- - ");
    let mut rtopic = rinterface.subscribe(&topic);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _ = ltopic.broadcast(vec![42]);
    let _ = rtopic.broadcast(vec![43]);

    std::thread::sleep(std::time::Duration::from_millis(10));

    let lret = ltopic.recv().unwrap();
    assert_eq!(lret.message.body.as_bytes(), vec![43]);

    let rret = rtopic.recv().unwrap();
    assert_eq!(rret.message.body.as_bytes(), vec![42]);

    assert_eq!(ltopic.subscribers.into_iter().next(), Some(rcenter.public));
    assert_eq!(rtopic.subscribers.into_iter().next(), Some(lcenter.public));
}

use actaeon::node::Address;
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
        public: Address::from_bytes(public).unwrap(),
        ..base
    }
}

fn gen_center_far(ip: &str, port: usize) -> Center {
    let secret = [128; 32];
    let secret = SecretKey::from_slice(&secret).unwrap();
    let public = [128; 32];

    let b = [0; 32];
    let s = SecretKey::from_slice(&b).unwrap();
    let base = Center::new(s, ip.to_string(), port);

    Center {
        secret,
        public: Address::from_bytes(public).unwrap(),
        ..base
    }
}
