use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

// fn init(config: &Config) {
//     let level = match config.log.as_str() {
//         "TRACE" => Level::TRACE,
//         "DEBUG" => Level::DEBUG,
//         "INFO" => Level::INFO,
//         "WARN" => Level::WARN,
//         "ERROR" => Level::ERROR,
//         _ => {
//             panic!("Config not valid!");
//         }
//     };
//     tracing_subscriber::fmt().with_max_level(level).init();
// }
