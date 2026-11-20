//! Tells the linker to use `link.ld`.
//!
//! A build script is used instead of `rustflags` in `.cargo/config.toml`
//! because it applies no matter which directory Cargo is started from.

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR");
    println!("cargo::rustc-link-arg-bins=-T{dir}/link.ld");
    println!("cargo::rerun-if-changed=link.ld");
    println!("cargo::rerun-if-changed=build.rs");
}
