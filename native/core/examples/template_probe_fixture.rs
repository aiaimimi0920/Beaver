fn main() {
    if std::env::args().nth(1).as_deref() != Some("--version") {
        std::process::exit(2);
    }
    println!("4.6.stable.official.fixture");
}
