//! LESSON 09 — CI and release: pipelines, packaging, profiles, cross-compilation
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l09_
//! Read first: CONCEPTS.md, "Lesson 09 · CI and release".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: CI is green on Monday and red on
//! Tuesday, and nobody changed the code. Name three things that may have changed.
//! IMPLEMENT: no Rust bodies here. You prepare `fixtures/l09_ship/` (the crate
//! `shipit`) for its first release: a CI pipeline, the content of the published
//! package, the release profile, and a cross-compilation setting.
//! EXPLAIN afterward: Explain what makes a build reproducible, and which inputs of
//! a CI run you have pinned after this lesson.
//! JUDGMENT: CI should run exactly the commands a contributor can run locally, with
//! everything that can change by itself pinned: toolchain, dependencies (`--locked`),
//! runner image, action versions.
//!
//! TASKS (edit only files under `fixtures/l09_ship/`):
//!
//! ```text
//! A. Rewrite `.github/workflows/ci.yml`:
//!    - It runs on pull requests and on pushes, cancels superseded runs
//!      (`concurrency` with `cancel-in-progress: true`), and gives its token
//!      read-only access (`permissions: contents: read`).
//!    - Every action is pinned to a version tag or a commit, never to a branch, and
//!      no unmaintained `actions-rs/*` action is used. No step or job has
//!      `continue-on-error: true`.
//!    - The Rust toolchain is the exact release 1.99.0, written in one place:
//!      either once in the workflow (for example a workflow-level `env:` entry),
//!      or in a `rust-toolchain.toml` next to `Cargo.toml`, which the workflow
//!      lets rustup read (`rustup toolchain install` without a version).
//!    - Build output is cached (`Swatinem/rust-cache` or `actions/cache`).
//!    - Checks, in any arrangement of jobs:
//!        cargo fmt --all --check
//!        cargo clippy ... --all-targets --locked -- -D warnings
//!        cargo test ... --locked      on Linux, macOS and Windows (a matrix with
//!                                     `fail-fast: false`); doc tests must run too
//!        cargo doc ... --no-deps      with RUSTDOCFLAGS containing `-D warnings`
//!        cargo deny check             (a `run:` step or `EmbarkStudios/cargo-deny-action`)
//!      `cargo deny check` reads `deny.toml` next to `Cargo.toml`. Without that file
//!      it allows no licence at all and rejects this crate's own MIT. So write a
//!      short `deny.toml` whose `[licenses]` `allow` list contains MIT (lesson 08
//!      shows the full file).
//!    - A job whose name contains `msrv` installs Rust 1.85.0 (the crate's
//!      `rust-version`) and runs `cargo check ... --locked`.
//! B. Package content. `cargo package --list` must contain the sources, README.md,
//!    LICENSE and CHANGELOG.md, and nothing from `notes/`, `testdata/`, `.github/`
//!    or `.cargo/` (the directory you create in task D). Inside a git repository
//!    Cargo packages every file that git does not ignore, hidden directories
//!    included; outside one it skips hidden files. Do not rely on either.
//!    Add `description`, `readme` and `repository` (an `https://example.com/...`
//!    address). Keep `publish = false`.
//! C. A release profile for a small command-line tool that ships to users:
//!    link-time optimisation (`lto` "thin" or "fat"), `codegen-units = 1`, and
//!    symbols stripped.
//! D. `.cargo/config.toml` names the linker `aarch64-linux-gnu-gcc` for the target
//!    `aarch64-unknown-linux-gnu`.
//! ```

#[cfg(test)]
mod tests {
    use crate::guess;
    use crate::support::workflow::{self, Step};
    use crate::support::{self, Value, cargo, fixture, text_at};
    use std::path::PathBuf;

    fn krate() -> PathBuf {
        fixture("l09_ship")
    }

    fn ci() -> (Value, Vec<Step>, String) {
        let path = krate().join(".github/workflows/ci.yml");
        let doc = support::yaml(&path);
        let steps = workflow::steps(&doc);
        (doc, steps, support::read(&path))
    }

    #[test]
    fn a_ci_triggers_permissions_and_pins() {
        let (doc, steps, _) = ci();
        let triggers = workflow::triggers(&doc);
        for event in ["pull_request", "push"] {
            assert!(
                triggers.iter().any(|t| t == event),
                "the workflow must run on `{event}`; `on:` has {triggers:?}"
            );
        }
        assert_eq!(
            doc.pointer("/concurrency/cancel-in-progress"),
            Some(&Value::Bool(true)),
            "add a `concurrency` group with `cancel-in-progress: true`"
        );
        assert_eq!(
            text_at(&doc, "/permissions/contents"),
            Some("read"),
            "give the workflow token least privilege: `permissions: contents: read`"
        );
        let floating = workflow::floating_actions(&steps);
        assert!(
            floating.is_empty(),
            "actions pinned to a branch can change under you: {floating:?}"
        );
        let abandoned: Vec<&str> = steps
            .iter()
            .filter_map(|step| step.uses.as_deref())
            .filter(|uses| uses.starts_with("actions-rs/"))
            .collect();
        assert!(
            abandoned.is_empty(),
            "the actions-rs actions are no longer maintained: {abandoned:?}"
        );
        assert!(
            !steps.iter().any(|step| step.continue_on_error),
            "`continue-on-error: true` turns a failing check into a green one"
        );
        for (key, job) in workflow::jobs(&doc) {
            let first = job.pointer("/steps/0/uses").and_then(Value::as_str);
            assert!(
                first.is_some_and(|uses| uses.starts_with("actions/checkout@")),
                "job `{key}` must start by checking out the repository"
            );
        }
    }

    #[test]
    fn a_ci_pins_the_toolchain_and_caches() {
        let (_, steps, text) = ci();
        let in_workflow = text.matches("1.99.0").count();
        let toolchain_file = krate().join("rust-toolchain.toml");
        let in_file = toolchain_file.is_file()
            && text_at(&support::toml(&toolchain_file), "/toolchain/channel") == Some("1.99.0");
        assert!(
            in_workflow > 0 || in_file,
            "the workflow must install the exact toolchain 1.99.0: name it in the \
             workflow, or pin it in fixtures/l09_ship/rust-toolchain.toml"
        );
        assert!(
            in_workflow + usize::from(in_file) == 1,
            "write the toolchain version in one place (for example a workflow-level \
             `env:` entry) and refer to it; it appears {in_workflow} times in the \
             workflow{}",
            if in_file {
                " and once more in rust-toolchain.toml"
            } else {
                ""
            }
        );
        assert!(
            !in_file || workflow::installs_toolchain_from_file(&steps),
            "the version is in rust-toolchain.toml, so let rustup read it: run \
             `rustup toolchain install` without a version in the jobs that build"
        );
        for moving in [
            "toolchain: stable",
            "@stable",
            "install stable",
            "default stable",
        ] {
            assert!(
                !text.contains(moving),
                "`{moving}`: a moving toolchain breaks CI on release day without any commit"
            );
        }
        assert!(
            workflow::find_action(&steps, "Swatinem/rust-cache").is_some()
                || workflow::find_action(&steps, "actions/cache").is_some(),
            "cache the build: `Swatinem/rust-cache@v2` or `actions/cache`"
        );
    }

    #[test]
    fn a_ci_runs_the_five_checks() {
        let (doc, steps, _) = ci();
        assert!(
            workflow::find_command(&steps, &["cargo", "fmt", "--all", "--check"]).is_some(),
            "missing: cargo fmt --all --check"
        );
        assert!(
            workflow::find_command(
                &steps,
                &[
                    "cargo",
                    "clippy",
                    "--all-targets",
                    "--locked",
                    "--",
                    "-D",
                    "warnings"
                ]
            )
            .is_some(),
            "missing: cargo clippy ... --all-targets --locked -- -D warnings"
        );

        let test = workflow::find_command(&steps, &["cargo", "test", "--locked"])
            .expect("missing: cargo test ... --locked");
        let skips_doc_tests = test
            .commands
            .iter()
            .any(|line| line.contains("cargo test") && line.contains("--all-targets"));
        assert!(
            !skips_doc_tests
                || workflow::find_command(&steps, &["cargo", "test", "--doc"]).is_some(),
            "`cargo test --all-targets` does not run doc tests; drop the flag or add \
             a `cargo test --doc` step"
        );
        let job = workflow::jobs(&doc)
            .into_iter()
            .find(|(key, _)| *key == test.job)
            .map(|(_, job)| job)
            .expect("the test step belongs to a job");
        let systems = support::list_at(&job, "/strategy/matrix/os");
        for family in ["ubuntu", "macos", "windows"] {
            assert!(
                systems.iter().any(|os| os.starts_with(family)),
                "the test job needs a matrix `os` with a {family} runner; found {systems:?}"
            );
        }
        assert_eq!(
            job.pointer("/strategy/fail-fast"),
            Some(&Value::Bool(false)),
            "set `fail-fast: false`: when Windows fails you still want to know \
             whether macOS passes"
        );
        assert_eq!(
            text_at(&job, "/runs-on"),
            Some("${{ matrix.os }}"),
            "the test job must run on the matrix value: `runs-on: ${{{{ matrix.os }}}}`"
        );

        let doc_step = workflow::find_command(&steps, &["cargo", "doc", "--no-deps"])
            .expect("missing: cargo doc ... --no-deps");
        assert!(
            doc_step
                .env
                .get("RUSTDOCFLAGS")
                .is_some_and(|flags| flags.contains("-D warnings")),
            "the doc step needs `RUSTDOCFLAGS: -D warnings` in its `env:`"
        );
        assert!(
            workflow::find_command(&steps, &["cargo", "deny", "check"]).is_some()
                || workflow::find_action(&steps, "EmbarkStudios/cargo-deny-action").is_some(),
            "missing: cargo deny check"
        );
        let policy = krate().join("deny.toml");
        assert!(
            policy.is_file(),
            "`cargo deny check` without a `deny.toml` allows no licence at all, so the \
             pipeline would reject this crate's own MIT. Create fixtures/l09_ship/deny.toml."
        );
        let allowed = support::list_at(&support::toml(&policy), "/licenses/allow");
        assert!(
            allowed.iter().any(|licence| licence == "MIT"),
            "deny.toml: the `allow` list under [licenses] must contain MIT, the licence \
             of this crate; found {allowed:?}"
        );
    }

    #[test]
    fn a_ci_checks_the_msrv() {
        let (doc, steps, _) = ci();
        let manifest = support::toml(&krate().join("Cargo.toml"));
        let msrv = text_at(&manifest, "/package/rust-version").expect("rust-version is set");
        let (key, _) = workflow::jobs(&doc)
            .into_iter()
            .find(|(key, _)| key.to_lowercase().contains("msrv"))
            .expect("add a job whose key contains `msrv`");
        let mine: Vec<Step> = steps.into_iter().filter(|step| step.job == key).collect();
        let installs = mine.iter().any(|step| {
            step.commands
                .iter()
                .any(|line| line.contains("rustup") && line.contains(msrv))
                || step.uses.as_deref().is_some_and(|uses| uses.contains(msrv))
                || step.with.to_string().contains(msrv)
        });
        assert!(
            installs,
            "the `{key}` job must install Rust {msrv}, the `rust-version` of Cargo.toml"
        );
        let check = workflow::find_command(&mine, &["cargo", "check", "--locked"])
            .unwrap_or_else(|| panic!("the `{key}` job must run `cargo check ... --locked`"));
        if krate().join("rust-toolchain.toml").is_file() {
            // Lesson 02: a toolchain file beats `rustup default`.
            let selected = check
                .commands
                .iter()
                .any(|line| line.contains(&format!("+{msrv}")))
                || check
                    .env
                    .get("RUSTUP_TOOLCHAIN")
                    .is_some_and(|toolchain| toolchain.starts_with(msrv));
            assert!(
                selected,
                "this crate has a rust-toolchain.toml, and that file beats `rustup \
                 default`. In the `{key}` job, select the old compiler for the command \
                 itself: `cargo +{msrv}.0 check ... --locked`, or `RUSTUP_TOOLCHAIN` in `env:`"
            );
        }
    }

    #[test]
    fn b_package_contains_what_users_need_and_nothing_else() {
        let _turn = support::take_turn();
        let manifest = support::toml(&krate().join("Cargo.toml"));
        assert_eq!(
            manifest.pointer("/package/publish"),
            Some(&Value::Bool(false)),
            "keep `publish = false` in this exercise"
        );
        assert!(
            text_at(&manifest, "/package/description").is_some_and(|d| d.len() >= 10),
            "crates.io refuses a package without a `description`"
        );
        assert_eq!(text_at(&manifest, "/package/readme"), Some("README.md"));
        assert!(
            text_at(&manifest, "/package/repository")
                .is_some_and(|url| url.starts_with("https://example.com/")),
            "`repository` must be an https://example.com/... address in this exercise"
        );

        let list = cargo(&krate(), &["package", "--list", "--allow-dirty"]);
        list.expect_ok("`cargo package --list --allow-dirty` must work");
        // Cargo prints the paths with the separator of the platform.
        let listed = list.stdout.replace('\\', "/");
        let files: Vec<&str> = listed.lines().map(str::trim).collect();
        for needed in [
            "Cargo.toml",
            "src/lib.rs",
            "src/main.rs",
            "README.md",
            "LICENSE",
            "CHANGELOG.md",
        ] {
            assert!(
                files.contains(&needed),
                "the package must contain {needed}; it contains {files:?}"
            );
        }
        for file in &files {
            let unwanted = ["notes/", "testdata/", ".github/", ".cargo/"]
                .iter()
                .any(|prefix| file.starts_with(prefix));
            assert!(
                !unwanted,
                "{file} must not be published: use `include` or `exclude` in [package]"
            );
        }
        assert!(
            support::cargo_warnings(&list).is_empty(),
            "Cargo still warns about the package:\n{}",
            list.stderr
        );

        // Technique: build the crate from the packaged files only. A package that
        // lost a file it needs (the README named in `readme`, a module) fails here.
        cargo(&krate(), &["package", "--allow-dirty"])
            .expect_ok("the packaged crate must build on its own (`cargo package`)");
    }

    #[test]
    fn c_release_profile() {
        let manifest = support::toml(&krate().join("Cargo.toml"));
        let lto = manifest.pointer("/profile/release/lto");
        assert!(
            matches!(lto, Some(Value::Bool(true)) | Some(Value::String(_)))
                && lto != Some(&Value::String("off".to_owned())),
            "[profile.release] must switch on `lto` (\"thin\", \"fat\" or true)"
        );
        assert_eq!(
            manifest
                .pointer("/profile/release/codegen-units")
                .and_then(Value::as_i64),
            Some(1),
            "[profile.release] must set `codegen-units = 1`"
        );
        let strip = manifest.pointer("/profile/release/strip");
        assert!(
            matches!(strip, Some(Value::Bool(true)))
                || matches!(strip, Some(Value::String(s)) if s == "symbols" || s == "debuginfo"),
            "[profile.release] must set `strip` (\"symbols\", \"debuginfo\" or true)"
        );
        let _turn = support::take_turn();
        cargo(&krate(), &["build", "--release"]).expect_ok("the release build must work");
    }

    #[test]
    fn d_cross_linker_is_configured() {
        let config = support::toml(&krate().join(".cargo/config.toml"));
        assert_eq!(
            text_at(&config, "/target/aarch64-unknown-linux-gnu/linker"),
            Some("aarch64-linux-gnu-gcc"),
            "[target.aarch64-unknown-linux-gnu] must name the cross linker"
        );
    }

    /// The value of `key` in the output of `rustc --print cfg --target <target>`.
    fn target_cfg(target: &str, key: &str) -> String {
        let cargo_path = PathBuf::from(support::cargo_program());
        let beside = cargo_path.with_file_name(format!("rustc{}", std::env::consts::EXE_SUFFIX));
        let rustc = if beside.is_file() {
            beside
        } else {
            PathBuf::from("rustc")
        };
        let mut command = std::process::Command::new(rustc);
        command.args(["--print", "cfg", "--target", target]);
        let run = support::run(
            command,
            format!("rustc --print cfg --target {target}"),
            std::time::Duration::from_secs(60),
        );
        run.expect_ok("rustc knows the cfg of every built-in target, installed or not");
        run.stdout
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(|value| value.trim_matches('"').to_owned())
            .unwrap_or_default()
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// P1 to P4: read `fixtures/l09_profiles/` (a manifest with profiles, and a
    /// `main` that panics twice).
    #[test]
    fn p_predictions() {
        let _turn = support::take_turn();
        let profiles = fixture("l09_profiles");
        let run = |profile: &str| cargo(&profiles, &["run", "-q", "--profile", profile]);
        let release = run("release");
        let small = run("small");

        // P1. `cargo run --release`: the third line starts with "200 + 100 as u8: ".
        // What follows: "44" or "panic"?
        let third = release.stdout.lines().nth(2).unwrap_or_default();
        let p1: &str = guess();
        assert_eq!(
            p1,
            third.trim_start_matches("200 + 100 as u8: "),
            "P1: a profile is a set of settings; `release` only names their defaults"
        );

        // P2. Same run: is "guard dropped" printed when the final panic unwinds `main`?
        let p2: bool = guess();
        assert_eq!(
            p2,
            release.stdout.contains("guard dropped"),
            "P2: unwinding runs destructors"
        );

        // P3. `cargo run --profile small` (panic = "abort"): how many lines does the
        // program print to stdout before it stops?
        let p3: usize = guess();
        assert_eq!(
            p3,
            small.stdout.lines().count(),
            "P3: with panic = \"abort\" the first panic ends the process; \
             catch_unwind has nothing to catch"
        );

        // P4. Same run: is the exit code 101, as after an unwinding panic?
        let p4: bool = guess();
        assert_eq!(
            p4,
            small.code == Some(101),
            "P4: abort is not an orderly exit"
        );

        // P5. In which directory under target/ does `--profile small` put the
        // binary, and in which does `--profile dev`? Answer with the two names.
        let nested = support::nested_target_dir(&profiles);
        cargo(&profiles, &["build", "-q"]).expect_ok("the dev build must work");
        let exists = |dir: &str| {
            nested
                .join(dir)
                .join(format!("profiles{}", std::env::consts::EXE_SUFFIX))
                .is_file()
        };
        let p5: (&str, &str) = guess();
        assert!(
            exists(p5.0) && exists(p5.1) && p5.0 != p5.1,
            "P5: a custom profile gets a directory with its own name; \
             `dev` is the historical exception"
        );

        // P6. Cross-compilation starts with knowing the target. What is
        // `target_env` for the target `x86_64-unknown-linux-musl`, and what is
        // `target_family` for `wasm32-unknown-unknown`?
        let p6: (&str, &str) = guess();
        assert_eq!(
            p6,
            (
                target_cfg("x86_64-unknown-linux-musl", "target_env").as_str(),
                target_cfg("wasm32-unknown-unknown", "target_family").as_str()
            ),
            "P6: `rustc --print cfg --target <triple>` lists what #[cfg] sees"
        );
    }
}
