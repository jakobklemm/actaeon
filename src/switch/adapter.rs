//! Handler
//!
//! Trait and structure specifications for network adapters. The
//! system comes by default with a TCP Adapter, but it is possible to
//! implement more methods.

//use super::message::Message;

use crate::error::Error;
use crate::transaction::Wire;

/// By default the system uses a TCP listener for incoming
/// connections, but it is easy to swap out the adapter for any other
/// networking protocol. The API is closely modeled after the rust
/// TcpListener API, for example there has to be a field or option to
/// set the adapter into blocking or non blocking mode. From there the
/// accept/0 function will be called in a loop inside the listening
/// thread.
pub trait Adapter {
    /// If the adapter is passive and doesn't need to be started this
    /// method won't do anything, but for TCP-like methods this should
    /// start the actuall listeners. Starting the listener does not
    /// require to start a thread, since the system will take care of
    /// starting the thread. This also means that any implementation
    /// of the adapter also has to implement Send & Sync.
    fn start(&mut self) -> Result<(), Error>;
    /// Core function that will be used in the listening thread to get
    /// incoming messages. It will also have to parse it into a Wire
    /// object, which will then get evaluated. This is comparable to
    /// the TcpListener accept/0, not the listen/0 function, which
    /// returns an iterator over new connectins.
    fn accept(&self) -> Result<Wire, Error>;
    /// Sets the Adapter into Blocking or Nonblocking mode. Currently
    /// only those two options are supported, it might be helpful to
    /// modularize it further or allow more modes. Changing the mode
    /// will change the behavior of the accept/0 function called in
    /// the listening loop.
    fn mode(&mut self, mode: Mode) -> Result<(), Error>;
    /// Shut down the listener / adapter. This might fail if the
    /// adapter is currently handling connections, in which case it
    /// will most likely return Error::Busy.
    fn terminate(&mut self) -> Result<(), Error>;
}

/// The two different modes an adapter can be in. Currently the two
/// modes represent the two modes a TCP listener can be in, in the
/// future more modes or a trait object might get added.
pub enum Mode {
    Blocking,
    Unblocking,
}

// impl<T> Handler<T> {
//     pub fn start(addr: String) -> std::io::Result<SwitchInterface<T>> {
//         let listener = TcpListener::bind(addr)?;

//         listener
//             .set_nonblocking(true)
//             .expect("Listener setup failed!");

//         let (tx, rx) = mpsc::channel();

//         let handle = thread::spawn(move || {
//             for stream in listener.incoming() {
//                 match stream {
//                     Ok(stream) => {
//                         tracing::info!("New stream icoming: {:?}", stream.peer_addr().unwrap());
//                         Handler::handle(stream);
//                     }
//                     Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
//                         continue;
//                     }
//                     Err(e) => {
//                         tracing::error!("Unable to handle new connection: {:?}", e)
//                     }
//                 }
//             }
//         });

//         let _a = handle.join();

//         return SwitchInterface { channel: tx };
//     }

//     fn handle(mut s: TcpStream) {
//         let mut data = Vec::new();
//         let size = s.read_to_end(&mut data).unwrap();
//         tracing::info!("Received {} bytes of data.", size);
//         for i in data.into_iter() {
//             println!("{:?}", i);
//         }
//     }
// }
