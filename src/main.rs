use actaeon::config::Config;
use actaeon::instance::Instance;
use actaeon::router::node::Node;
use std::error::Error;
use tracing::Level;

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_file("config.toml");
    init(&config);
    let this = Node::new("abc", 42, "self");
    let instance = Instance::new(config, this);
    tracing::info!(instance.config.limit);
    Ok(())
}

fn init(config: &Config) {
    let level = match config.log.as_str() {
        "TRACE" => Level::TRACE,
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        _ => {
            panic!("Config not valid!");
        }
    };
    tracing_subscriber::fmt().with_max_level(level).init();
}
