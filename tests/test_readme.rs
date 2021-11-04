// Copy of the README code for testing.

use actaeon::{
    config::Config,
    node::{Center, ToAddress},
    Interface,
};
use sodiumoxide::crypto::box_;

#[test]
fn test_main() {
    let config = Config::new(20, 1, 100, "example.com".to_string(), 4242);
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 1234);

    let interface = Interface::new(config, center).unwrap();

    let mut topic = interface.subscribe(&"example".to_string().to_address());

    let _ = topic.broadcast("hello world".as_bytes().to_vec());
}
