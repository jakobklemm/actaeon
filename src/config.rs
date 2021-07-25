//! # Config
//!
//! System wide configuration data like network details and system parameters.
//! The config struct can be populated either through hard coded values, loaded from a file or set through command line arguments.

// Public configuration struct, fields can be directly accessed.
pub struct Config<'a> {
    pub ip: &'a str,
    pub port: u16,
    pub size: u16,
}

impl<'a> Config<'a> {
    pub fn new(ip: &'a str, port: u16, size: u16) -> Self {
        Self { ip, port, size }
    }
}
