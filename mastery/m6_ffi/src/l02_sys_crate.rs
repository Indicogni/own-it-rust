//! LESSON 02 — Building C from Cargo: build.rs, the cc crate, `links`, and a `-sys` crate
//! Read first: CONCEPTS.md, "Lesson 02". Start after: lesson 01 and Chapter 8.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then read `build.rs` of this package from top to
//! bottom; it is the worked example for this lesson. Consider: the `cc` crate prints
//! many `cargo:rerun-if-env-changed` lines. What does Cargo stop doing as soon as
//! a build script prints its first `rerun-if` line?
//! IMPLEMENT the three TODOs in `sys_lab/fletch-sys/` (see below). There is no Rust
//! code to write in this file.
//! EXPLAIN afterward: Follow one `cargo build` of the lab from start to finish. Say
//! in which order Cargo runs the two build scripts and compiles the two crates, which
//! lines of your build script reach rustc and which reach the build script of
//! `checker`, and what would happen if a second package in the graph also said
//! `links = "fletch"`.
//! MIRI: this lesson starts Cargo and a C compiler, which Miri cannot do. Its tests
//! are skipped under Miri.
//!
//! # The lab
//!
//! `sys_lab/` is a small workspace of its own, next to this package:
//!
//! ```text
//! sys_lab/
//!   Cargo.toml                        the workspace
//!   fletch-sys/                       YOU FINISH THIS CRATE
//!     Cargo.toml                      TODO 1: the `links` key
//!     build.rs                        TODO 2: the build script
//!     src/lib.rs                      TODO 3: the declarations
//!     vendor/fletch/include/fletch.h  the C library (read the header)
//!     vendor/fletch/src/fletch.c
//!   checker/                          supplied; depends on fletch-sys
//! ```
//!
//! `fletch` is a tiny C library with two checksum functions. The second one,
//! `fletch32`, is compiled only when the C macro `FLETCH_WIDE` is defined.
//! Your `-sys` crate has a Cargo feature `wide` that must switch it on.
//!
//! The contract of `fletch-sys`:
//!
//! 1. Its manifest declares that it links the native library `fletch`.
//! 2. Its build script compiles `vendor/fletch/src/fletch.c` into a static
//!    library with the `cc` crate (already listed under `[build-dependencies]`)
//!    and lets `cc` print the link instructions.
//! 3. The build script defines `FLETCH_WIDE` for the C compiler when, and only
//!    when, the feature `wide` is enabled.
//! 4. The build script names its inputs: after a change to `fletch.c` or
//!    `fletch.h`, the next `cargo build` compiles the C code again.
//! 5. The build script publishes the absolute path of the include directory
//!    under the metadata key `include`, so that a dependent package finds
//!    `fletch.h`.
//! 6. `src/lib.rs` declares `fletch16`, `fletch_has_wide` and, behind the
//!    feature, `fletch32`, as `pub` items.
//!
//! You can run the lab by hand while you work:
//!
//! ```text
//! cd mastery/m6_ffi/sys_lab
//! cargo test -p checker
//! cargo test -p checker --features wide
//! cargo build -p fletch-sys -vv        # shows what the build script printed
//! ```
//!
//! The tests below run the same commands with a target directory inside this
//! package's target directory, so they leave nothing behind in `sys_lab/`.

#[cfg(all(test, not(miri)))]
mod tests {
    use crate::guess;
    use crate::harness::{self, BUILD_LIMIT, Finished};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    fn lab_dir() -> PathBuf {
        harness::package_dir().join("sys_lab")
    }

    /// Runs `cargo <args>` in the workspace at `lab`, with `target_dir` as
    /// its target directory.
    ///
    /// Each copy of the lab needs a target directory of its own. Cargo names
    /// build products after the package's path *relative to its workspace*,
    /// so two copies of one workspace that share a target directory would
    /// overwrite each other's files.
    fn lab_cargo(lab: &Path, target_dir: &Path, args: &[&str]) -> Finished {
        let mut command = harness::cargo();
        command
            .current_dir(lab)
            .args(args)
            .args(["--offline", "--locked", "--color", "never"]);
        if args[0] != "metadata" {
            command.arg("--target-dir").arg(target_dir);
        }
        harness::run(&mut command, "cargo (lesson 02 lab)", BUILD_LIMIT)
    }

    /// The target directory for builds of the lab itself.
    fn lab_target() -> PathBuf {
        harness::scratch_dir("l02-target")
    }

    /// Fails the test with Cargo's messages if the run was not successful.
    #[track_caller]
    fn expect_success(finished: &Finished, what: &str) {
        assert!(
            finished.status.success(),
            "{what} failed. Cargo said:\n{}",
            finished.output()
        );
    }

    /// The value of a string field in one line of Cargo's JSON output.
    fn json_string(line: &str, field: &str) -> Option<String> {
        let pattern = format!("\"{field}\":\"");
        let start = line.find(&pattern)? + pattern.len();
        let mut value = String::new();
        let mut chars = line[start..].chars();
        loop {
            match chars.next()? {
                '"' => return Some(value),
                '\\' => match chars.next()? {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'u' => {
                        let hex: String = chars.by_ref().take(4).collect();
                        value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    other => value.push(other),
                },
                other => value.push(other),
            }
        }
    }

    /// Everything that the build script of fletch-sys printed in its last run.
    /// Cargo keeps it in the file `output` next to the script's OUT_DIR.
    fn build_script_output() -> String {
        let built = lab_cargo(
            &lab_dir(),
            &lab_target(),
            &["build", "-p", "fletch-sys", "--message-format=json"],
        );
        assert!(
            built.status.success(),
            "building fletch-sys failed. Run `cargo build -p fletch-sys` in sys_lab/ \
             to see the full message.\n{}",
            built.stderr.trim_end()
        );
        let out_dir = built
            .stdout
            .lines()
            .filter(|line| line.contains("\"reason\":\"build-script-executed\""))
            .filter(|line| {
                json_string(line, "package_id").is_some_and(|id| id.contains("fletch-sys"))
            })
            .find_map(|line| json_string(line, "out_dir"))
            .expect("Cargo reports the build script run of fletch-sys");
        let output = Path::new(&out_dir).with_file_name("output");
        std::fs::read_to_string(&output)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", output.display()))
            // Windows paths: compare with forward slashes only.
            .replace('\\', "/")
    }

    /// The part after `cargo::KEY=` or `cargo:KEY=` of every such line.
    fn directives<'a>(output: &'a str, key: &str) -> Vec<&'a str> {
        output
            .lines()
            .filter_map(|line| {
                let rest = line.strip_prefix("cargo:")?;
                let rest = rest.strip_prefix(':').unwrap_or(rest);
                rest.strip_prefix(key)?.strip_prefix('=')
            })
            .collect()
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("create a directory for the copy");
        for entry in std::fs::read_dir(from).expect("read the lab directory") {
            let entry = entry.expect("read a directory entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("file type").is_dir() {
                if entry.file_name() != "target" {
                    copy_dir(&entry.path(), &target);
                }
            } else {
                std::fs::copy(entry.path(), &target).expect("copy a lab file");
            }
        }
    }

    #[test]
    fn a_manifest_declares_the_native_library() {
        let metadata = lab_cargo(
            &lab_dir(),
            &lab_target(),
            &["metadata", "--no-deps", "--format-version", "1"],
        );
        expect_success(&metadata, "`cargo metadata` in sys_lab/");
        assert!(
            metadata.stdout.contains("\"links\":\"fletch\""),
            "fletch-sys/Cargo.toml must say which native library the package links: \
             add the `links` key to [package] with the value \"fletch\""
        );
    }

    #[test]
    fn b_checker_passes_with_default_features() {
        let tested = lab_cargo(&lab_dir(), &lab_target(), &["test", "-p", "checker"]);
        expect_success(&tested, "`cargo test -p checker` in sys_lab/");
    }

    #[test]
    fn b_checker_passes_with_the_wide_feature() {
        let tested = lab_cargo(
            &lab_dir(),
            &lab_target(),
            &["test", "-p", "checker", "--features", "wide"],
        );
        expect_success(
            &tested,
            "`cargo test -p checker --features wide` in sys_lab/",
        );
    }

    #[test]
    fn c_build_script_links_a_static_library_named_fletch() {
        let output = build_script_output();
        assert!(
            directives(&output, "rustc-link-lib").contains(&"static=fletch"),
            "the build script must print `rustc-link-lib=static=fletch`; \
             `cc::Build::compile(\"fletch\")` does that for you"
        );
        assert!(
            !directives(&output, "rustc-link-search").is_empty(),
            "rustc must be told where libfletch.a is (`rustc-link-search`); \
             `cc::Build::compile` does that for you"
        );
    }

    #[test]
    fn c_build_script_names_its_inputs() {
        let output = build_script_output();
        let watched = directives(&output, "rerun-if-changed");
        // A watched path may be the file itself or a directory that contains
        // it: Cargo scans a watched directory for changes. A relative path
        // is relative to the package directory.
        let package = lab_dir().join("fletch-sys");
        for file in [
            "vendor/fletch/src/fletch.c",
            "vendor/fletch/include/fletch.h",
        ] {
            let covered = watched.iter().any(|path| {
                path.ends_with(file) || package.join(file).starts_with(package.join(path))
            });
            assert!(
                covered,
                "the build script must print `cargo::rerun-if-changed=` for {file} \
                 (or for a directory that contains it). \
                 It printed these rerun-if-changed lines: {watched:?}"
            );
        }
    }

    #[test]
    fn c_build_script_publishes_the_include_directory() {
        let output = build_script_output();
        // `cargo::metadata=include=...` is the current spelling and
        // `cargo:include=...` the old one; both set DEP_FLETCH_INCLUDE.
        let mut published = directives(&output, "metadata")
            .into_iter()
            .filter_map(|pair| pair.strip_prefix("include="))
            .chain(directives(&output, "include"));
        let include = published.next().expect(
            "the build script must print `cargo::metadata=include=<directory>` \
             so that dependents can find fletch.h",
        );
        assert!(
            Path::new(include).is_absolute(),
            "publish an absolute path: a dependent's build script runs in a \
             different directory. Got `{include}`"
        );
        assert!(
            Path::new(include).join("fletch.h").is_file(),
            "`{include}` does not contain fletch.h"
        );
    }

    #[test]
    fn d_technique_editing_the_c_source_rebuilds_the_library() {
        // Work on a fresh copy with a fresh target directory, so that the
        // lab's own files are never changed and nothing is left over from an
        // earlier run of this test.
        let copy = harness::scratch_dir("l02-copy");
        let _ = std::fs::remove_dir_all(&copy);
        copy_dir(&lab_dir(), &copy);
        let copy_target = copy.join("target");
        let show = |when: &str| {
            let ran = lab_cargo(
                &copy,
                &copy_target,
                &["run", "--quiet", "-p", "checker", "--bin", "show"],
            );
            expect_success(&ran, &format!("`cargo run -p checker --bin show` ({when})"));
            ran.stdout.trim().to_string()
        };

        let before = show("before the edit");
        assert_eq!(
            before, "c8f0",
            "the Fletcher-16 checksum of \"abcde\", computed by the C library"
        );

        // Change the arithmetic of the C code, and move the file's timestamp
        // into the future so that no file system can miss the change.
        let source = copy.join("fletch-sys/vendor/fletch/src/fletch.c");
        let text = std::fs::read_to_string(&source).expect("read the copied fletch.c");
        assert!(text.contains("% 255"), "the supplied fletch.c was changed");
        std::fs::write(&source, text.replace("% 255", "% 251")).expect("edit the copied fletch.c");
        std::fs::File::options()
            .write(true)
            .open(&source)
            .and_then(|file| file.set_modified(SystemTime::now() + Duration::from_secs(5)))
            .expect("set the modification time of the copied fletch.c");

        let after = show("after the edit");
        assert_ne!(
            after, before,
            "fletch.c changed, but the old C code is still linked: the build \
             script did not run again. `cc` prints rerun-if-env-changed lines, \
             and after the first rerun-if line Cargo watches only what the \
             script names. Print `cargo::rerun-if-changed=` for each C input."
        );
    }

    #[test]
    fn p_predictions() {
        // P1. Cargo.toml of this package says `links = "kvtab"`, and build.rs
        // prints `cargo::metadata=include=<directory>`. Under which name does
        // the build script of a package that depends on this one find that
        // directory in its environment?
        let name = format!("DEP_{}_INCLUDE", env!("M6_LINKS").to_uppercase());
        let p1: &str = guess();
        assert_eq!(p1, name, "P1: how `links` metadata reaches a dependent");

        // P2. Read build.rs. How many static libraries (`.a` files, or `.lib`
        // files with MSVC) does it leave in OUT_DIR?
        let extension = if cfg!(target_env = "msvc") {
            "lib"
        } else {
            "a"
        };
        let archives = std::fs::read_dir(env!("OUT_DIR"))
            .expect("OUT_DIR exists after the build")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == extension))
            .count();
        let p2: usize = guess();
        assert_eq!(p2, archives, "P2: what the build script produces");

        // P3. Cargo sets OUT_DIR and CARGO_MANIFEST_LINKS while build.rs
        // RUNS. Which of the two can the library also read at compile time,
        // with `option_env!`? Write (out_dir_is_set, manifest_links_is_set).
        let visible = (
            option_env!("OUT_DIR").is_some(),
            option_env!("CARGO_MANIFEST_LINKS").is_some(),
        );
        let p3: (bool, bool) = guess();
        assert_eq!(
            p3, visible,
            "P3: which build-script facts the crate can see"
        );
    }
}
