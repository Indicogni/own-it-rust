//! Build script for label_core. Cargo compiles and runs this program before
//! it compiles the crate. Lines printed with the `cargo::` prefix are
//! instructions to Cargo.

fn main() {
    // Run this script again only when it changes, not after every edit in src/.
    println!("cargo::rerun-if-changed=build.rs");

    // Step 7: Cargo tells build scripts the optimisation level in the
    // environment variable OPT_LEVEL ("0" to "3", "s" or "z"). The crate
    // itself cannot see that variable. Pass its value on as
    // LABEL_CORE_OPT_LEVEL with a `cargo::rustc-env=NAME=VALUE` line.
    //
    // A build script runs on the machine that builds the crate, so this is an
    // ordinary run-time read. Cargo always sets OPT_LEVEL for build scripts.
    let level = std::env::var("OPT_LEVEL").expect("Cargo sets OPT_LEVEL for build scripts");
    println!("cargo::rustc-env=LABEL_CORE_OPT_LEVEL={level}");
}
