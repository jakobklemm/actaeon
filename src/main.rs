fn main() {
    println!(
        "{:?}",
        actaeon::config::CenterConfig::load_key("secret.key").unwrap()
    );
}
