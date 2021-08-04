use actaeon::config::Config;

use actaeon::router::address::Address;
use actaeon::router::node::Node;
use actaeon::router::table::Table;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Actaeon starting up!");
    let home = Node::new("", 0, Address::new(""));
    let mut table = Table::new(home, 20);
    let (f, n) = nodes();
    table.add(f);
    table.run(&n, 5, |i| tracing::info!("{}", i.print()));
    Ok(())
}

fn nodes() -> (Node<'static>, Node<'static>) {
    let mut b = [0; 32];
    b[1] = 32;
    let first = Address {
        bytes: b,
        public: "aoeu",
    };

    let mut b = [0; 32];
    b[0] = 42;
    let second = Address {
        bytes: b,
        public: "aoeu",
    };
    let first = Node::new("", 0, first);
    let second = Node::new("", 0, second);
    (first, second)
}
