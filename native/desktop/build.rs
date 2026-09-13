fn main() {
    println!("cargo:rerun-if-changed=../../resources/branding/beaver.ico");
    tauri_build::build()
}
