//! # Database
//!
//! Responsible for storing topics and possibly a copy of the routing
//! table on the local file system. All data will be stored in binary,
//! in order to minimize size and make interactions with Wire data as
//! easy as possible.

pub struct Database {
    pub path: String,
}

impl Database {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}
