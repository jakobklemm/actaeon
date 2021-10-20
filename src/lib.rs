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
