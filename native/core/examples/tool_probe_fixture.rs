fn main() -> std::io::Result<()> {
    if std::env::args().nth(1).as_deref() != Some("--version") {
        std::process::exit(2);
    }
    let marker = std::env::current_exe()?.with_extension("started");
    std::fs::write(marker, std::process::id().to_string())?;
    std::thread::sleep(std::time::Duration::from_secs(30));
    println!("v22.22.2");
    Ok(())
}
