use std::process::Command;

pub fn main() {
    Command::new("cargo")
        .args(&["build", "--release", "--target=wasm32-unknown-unknown"])
        .current_dir("wormrtl")
        .status()
        .unwrap();
}
