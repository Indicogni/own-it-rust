//! Build script of `fletch-sys` (lesson 02 lab).

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let include = manifest_dir.join("vendor").join("fletch").join("include");

    // cc prints rerun-if-env-changed lines, and any rerun-if line switches
    // off Cargo's default rule ("run again when any file of the package
    // changes"). So the C inputs must be named one by one.
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=vendor/fletch/src/fletch.c");
    println!("cargo::rerun-if-changed=vendor/fletch/include/fletch.h");

    let mut build = cc::Build::new();
    build
        .file("vendor/fletch/src/fletch.c")
        .include(&include)
        .warnings(true);
    // Cargo sets CARGO_FEATURE_<NAME> for every enabled feature of the
    // package whose build script is running.
    if env::var_os("CARGO_FEATURE_WIDE").is_some() {
        build.define("FLETCH_WIDE", None);
    }
    // Compiles, archives, and prints
    //     cargo:rustc-link-search=native=<OUT_DIR>
    //     cargo:rustc-link-lib=static=fletch
    build.compile("fletch");

    // Because Cargo.toml says `links = "fletch"`, the build scripts of
    // direct dependents see this value as DEP_FLETCH_INCLUDE.
    println!("cargo::metadata=include={}", include.display());
}
