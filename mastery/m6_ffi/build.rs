//! Build script of the M6 package. Cargo compiles and runs this program
//! before it compiles `src/lib.rs`. Lesson 02 explains every line.
//!
//! It has three jobs:
//!
//! 1. compile the vendored C library `vendor/kvtab` into a static library;
//! 2. tell Cargo how to link it, when to run this script again, and what to
//!    pass on to packages that depend on this one;
//! 3. hand a few facts about the build (target, host, a scratch directory)
//!    to the tests that drive the C compiler themselves.

use std::env;
use std::path::PathBuf;

fn main() {
    // Cargo runs a build script with the package directory as the working
    // directory, so relative paths work. Absolute paths are used where the
    // value leaves this process.
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let kvtab = manifest_dir.join("vendor").join("kvtab");

    // When must this script run again? As soon as a script prints one
    // `rerun-if-*` line, Cargo stops watching the package directory for it
    // and watches only what the script names. The `cc` crate prints
    // `rerun-if-env-changed` lines (for CC, CFLAGS and so on) but none for
    // source files, so every C input must be named here.
    for input in [
        "build.rs",
        "vendor/kvtab/kvtab.c",
        "vendor/kvtab/kvtab.h",
        "csrc/layout_probe.c",
    ] {
        println!("cargo::rerun-if-changed={input}");
    }

    // Compile kvtab.c and archive it as libkvtab.a (kvtab.lib with MSVC) in
    // OUT_DIR. `compile` also prints the two lines that make rustc link it:
    //     cargo:rustc-link-search=native=<OUT_DIR>
    //     cargo:rustc-link-lib=static=kvtab
    cc::Build::new()
        .file(kvtab.join("kvtab.c"))
        .include(&kvtab)
        .warnings(true)
        .extra_warnings(true)
        .compile("kvtab");

    // The layout probe of lesson 03: a second, separate static library.
    cc::Build::new()
        .file("csrc/layout_probe.c")
        .include(&kvtab)
        .warnings(true)
        .compile("m6_layout_probe");

    // Because Cargo.toml says `links = "kvtab"`, this line becomes the
    // environment variable DEP_KVTAB_INCLUDE in the build script of every
    // package that depends directly on this one. A dependent that compiles
    // its own C code against kvtab.h finds the header this way.
    println!("cargo::metadata=include={}", kvtab.display());

    // Facts for the tests that compile C programs (lesson 07, capstone).
    // TARGET and HOST exist only while a build script runs; `rustc-env`
    // turns them into `env!("M6_TARGET")` inside the crate.
    for name in ["TARGET", "HOST"] {
        let value = env::var(name).expect("set by Cargo for build scripts");
        println!("cargo::rustc-env=M6_{name}={value}");
    }
    // The value of the `links` key, for a prediction in lesson 02.
    let links = env::var("CARGO_MANIFEST_LINKS").expect("set because Cargo.toml has `links`");
    println!("cargo::rustc-env=M6_LINKS={links}");

    // A scratch directory for nested Cargo runs and compiled C test programs.
    // OUT_DIR is <target>/<profile>/build/m6_ffi-<hash>/out, so four levels
    // up is the profile's parent directory.
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("set by Cargo"));
    let scratch = out_dir
        .ancestors()
        .nth(4)
        .unwrap_or(&out_dir)
        .join("m6-nested");
    println!("cargo::rustc-env=M6_SCRATCH_DIR={}", scratch.display());
}
