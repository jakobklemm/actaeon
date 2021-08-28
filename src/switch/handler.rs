//! Handler
//!
//! Accepts incoming tcp messages.

//use super::message::Message;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

pub struct SwitchInterface<T> {
    channel: mpsc::Sender<T>,
}

pub struct Handler<T> {
    channel: mpsc::Receiver<T>,
}

impl<T> Handler<T> {
    pub fn start(addr: String) -> std::io::Result<SwitchInterface<T>> {
        let listener = TcpListener::bind(addr)?;

        listener
            .set_nonblocking(true)
            .expect("Listener setup failed!");

        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        tracing::info!("New stream icoming: {:?}", stream.peer_addr().unwrap());
                        Handler::handle(stream);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Unable to handle new connection: {:?}", e)
                    }
                }
            }
        });

        let _a = handle.join();

        return SwitchInterface { channel: tx };
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
