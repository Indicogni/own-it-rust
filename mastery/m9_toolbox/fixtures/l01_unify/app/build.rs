//! Records which features `shared` had when it was linked into this build script.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!(
        "cargo::rustc-env=SHARED_IN_BUILD_SCRIPT={}",
        shared::features()
    );
}
