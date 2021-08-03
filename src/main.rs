use actaeon::config::Config;

use actaeon::instance::Instance;
use actaeon::router;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Actaeon starting up!");
    let config = Config::from_file("config.toml");
    let _i = Instance::new(config);
    let a = router::address::Address::new("hello world");
    let _n = router::node::Node::new("192.168.1.25", 1234, a);
    let a = router::address::Address::new("hello world");
    let _n2 = router::node::Node::new("127.0.0.1", 4321, a);
    let a = router::address::Address::new("hello world");
    let n3 = router::node::Node::new("127.0.0.1", 4321, a);
    let mut b = router::bucket::Bucket::new(n3);
    b.sort();
    Ok(())
}
