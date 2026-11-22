//! This is minimaze4, a web app containing lots of applets. It uses Rust, Leptos, and WebAssembly.
//!
//! # Links
//!
//! Visit minimaze4 via [Google sites](https://sites.google.com/view/minimaze4/h) or via [Cloudflare pages](minimaze.pages.dev).

use std::fs;
use std::path::PathBuf;

fn main() {
    let version_file = PathBuf::from("version.txt");
    let current_version = fs::read_to_string(&version_file)
        .unwrap_or_else(|_| "0".to_owned())
        .trim()
        .parse::<usize>()
        .unwrap_or(0);
    let new_version = current_version + 1;
    _ = fs::write(version_file, new_version.to_string());
    println!("cargo:rustc-env=BUILD_VERSION={new_version}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
}
