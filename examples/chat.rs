use actaeon::{
    config::Config,
    node::{Center, ToAddress},
    topic::Topic,
    Interface,
};
use sodiumoxide::crypto::box_;
use std::io;
use std::sync::mpsc;

fn main() -> io::Result<()> {
    let config = Config::new(20, 1, 100, "example.com".to_string(), 4242);
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 1234);
    let interface = Interface::new(config, center).unwrap();
    let (s, r) = mpsc::channel();
    println!("Actaeon Chat Example Application!");
    println!(" - - - - - - - - - - - - - - ");
    println!("Enter chat room name: ");
    let mut buffer = String::new();
    let stdin = io::stdin();
    stdin.read_line(&mut buffer)?;
    println!(" - - - - - - - - - - - - - - ");
    let topic = buffer.to_address();
    println!("Chat room ID: {:?}", topic);
    let topic = interface.subscribe(&topic);
    receiver(topic, r);
    println!("");
    loop {
        println!("Send: ");
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer)?;
        let _ = s.send(buffer.as_bytes().to_vec());
    }
}

fn receiver(mut topic: Topic, recv: mpsc::Receiver<Vec<u8>>) {
    std::thread::spawn(move || {
        if let Some(msg) = topic.try_recv() {
            println!("Received message: {:#?}", msg.message.body.as_bytes());
        }
        if let Ok(bytes) = recv.try_recv() {
            let _ = topic.broadcast(bytes);
        }
    });
}
