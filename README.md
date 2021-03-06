# Actaeon

Distributed PubSub and messaging protocol for decentralized real time
applications.

This library offers users a simple to use, easy to understand API for a
mostly automatically managed decentralized messaging system based on the
Kademlia paper. Fast and reliable message exchange is possible through
both direct and indirect TCP messaging.

A very simple example of how to use the library to broadcast messages on
a specific topic:

``` rust
use actaeon::{
    config::Config,
    node::{Center, ToAddress},
    Interface,
};
use sodiumoxide::crypto::box_;

fn main() {
    let config = Config::new(20, 1, 100, "example.com".to_string(), 4242);
    let (_, secret) = box_::gen_keypair();
    let center = Center::new(secret, String::from("127.0.0.1"), 1234);

    let interface = Interface::new(config, center).unwrap();

    let mut topic = interface.subscribe(&"example".to_string().to_address());

    let _ = topic.broadcast("hello world".as_bytes().to_vec());
}
```

There are still some unresolved issues:

- Error handling for signaling & multiple signaling servers.
- Add interior mutability for topics so the user doesn't have to
  declare them as mutable.
- Handle crashed threads.
