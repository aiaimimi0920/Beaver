fn main() {
    match beaver_core::code_structure::cli::run(std::env::args_os().skip(1).collect()) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("Code structure: {error}");
            std::process::exit(2);
        }
    }
}
