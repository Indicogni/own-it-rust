//! Build script of `meter`: turns the environment variable `METER_FAST` into the
//! custom cfg `has_fast_path`.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=METER_FAST");
    // Declare the cfg even when it is off, so `cfg(has_fast_path)` is never "unexpected".
    println!("cargo::rustc-check-cfg=cfg(has_fast_path)");
    if std::env::var_os("METER_FAST").is_some() {
        println!("cargo::rustc-cfg=has_fast_path");
    }
}
