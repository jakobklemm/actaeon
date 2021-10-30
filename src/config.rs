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
use std::fs::File;
use std::io::BufRead;

/// Config values for the config of networking parameters if the
/// config is loaded from the default toml file. The values will
/// usually come from the config file. Others might get populated by
/// the code in order to prevent the user from chaning parameters.
#[derive(Deserialize)]
struct Network {
    /// serde deserialization value for the config file.
    bucket: usize,
    /// Number of message to be sent
    replication: usize,
    /// serde deserialization value for the config file.
    signaling: String,
    port: usize,
    /// serde deserialization value for the config file.
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
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Config {
    /// Defines the size value of the kademlia based routing system,
    /// comparable to the variable "k". It defines the size of each
    /// bucket, the maximum number of buckets is currently not
    /// configurable and defined through the hashing / address system.
    pub bucket: usize,
    /// Concurrent messages count
    pub replication: usize,
    /// Array of signaling servers, used to connect to the system
    /// initially and possibly provide forwarding.
    pub signaling: String,
    /// Port of the signaling server.
    pub port: usize,
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
    port: usize,
    /// Whenever possible the hostname is used instead of the routing
    /// key.
    hostname: String,
}

pub struct Signaling {
    server: String,
    port: usize,
}

/// The local node is configured separately, since some of the values
/// will have to be set manually or optained through an external
/// method. Like with the SystemConfig all fields are public and will
/// be parsed into internall formats down the line.
pub struct CenterConfig {
    /// IP address, currently must be reachable publicly.
    pub ip: String,
    /// Currently hard coded to networking, for full modularity this
    /// needs to be replaced by something part of the adapter, since
    /// not every adapter requires ip/port.
    pub port: usize,
    /// The secret key stored for encryption, the public key can be
    /// generated from it, so it doesn't have to be stored. Instead of
    /// storing the object only the bytes will be processed here.
    pub secret: Option<[u8; 32]>,
    /// Where possible this is used as a user facing alternative to
    /// the routing key.
    pub hostname: String,
}

impl Signaling {
    pub fn new(server: String, port: usize) -> Self {
        Self { server, port }
    }

    pub fn to_string(&self) -> String {
        let elements = [self.server.clone(), self.port.to_string()];
        elements.join(":")
    }
}

impl Config {
    /// Manually define the config. This should be used if all values
    /// are hard coded or obtained through a different way.
    pub fn new(
        bucket: usize,
        replication: usize,
        cache: usize,
        signaling: String,
        port: usize,
    ) -> Self {
        Self {
            bucket,
            replication,
            signaling,
            port,
            cache,
        }
    }

    /// Shorthand for reading the config and parsing the toml. Will
    /// fail if the fail is not readable or invalid.
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let content = fs::read_to_string(path)?;
        Self::from_string(content)
    }

    /// Should the config already be available as a toml formatted
    /// string it can be parsed directly. In the future this should be
    /// made format independant by removing the hard coded dependancy
    /// on serde / toml.
    pub fn from_string(content: String) -> Result<Self, Error> {
        let config: Result<LoadConfig, toml::de::Error> = toml::from_str(&content);
        match config {
            Ok(c) => {
                log::info!("Successfully loaded system config from file!");
                return Ok(Self {
                    bucket: c.network.bucket,
                    replication: c.network.replication,
                    signaling: c.network.signaling,
                    port: c.network.port,
                    cache: c.network.cache,
                });
            }
            Err(e) => {
                log::error!("System config is not valid: {}", e);
                return Err(Error::Config(String::from("unable to parse toml")));
            }
        }
    }
}

impl CenterConfig {
    /// Should the config be optained through a custom method or all
    /// be hard hard coded (?) a new config can be created directly.
    /// It is important to keep in mind that this part of the
    /// configuration is meant to be in human readable (non-custom)
    /// formatt, therefor the secret key is stored as an array of
    /// bytes. It is not recommended to randomly generate these bytes,
    /// instead encryption specific tools should be used.
    pub fn new(ip: String, port: usize, secret: [u8; 32], hostname: String) -> Self {
        Self {
            ip,
            port,
            secret: Some(secret),
            hostname,
        }
    }

    /// Opens a config toml file at the provided path and parse it
    /// into the object. This will not consider the secret key, since
    /// it needs to be read separately.
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let content = fs::read_to_string(path)?;
        Self::from_string(content)
    }

    /// If the config is already available as a tomll string it can be
    /// parsed directly. This will also ignore the secret key, since
    /// it can't easily be stored in the same file.
    pub fn from_string(config: String) -> Result<Self, Error> {
        let config: Result<LoadCenter, toml::de::Error> = toml::from_str(&config);
        match config {
            Ok(c) => {
                log::info!("Successfully loaded center config from file!");
                return Ok(Self {
                    ip: c.ip,
                    port: c.port,
                    secret: None,
                    hostname: c.hostname,
                });
            }
            Err(e) => {
                log::error!("Config is not valid: {}", e);
                return Err(Error::Config(String::from(
                    "unable to parse config from toml",
                )));
            }
        }
    }

    /// The secret key can't be formatted as UTF-8 and if stored as a
    /// file it needs to be encoded / decoded using special methods.
    pub fn load_key(path: &str) -> Result<[u8; 32], Error> {
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(file);

        let line = reader.split(b'\n').next();
        match line {
            Some(rkey) => {
                let key = rkey?;
                if key.len() != 32 {
                    return Err(Error::Config(String::from("invalid byte length in key")));
                }
                let mut bytes: [u8; 32] = [0; 32];
                for (i, j) in key.iter().enumerate() {
                    bytes[i] = *j;
                }
                return Ok(bytes);
            }
            None => {
                return Err(Error::Config(String::from("key file is empty")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_parse() {
        let c = "# Example Actaeon config.
[network]
        bucket = 32
        signaling = '127.0.0.1'
        replication = 3
        port = 4242
        cache = 32

";
        let config = Config::from_string(c.to_string()).unwrap();
        let created = Config::new(32, 3, 32, "127.0.0.1".to_owned(), 4242);
        assert_eq!(config, created);
    }

    #[test]
    fn test_center_parse() {
        let c = "# Example Actaeon config.
        ip = '127.0.0.1'
        port = 42
        hostname = 'actaeon'
";

        let config = CenterConfig::from_string(c.to_string()).unwrap();
        let created = CenterConfig::new("127.0.0.1".to_owned(), 42, [0; 32], "actaeon".to_owned());
        assert_eq!(config.ip, created.ip);
    }
}
