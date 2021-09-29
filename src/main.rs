use actaeon::node::Address;
use actaeon::node::Center;
use actaeon::tcp::Handler;
use sodiumoxide::crypto::box_;
use uuid::Uuid;

fn main() {
    let d = generate_test_data();
    std::fs::write("message.txt", d).unwrap();
    let (_, s) = box_::gen_keypair();
    let center = Center::new(s, String::from("127.0.0.1"), 4242);
    let mut h = Handler::new(center).unwrap();
    loop {
        match h.read() {
            Some(bts) => {
                println!("{:?}", bts);
            }
            None => {}
        }
    }
}

fn generate_test_data() -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();

    data.append(&mut [0, 0, 0, 1].to_vec());

    let source = Address::generate("abc")
        .unwrap()
        .as_bytes()
        .to_owned()
        .to_vec();
    data.append(&mut source.clone());
    let target = Address::generate("def")
        .unwrap()
        .as_bytes()
        .to_owned()
        .to_vec();
    data.append(&mut target.clone());
    let uuid = Uuid::parse_str(&mut "27d626f0-1515-47d4-a366-0b75ce6950bf").unwrap();
    data.append(&mut uuid.clone().as_bytes().to_vec());

    data.append(&mut [0; 24].to_vec());

    data.append(&mut "test".to_string().into_bytes());
    return data;
}
