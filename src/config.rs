//! # Config
//!
//! The config is split into two parts: - System: General
//! "system-wide" config for all major components. This section is at
//! least partially meant to be manipulated by the user directly and
//! includes sections like the signaling config and logging setups. -
//! Center: The config describing this node itself, known as the
//! "center". (This is a decentralized network but from the
//! perspective of each node it is the center of it's own routing
//! tree.) This also includes sections like the connection info of the
//! node, which currently need to be manually set by the user. In the
//! future this should get replaced by some sort of setup script or
//! automatically handled in the signaling config.

use crate::error::Error;
use serde::Deserialize;
use std::fs;

use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::SecretKey;

/// Config values for the config of networking parameters if the
/// config is loaded from the default toml file. The values will
/// usually come from the config file. Others might get populated by
/// the code in order to prevent the user from chaning parameters.
#[derive(Deserialize)]
struct Network {
    /// serde deserialization value for the config file.
    bucket: usize,
    /// serde deserialization value for the config file.
    signaling: Vec<String>,
    /// serde deserialization value for the config file.
    /// TODO: Move cache to section "local" with additional fields.
    cache: usize,
}

/// The current config only contains details about the network. In the
/// default toml file this is still stored as a section, therefor
/// requiring a dedicated struct.
#[derive(Deserialize)]
struct LoadConfig {
    /// Only section in the system config toml file.
    network: Network,
}

/// The main system config for the entire system. It is created here
/// and then passed to the different modules. Currently the config is
/// just a collection of fields, in the future this might get replaced
/// by a collection of structs that keep the fields in their
/// categories. The fields here will already be processed and
/// converted, so the function might fail. Depending on how much is
/// eventually handled by the application instead of the implementer
/// it might also contain details about logging.
pub struct Config {
    /// Defines the size value of the kademlia based routing system,
    /// comparable to the variable "k". It defines the size of each
    /// bucket, the maximum number of buckets is currently not
    /// configurable and defined through the hashing / address system.
    pub bucket: usize,
    /// Array of signaling servers, used to connect to the system
    /// initially and possibly provide forwarding. TODO: Replace
    /// String with signaling struct and update toml file with new
    /// string format.
    pub signaling: Vec<String>,
    /// Maximum number of arguments in the Transaction cache in the
    /// Actaeon Process.
    pub cache: usize,
}

/// The center config can be loaded from a dedicated file, therefore a
/// simplified struct is needed. The toml file must list all fields
/// directly, except for the secret key, which must be stored as a
/// dedicated key file and can't (currently) be UTF-8 encoded. The
/// secret key file (currently) has to be stored in the same directory
/// as the center file.
#[derive(Deserialize)]
struct LoadCenter {
    /// IP without any protocols (https, tcp, etc). Domains (should)
    /// also work.
    ip: String,
    /// Port must be available and it has to be possible to bind to
    /// at.
    port: u8,
    /// Path to the secret key file in the same directory.
    secret: String,
    /// Username to use when possible instead of the routing ID.
    hostname: String,
}

/// The local node is configured separately, since some of the values
/// will have to be set manually or optained through an external
/// method.
pub struct Center {
    /// IP address, currently must be reachable publicly.
    /// TODO: Replace with "Connection".
    ip: String,
    /// Currently hard coded to networking, for full modularity this
    /// needs to be replaced by something part of the adapter, since
    /// not every adapter requires ip/port.
    port: usize,
    /// The secret key stored for encryption, the public key can be
    /// generated from it, so it doesn't have to be stored.
    secret: SecretKey,
    /// Where possible this is used as a user facing alternative to
    /// the routing key.
    hostname: String,
}

impl Config {
    /// Manually define the config. This should be used if all values
    /// are hard coded or obtained through a different way.
    pub fn new(bucket: usize, cache: usize, signaling: Vec<String>) -> Self {
        Self {
            bucket,
            signaling,
            cache,
        }
    }

    /// Loades the config from a toml file at the given path. It will
    /// fail and return Err(e) should the file not be readable or
    /// invalid. The file should be formatted and structured like the
    /// example "config.toml" but any formatting that is acceptable to
    /// the crates toml and serde should be acceptable.
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let load = match fs::read_to_string(path) {
            Ok(c) => {
                log::trace!("Loading config from '{}'", path);
                c
            }
            Err(e) => {
                log::error!("Invalid path '{}', unable to load config: {}", path, e);
                // If the config is not there simply return an empty
                // string and let the system fail (again) in the next
                // step.
                String::new()
            }
        };
        let config: Result<LoadConfig, toml::de::Error> = toml::from_str(&load);
        match config {
            Ok(c) => {
                log::info!("Successfully loaded config from file!");
                return Ok(Self {
                    bucket: c.network.bucket,
                    signaling: c.network.signaling,
                    cache: c.network.cache,
                });
            }
            Err(e) => {
                log::error!("Config is not valid: {}", e);
                return Err(Error::Config);
            }
        }
    }
}
