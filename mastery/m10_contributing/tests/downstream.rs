//! Lessons 04 and 08: build the crates in `downstream/` against the fixture.
//!
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml --test downstream
//!
//! Each test starts a second `cargo`, which compiles the fixture again for the
//! downstream workspace. The first run takes a few seconds.

use std::path::Path;
use std::process::Command;

/// Runs `cargo test --package PACKAGE` in the downstream workspace and returns
/// whether it succeeded, with everything it printed.
fn cargo_test(package: &str) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| env!("CARGO").into());
    let output = Command::new(cargo)
        .args(["test", "--quiet", "--offline", "--package", package])
        .arg("--manifest-path")
        .arg(root.join("downstream/Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("downstream"),
        )
        .output()
        .expect("cargo can be started");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

/// Lessons 04, 05, 08 and the capstone: whatever you change in the fixture, the user
/// of 0.4.0 must still compile and its tests must still pass.
#[test]
fn pinned_user_of_0_4_0_still_builds_and_passes() {
    let (ok, output) = cargo_test("pinned");
    assert!(
        ok,
        "downstream/pinned no longer builds or passes against the fixture. A change \
         you made is a breaking change for users of 0.4.0. Cargo printed:\n{output}"
    );
}

/// Lesson 04: the user who asked for the dry run can use it.
#[test]
fn l04_adopter_of_dry_run_builds_and_passes() {
    let (ok, output) = cargo_test("adopter");
    assert!(
        ok,
        "downstream/adopter does not build or pass yet. It uses the API that the \
         maintainers accepted in fixture/issues/0021-dry-run.md. Cargo printed:\n{output}"
    );
}
