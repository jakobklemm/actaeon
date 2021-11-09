use actaeon::{
    self,
    config::Config,
    message::Message,
    node::{Address, Center},
    transaction::{Class, Transaction},
    Interface,
};

use sodiumoxide::crypto::box_;

#[test]
fn test_interface() {
    let lconfig = Config::new(20, 5, 100, "127.0.0.1".to_string(), 42443);
    let (_, secret) = box_::gen_keypair();
    let lcenter = Center::new(secret, String::from("127.0.0.1"), 42444);

    let linterface = Interface::new(lconfig, lcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let rconfig = Config::new(20, 5, 100, "127.0.0.1".to_string(), 42444);
    let (_, secret) = box_::gen_keypair();
    let rcenter = Center::new(secret, String::from("127.0.0.1"), 42443);

    let rinterface = Interface::new(rconfig, rcenter.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let test = Transaction::new(Message::new(
        Class::Action,
        lcenter.public.clone(),
        rcenter.public.clone(),
        Address::default(),
        vec![42],
    ));

    let _ = linterface.send(test.clone());

    let ret = rinterface.recv().unwrap();
    assert_eq!(ret, test);

    let test = Transaction::new(Message::new(
        Class::Action,
        rcenter.public.clone(),
        lcenter.public.clone(),
        Address::default(),
        vec![43],
    ));

    let _ = rinterface.send(test.clone());

    let ret = linterface.recv().unwrap();
    assert_eq!(ret, test);
}
