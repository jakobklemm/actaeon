use actaeon::{config::Config, node::Center, Interface};
use sodiumoxide::crypto::box_;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::new(20, 1, 100, "example.com".to_string(), 4242);
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 1234);

    let i = Interface::new(config, center).await.unwrap();

    assert!(i.recv() == None);
}
