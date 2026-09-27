use std::{env, fs};

fn main() {
    assert_eq!(env::args().skip(1).collect::<Vec<_>>(), ["--version"]);
    let executable = env::current_exe().unwrap();
    let version = match executable.file_stem().unwrap().to_str().unwrap() {
        "codex" => "codex-cli 0.1.0",
        "godot" => "4.4.1.stable.migration-fixture",
        "blender" => "Blender 4.2.0",
        "node" => "v22.1.0",
        name => panic!("Unexpected migration fixture tool: {name}"),
    };
    fs::write(executable.with_extension("probed"), "--version\n").unwrap();
    println!("{version}");
}
