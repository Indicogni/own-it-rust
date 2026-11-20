//! Build script of `meter`: turns the environment variable `METER_FAST` into the
//! custom cfg `has_fast_path`.

fn main() {
    if std::env::var_os("METER_FAST").is_some() {
        println!("cargo:rustc-cfg=has_fast_path");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
