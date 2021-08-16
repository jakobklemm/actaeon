//! Handler
//!
//! Accepts incoming tcp messages.

//use super::message::Message;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;

pub struct Handler {}

impl Handler {
    pub fn start(addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr)?;

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        tracing::info!("New stream icoming: {:?}", stream.peer_addr().unwrap());
                        Handler::handle(stream);
                    }
                    Err(e) => {
                        tracing::error!("Unable to handle new connection: {:?}", e)
                    }
                }
            }
        });

        let _a = handle.join();

        Ok(())
    }

    fn handle(mut s: TcpStream) {
        let mut data = Vec::new();
        let size = s.read_to_end(&mut data).unwrap();
        tracing::info!("Received {} bytes of data.", size);
        for i in data.into_iter() {
            println!("{:?}", i);
        }
    }
}
