use actaeon::config::Config;
use actaeon::instance::Instance;
use actaeon::router;

fn main() {
    let config = Config::new("127.0.0.1", 4242, 255);
    let i = Instance::new(&config);
    let a = router::address::Address::new("hello world");
    let n = router::node::Node::new("192.168.1.25", 1234, a);
    let b = router::bucket::Bucket::new(n);
    println!("{}", i.config.port);
    println!("{:?}", b.nodes[0].port);
}
