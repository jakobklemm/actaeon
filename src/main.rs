use actaeon::config::Config;
use actaeon::instance::Instance;
use actaeon::router;

use std::thread::sleep;
use std::time::Duration;

fn main() {
    let config = Config::new("127.0.0.1", 4242, 255);
    let i = Instance::new(&config);
    let a = router::address::Address::new("hello world");
    let n = router::node::Node::new("192.168.1.25", 1234, a);
    let one_sec = Duration::from_secs(1);
    sleep(one_sec);
    let a = router::address::Address::new("hello world");
    let n2 = router::node::Node::new("127.0.0.1", 4321, a);
    let a = router::address::Address::new("hello world");
    let n3 = router::node::Node::new("127.0.0.1", 4321, a);
    let mut b = router::bucket::Bucket::new(n3);
    println!("{}", i.config.port);
    println!("{:?}", n > n2);
    b.sort();
    println!("{:?}", b.nodes)
}
