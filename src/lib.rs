pub mod bucket;
pub mod config;
pub mod database;
pub mod error;
pub mod interface;
pub mod message;
pub mod node;
pub mod router;
pub mod signaling;
pub mod switch;
pub mod tcp;
pub mod topic;
pub mod transaction;

use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;
use std::time::SystemTime;

/// The central function for launching the entire application. It
/// requires the configuration and then takes care of starting all
/// threads. This function will panic if it fails for any reason. If
/// any special uses are required it is not recommended to use this
/// function.
pub fn start(config: config::Config, center: config::CenterConfig) -> interface::Interface {
    // Validate system time
    let now = SystemTime::now();
    log::info!("starting Actaeon application!");

    if now.elapsed().is_err() {
        log::error!("system time is configured incorrectly, unable to launch actaeon!");
        panic!("unable to launch actaeon");
    }

    match center.secret {
        Some(bytes) => {
            if let Some(key) = SecretKey::from_slice(&bytes) {
                let center = node::Center::new(key, center.ip, center.port);
                if let Ok((s, i)) = switch::Switch::new(center, config.clone(), config.cache) {
                    if s.start().is_err() {
                        log::error!("handler thread failed at launch!");
                        panic!("launch failed");
                    } else {
                        i
                    }
                } else {
                    log::error!("unable to launch actaeon!");
                    panic!("launch failed");
                }
            } else {
                log::error!("unable to launch actaeon!");
                panic!("launch failed");
            }
        }
        None => {
            log::error!("invalid Center configuration");
            panic!("configuration is invalid");
        }
    }
}

/// Loads all required configuration if the provided directory has the
/// proper structure:
/// / = provided path
/// /config.toml = Main configuration in toml format.
/// /center.toml = Variables describing the Center (this node).
/// /secret.key = Binary encoded secret key.
///
/// Currently this will simply panic should the configuration be
/// invalid.
pub fn config(dir: &str) -> (config::Config, config::CenterConfig) {
    let mut path = String::from(dir);
    if path.get(path.len() - 1..path.len()) != Some("/") {
        path.push_str("/");
    }
    let mut config_path = String::from(dir);
    config_path.push_str("config.toml");
    let config = config::Config::from_file(&config_path);

    let mut center_path = String::from(dir);
    center_path.push_str("center.toml");

    let center = config::CenterConfig::from_file(&center_path);

    let mut center_key_path = String::from(dir);
    center_key_path.push_str("secret.key");

    let key = config::CenterConfig::load_key(&center_key_path);

    if center.is_err() || config.is_err() || key.is_err() {
        log::error!("invalid configuration");
        panic!("configuration failed");
    }

    let config = config.unwrap();
    let center = center.unwrap();
    let key = key.unwrap();

    let center = config::CenterConfig::new(center.ip, center.port, key, center.hostname);

    (config, center)
}
