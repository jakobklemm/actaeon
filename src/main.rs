use actaeon::config::Config;
use actaeon::router::address::Address;
use actaeon::router::node::Node;
use actaeon::router::table::Table;
use std::error::Error;
use tracing::Level;

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_file("config.toml");
    init(config);
    tracing::info!("Actaeon starting up!");
    let home = Node::new("", 0, Address::new(""));
    let mut table = Table::new(home, 20);
    let (f, n, l) = nodes();
    let x = f.clone();
    table.add(f);
    table.add(l);
    table.add(x);
    table.run(&n, 20, |i| tracing::info!("{:?}", i.print()));
    tracing::error!("error");
    tracing::trace!("trace");
    Ok(())
}

fn init(config: Config) {
    let level = match config.log.as_str() {
        "TRACE" => Level::TRACE,
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        _ => panic!("Config not valid!"),
    };
    tracing_subscriber::fmt().with_max_level(level).init();
}

fn nodes() -> (Node<'static>, Node<'static>, Node<'static>) {
    let mut b = [0; 32];
    b[1] = 32;
    let first = Address::from_message(b);

    let mut b = [0; 32];
    b[0] = 42;
    let second = Address::from_message(b);

    let mut b = [0; 32];
    b[0] = 132;
    let third = Address::from_message(b);

    let first = Node::new("", 0, first);
    let second = Node::new("", 0, second);
    let third = Node::new("", 0, third);
    (first, second, third)
}
