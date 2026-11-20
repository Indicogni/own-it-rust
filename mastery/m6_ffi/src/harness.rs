//! Test harness: runs other programs for the tests. You do not need to edit
//! this file, but lesson 07 explains what it does, and it is worth reading
//! once you have finished that lesson.
//!
//! - [`run`] runs a command with a time limit and collects its output.
//! - [`cargo`] prepares a nested Cargo run (lesson 02's lab, the static
//!   library of lesson 07).
//! - [`staticlib`] builds this package as a C static library.
//! - [`run_c_test`] compiles a C test program against that library and runs it.
//! - [`in_child`] runs one test again in a child process, so that a test may
//!   provoke a process abort without taking the whole test run down.
//!
//! Miri cannot start processes. The tests that need one are skipped under
//! Miri, and [`in_child`] runs the test body in place there.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// How long a compiler or Cargo run may take.
pub const BUILD_LIMIT: Duration = Duration::from_secs(600);
/// How long a compiled test program may run.
pub const RUN_LIMIT: Duration = Duration::from_secs(60);

/// A finished child process.
pub struct Finished {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Finished {
    /// Both output streams, labelled, for a failure message.
    pub fn output(&self) -> String {
        format!(
            "--- stdout ---\n{}\n--- stderr ---\n{}",
            self.stdout.trim_end(),
            self.stderr.trim_end()
        )
    }
}

/// Runs a command to its end, or stops it after `limit`.
#[track_caller]
pub fn run(command: &mut Command, what: &str, limit: Duration) -> Finished {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("cannot start {what}: {error}"));
    // Read both pipes on helper threads, so that a full pipe never blocks the child.
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let read_out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let read_err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().expect("waiting for a child process") {
            Some(status) => break status,
            None if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "{what} was still running after {} s and was stopped. \
                     Look for a loop that never ends.",
                    limit.as_secs()
                );
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    Finished {
        status,
        stdout: text(read_out.join().unwrap_or_default()),
        stderr: text(read_err.join().unwrap_or_default()),
    }
}

/// The directory of this package (where its Cargo.toml is).
pub fn package_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A directory inside Cargo's target directory for nested builds and compiled
/// C programs. build.rs computes it; nothing is written into the source tree.
pub fn scratch_dir(name: &str) -> PathBuf {
    Path::new(env!("M6_SCRATCH_DIR")).join(name)
}

/// A nested Cargo command: the same `cargo` that runs this test, with the
/// variables removed that describe *this* package and would confuse a build
/// of another one. The caller adds the subcommand and `--target-dir`.
pub fn cargo() -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("CARGO_MANIFEST_PATH")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("OUT_DIR");
    command
}

/// This package, built as a static library for C programs.
pub struct StaticLib {
    /// `libm6_ffi.a`, or `m6_ffi.lib` with MSVC.
    pub path: PathBuf,
    /// What the final link also needs, as printed by
    /// `rustc --print native-static-libs`. A Rust static library contains the
    /// Rust standard library but not the system libraries that std calls.
    pub native_libs: Vec<String>,
}

/// Builds the static library once per test run.
///
/// The command is the one you would type yourself:
///
/// ```text
/// cargo rustc --lib --crate-type staticlib -- --print native-static-libs
/// ```
///
/// with a target directory of its own, so that it never waits for the lock
/// of the `cargo test` that is running this test.
pub fn staticlib() -> Result<&'static StaticLib, &'static str> {
    static BUILT: OnceLock<Result<StaticLib, String>> = OnceLock::new();
    BUILT
        .get_or_init(build_staticlib)
        .as_ref()
        .map_err(String::as_str)
}

fn build_staticlib() -> Result<StaticLib, String> {
    let target_dir = scratch_dir("staticlib");
    let release = !cfg!(debug_assertions);
    let mut command = cargo();
    command
        .current_dir(package_dir())
        .args(["rustc", "--lib", "--crate-type", "staticlib"])
        .args(["--locked", "--offline", "--color", "never"])
        .arg("--target-dir")
        .arg(&target_dir);
    if release {
        command.arg("--release");
    }
    command.args(["--", "--print", "native-static-libs"]);
    let built = run(&mut command, "the static library build", BUILD_LIMIT);
    if !built.status.success() {
        return Err(format!(
            "building the static library failed:\n{}",
            built.stderr.trim_end()
        ));
    }
    // rustc prints: "note: native-static-libs: -lSystem -lc -lm"
    let marker = "native-static-libs:";
    let native_libs = built
        .stderr
        .lines()
        .filter_map(|line| line.split_once(marker))
        .map(|(_, libs)| libs.split_whitespace().map(String::from).collect())
        .next_back()
        .ok_or_else(|| {
            format!(
                "rustc did not print the `{marker}` line:\n{}",
                built.stderr.trim_end()
            )
        })?;
    let file = if cfg!(target_env = "msvc") {
        "m6_ffi.lib"
    } else {
        "libm6_ffi.a"
    };
    let profile = if release { "release" } else { "debug" };
    let path = target_dir.join(profile).join(file);
    if !path.is_file() {
        return Err(format!(
            "the static library was not found at {}",
            path.display()
        ));
    }
    Ok(StaticLib { path, native_libs })
}

/// Compiles `ctests/<name>.c` with the system's C compiler, links it with the
/// static library, runs it, and returns what happened. A compile or link
/// error is returned as `Err` with the compiler's messages.
///
/// The `cc` crate finds the compiler and the flags for this target, exactly
/// as it does in build.rs. Outside a build script it needs to be told the
/// target and the host, which build.rs recorded for us.
pub fn run_c_test(name: &str, library: &StaticLib) -> Result<Finished, String> {
    let source = package_dir().join("ctests").join(format!("{name}.c"));
    let out_dir = scratch_dir("ctests").join(name);
    std::fs::create_dir_all(&out_dir)
        .map_err(|error| format!("cannot create {}: {error}", out_dir.display()))?;
    let exe = out_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));

    let mut build = cc::Build::new();
    build
        .target(env!("M6_TARGET"))
        .host(env!("M6_HOST"))
        .opt_level(0)
        .debug(false)
        .out_dir(&out_dir)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .warnings(true)
        .include(package_dir().join("include"))
        .include(package_dir().join("vendor").join("kvtab"));
    let compiler = build
        .try_get_compiler()
        .map_err(|error| format!("no C compiler found: {error}"))?;
    let mut command = compiler.to_command();
    command.current_dir(&out_dir);
    if compiler.is_like_msvc() {
        // cl <flags> test.c /Fe<exe> /link <library> <system libraries>
        // The /we flags turn these warnings into errors, as the flags for
        // gcc and clang do below: C4013 (call of an undeclared function),
        // and C4024, C4047, C4113, C4133 (a function or pointer whose type
        // differs from the one that the test program expects).
        let mut exe_flag = OsString::from("/Fe");
        exe_flag.push(&exe);
        command
            .args([
                "/nologo", "/we4013", "/we4024", "/we4047", "/we4113", "/we4133",
            ])
            .arg(&source)
            .arg(exe_flag)
            .arg("/link")
            .arg(&library.path)
            .args(&library.native_libs);
    } else {
        // cc <flags> test.c -o <exe> <library> <system libraries>
        command
            .arg("-Werror=implicit-function-declaration")
            .arg("-Werror=incompatible-pointer-types")
            .arg(&source)
            .arg("-o")
            .arg(&exe)
            .arg(&library.path)
            .args(&library.native_libs);
    }
    let compiled = run(&mut command, "the C compiler", BUILD_LIMIT);
    if !compiled.status.success() {
        return Err(format!(
            "compiling or linking ctests/{name}.c failed:\n{}",
            compiled.output()
        ));
    }
    Ok(run(
        &mut Command::new(&exe),
        &format!("the C test program {name}"),
        RUN_LIMIT,
    ))
}

/// Name of the variable that tells a test that it is the child.
const CHILD_VAR: &str = "M6_FFI_CHILD_TEST";

/// Runs the calling test again in a child process and reports the result.
///
/// Use it as the first line of a test whose body may abort the process when
/// the code under test is wrong (a panic that reaches an `extern "C"` frame
/// aborts; it cannot be caught):
///
/// ```ignore
/// if in_child("l05_callbacks::tests::a_panic_is_carried_over_the_c_frames") {
///     return;
/// }
/// // ... the risky part ...
/// ```
///
/// In the parent, this function starts the test executable again for exactly
/// this test, waits, and panics with a readable message if the child failed
/// or was killed; then it returns `true` and the parent's test is done. In
/// the child it returns `false`, and the body runs. Under Miri there are no
/// child processes, so it returns `false` and the body runs in place.
#[track_caller]
pub fn in_child(test_path: &str) -> bool {
    if cfg!(miri) || std::env::var_os(CHILD_VAR).is_some_and(|name| name == test_path) {
        return false;
    }
    let exe = std::env::current_exe().expect("the path of the test executable");
    let mut command = Command::new(exe);
    command
        .args([test_path, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_VAR, test_path);
    let child = run(&mut command, test_path, RUN_LIMIT);
    if !child.status.success() {
        panic!(
            "the test failed in its child process ({}).\n\
             If the output below ends with \"panic in a function that cannot unwind\" \
             and no test result, the process was aborted: a panic reached an \
             `extern \"C\"` function and tried to leave it.\n{}",
            child.status,
            child.output()
        );
    }
    assert!(
        child.stdout.contains("1 passed"),
        "the child process ran no test named `{test_path}`:\n{}",
        child.output()
    );
    true
}
