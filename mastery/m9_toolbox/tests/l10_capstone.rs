//! LESSON 10 — Capstone: turn `capstone/` into a well-run workspace
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --test l10_capstone
//!
//! The task is described in `capstone/TASK.md`. There is no `todo!()` here and no
//! prediction: the starting point is a small project in a bad state, and every test
//! below checks one thing a maintainer would have put in order. The tests run real
//! Cargo commands inside `capstone/`.
//!
//! EXPLAIN afterward: Which three of the changes you made would you do first in a
//! real project that is in this state, and why those?

use m9_toolbox::support::workflow::{self, Step};
use m9_toolbox::support::{self, Run, Value, list_at, text_at};
use std::path::{Path, PathBuf};

fn ws() -> PathBuf {
    support::package_root().join("capstone")
}

/// Runs Cargo in the capstone workspace. Downloads are allowed here, because the
/// workspace depends on `insta`.
#[track_caller]
fn cargo(args: &[&str]) -> Run {
    support::cargo_with(&ws(), args, &[("CARGO_NET_OFFLINE", "")])
}

fn root_manifest() -> Value {
    support::toml(&ws().join("Cargo.toml"))
}

fn tally(args: &[&str], features: &[&str]) -> Run {
    let mut all = vec!["run", "-q", "--locked", "-p", "tally"];
    all.extend_from_slice(features);
    all.push("--");
    all.extend_from_slice(args);
    cargo(&all)
}

fn ledger(text: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.txt");
    std::fs::write(&path, text).expect("the ledger file can be written");
    let shown = path.to_string_lossy().into_owned();
    (dir, shown)
}

fn sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, found: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                if name != "target" && name != "corpus" && name != "artifacts" {
                    walk(&path, found);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push((path.clone(), support::read(&path)));
            }
        }
    }
    let mut found = Vec::new();
    walk(&ws(), &mut found);
    found
}

#[test]
fn l10_1_workspace_with_inherited_fields() {
    let root = root_manifest();
    assert!(
        root.get("package").is_none(),
        "capstone/Cargo.toml is a virtual manifest: [workspace], no [package]"
    );
    assert_eq!(text_at(&root, "/workspace/resolver"), Some("3"));
    for (key, expected) in [
        ("version", "0.4.0"),
        ("edition", "2024"),
        ("license", "MIT"),
        ("rust-version", "1.99"),
    ] {
        assert_eq!(
            text_at(&root, &format!("/workspace/package/{key}")),
            Some(expected),
            "[workspace.package] must set `{key}`"
        );
    }
    for name in ["tally-core", "insta"] {
        assert!(
            root.pointer(&format!("/workspace/dependencies/{name}"))
                .is_some(),
            "[workspace.dependencies] must declare `{name}`"
        );
    }
    for member in ["tally-core", "tally"] {
        let manifest = support::toml(&ws().join(member).join("Cargo.toml"));
        for key in ["version", "edition", "license", "rust-version"] {
            assert_eq!(
                manifest.pointer(&format!("/package/{key}/workspace")),
                Some(&Value::Bool(true)),
                "{member}/Cargo.toml must inherit `{key}`"
            );
        }
        assert_eq!(
            manifest.pointer("/lints/workspace"),
            Some(&Value::Bool(true)),
            "{member}/Cargo.toml must inherit the workspace lints"
        );
    }
    let toolchain = support::toml(&ws().join("rust-toolchain.toml"));
    assert_eq!(text_at(&toolchain, "/toolchain/channel"), Some("1.99.0"));
    let components = list_at(&toolchain, "/toolchain/components");
    assert!(
        components.iter().any(|c| c == "rustfmt") && components.iter().any(|c| c == "clippy"),
        "rust-toolchain.toml must list rustfmt and clippy"
    );

    let _turn = support::take_turn();
    let run = cargo(&["metadata", "--format-version", "1", "--no-deps", "--locked"]);
    run.expect_ok("`cargo metadata --locked` must load the workspace (is Cargo.lock there?)");
    assert!(
        support::cargo_warnings(&run).is_empty(),
        "Cargo warns about the workspace:\n{}",
        run.stderr
    );
    let metadata: Value = serde_json::from_str(&run.stdout).expect("metadata is JSON");
    let mut names: Vec<&str> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| package["name"].as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["tally", "tally-core"],
        "the workspace has exactly these two members (the fuzz project is not one)"
    );
}

#[test]
fn l10_2_lints_and_formatting() {
    let root = root_manifest();
    assert_eq!(
        text_at(&root, "/workspace/lints/rust/unsafe_code"),
        Some("forbid")
    );
    assert_eq!(
        text_at(&root, "/workspace/lints/rust/missing_docs"),
        Some("warn")
    );
    assert_eq!(
        text_at(&root, "/workspace/lints/clippy/pedantic/level"),
        Some("warn")
    );
    assert!(
        root.pointer("/workspace/lints/clippy/pedantic/priority")
            .and_then(Value::as_i64)
            .is_some_and(|priority| priority < 0),
        "the pedantic group needs a negative priority"
    );
    for (path, text) in sources() {
        let squeezed = support::squeeze(&text);
        assert!(
            !squeezed.contains("allow(warnings)") && !squeezed.contains("deny(warnings)"),
            "{} still has a blanket `allow(warnings)` or `deny(warnings)`",
            support::shown(&path)
        );
    }
    let _turn = support::take_turn();
    cargo(&["fmt", "--all", "--check"]).expect_ok("`cargo fmt --all --check` must pass");
    let clippy = cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--message-format=json",
        "--",
        "-D",
        "warnings",
    ]);
    let problems = clippy.diagnostics("error");
    assert!(
        clippy.ok() && problems.is_empty(),
        "`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` \
         must pass. Still reported: {:?}\n{}\n{}",
        support::codes(&problems),
        problems.first().map_or("", |d| d.rendered.as_str()),
        clippy.stderr
    );
}

#[test]
fn l10_3_json_is_an_optional_feature() {
    let core = support::toml(&ws().join("tally-core/Cargo.toml"));
    assert!(
        core.pointer("/features/json").is_some(),
        "tally-core needs a feature `json`"
    );
    assert!(
        !list_at(&core, "/features/default")
            .iter()
            .any(|f| f == "json"),
        "`json` must be off by default in the library"
    );
    let cli = support::toml(&ws().join("tally/Cargo.toml"));
    assert_eq!(
        list_at(&cli, "/features/json"),
        ["tally-core/json"],
        "tally's feature `json` forwards to `tally-core/json`"
    );
    assert!(
        list_at(&cli, "/features/default")
            .iter()
            .any(|f| f == "json"),
        "`json` must be on by default in the binary"
    );

    let _turn = support::take_turn();
    for flags in [&["--no-default-features"][..], &["--all-features"][..]] {
        let mut args = vec!["check", "--workspace", "--locked", "--message-format=json"];
        args.extend_from_slice(flags);
        let check = cargo(&args);
        check.expect_ok("the workspace must build with and without the feature");
        let warnings = check.diagnostics("warning");
        assert!(
            warnings.is_empty(),
            "`cargo {}` reports warnings: {:?}",
            args.join(" "),
            support::codes(&warnings)
        );
    }
    let (_dir, file) = ledger("2026-11-03 groceries 12.50\n");
    let without = tally(&["report", "--json", &file], &["--no-default-features"]);
    assert_eq!(
        without.code,
        Some(2),
        "without the feature, `--json` is a usage error (exit code 2); stderr:\n{}",
        without.stderr
    );
    assert!(
        without.stderr.to_lowercase().contains("json"),
        "the message must say that this build has no JSON support"
    );
    let table = tally(&["report", &file], &["--no-default-features"]);
    assert_eq!(table.stdout, "groceries  12.50\ntotal      12.50\n");
}

#[test]
fn l10_4_behaviour() {
    let _turn = support::take_turn();
    let (_dir, file) = ledger(
        "# November\n2026-11-02 rent 1200\n2026-11-03 groceries 12.50\n\
         2026-11-05\tbooks\t31.98\n\n2026-11-09 groceries 45.8\n",
    );
    let report = tally(&["report", &file], &[]);
    report.expect_ok("`tally report <file>` must work");
    assert_eq!(
        report.stdout,
        "books        31.98\ngroceries    58.30\nrent       1200.00\ntotal      1290.28\n",
        "sorted by name, aligned columns, `12.5` means 12.50, `1200` means 1200.00"
    );
    let json = tally(&["report", "--json", &file], &[]);
    json.expect_ok("`tally report --json <file>` must work");
    assert_eq!(
        json.stdout.trim_end(),
        r#"[{"date":"2026-11-02","category":"rent","cents":120000},{"date":"2026-11-03","category":"groceries","cents":1250},{"date":"2026-11-05","category":"books","cents":3198},{"date":"2026-11-09","category":"groceries","cents":4580}]"#
    );

    let (_dir, quoted) = ledger("2026-11-03 say\"hi\\ 1\n");
    assert_eq!(
        tally(&["report", "--json", &quoted], &[]).stdout.trim_end(),
        r#"[{"date":"2026-11-03","category":"say\"hi\\","cents":100}]"#,
        "quotes and backslashes in a category must be escaped in JSON"
    );

    let (_dir, empty) = ledger("# nothing yet\n");
    assert_eq!(tally(&["report", &empty], &[]).stdout, "total  0.00\n");

    // Technique: each amount fits in 64 bits (signed or unsigned); their sum does
    // not. A plain `+=` panics here in a debug build and wraps in a release build.
    let (_dir, large) = ledger(&"2026-11-03 big 92233720368547758.07\n".repeat(3));
    let sum = tally(&["report", &large], &[]);
    assert_eq!(
        sum.code,
        Some(0),
        "three large amounts must not stop the program (101 is a panic: the sum \
         overflowed); stderr:\n{}",
        sum.stderr
    );
    assert_eq!(
        sum.stdout, "big    276701161105643274.21\ntotal  276701161105643274.21\n",
        "sums must be exact when they no longer fit in 64 bits: add in a wider type"
    );

    assert_eq!(tally(&[], &[]).code, Some(2), "no arguments: exit code 2");
    assert_eq!(
        tally(&["report"], &[]).code,
        Some(2),
        "no file: exit code 2"
    );
    assert_eq!(
        tally(&["report", "--json"], &[]).code,
        Some(2),
        "`--json` without a file: exit code 2"
    );
    let missing = tally(&["report", "this-file-does-not-exist.txt"], &[]);
    assert_eq!(missing.code, Some(1), "an unreadable file: exit code 1");
}

#[test]
fn l10_4_bad_input_is_an_error_not_a_panic() {
    let _turn = support::take_turn();
    let (_dir, file) = ledger("# header\n2026-11-03 groceries 12.50\n2026-11-04 oops\n");
    let run = tally(&["report", &file], &[]);
    assert_eq!(
        run.code,
        Some(1),
        "a bad line gives exit code 1; stderr:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.starts_with("line 3:"),
        "the message must start with the line number, `line 3:`; got {:?}",
        run.stderr
    );
    assert_eq!(
        run.stdout, "",
        "nothing is printed to stdout for a bad ledger"
    );

    for bad in [
        "x",
        "2026-11-03",
        "2026-11-03 food",
        "2026-11-03 food 1 extra",
        "2026-13-01 food 1",
        "2026-11-00 food 1",
        "20261103 food 1",
        "2026-11-03 food 1.234",
        "2026-11-03 food .5",
        "2026-11-03 food 5.",
        "2026-11-03 food -5",
        "2026-11-03 food 1e3",
        "2026-11-03 food 99999999999999999999999",
        "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9} food 1",
        "2026-11-0\u{e9} food 1",
        "2026\u{2011}11\u{2011}03 food 1",
        "2026-11-03 food \u{663}",
    ] {
        let (_dir, file) = ledger(&format!("{bad}\n"));
        let run = tally(&["report", &file], &[]);
        assert_eq!(
            run.code,
            Some(1),
            "the line {bad:?} must be rejected with exit code 1 (101 is a panic); stderr:\n{}",
            run.stderr
        );
        assert!(
            run.stderr.starts_with("line 1:") && !run.stderr.contains("panicked"),
            "the line {bad:?} must give a `line 1:` message, got {:?}",
            run.stderr
        );
    }
}

#[test]
fn l10_5_tests_with_a_snapshot() {
    let files = sources();
    assert!(
        files
            .iter()
            .any(|(_, text)| text.contains("assert_snapshot!")),
        "write at least one snapshot test with `insta::assert_snapshot!`"
    );
    fn snapshots(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if entry.file_name() != "target" {
                    snapshots(&path, found);
                }
            } else if path.extension().is_some_and(|ext| ext == "snap") {
                found.push(path);
            }
        }
    }
    let mut accepted = Vec::new();
    snapshots(&ws(), &mut accepted);
    assert!(
        accepted
            .iter()
            .any(|path| support::read(path).contains("total")),
        "commit the accepted `.snap` file of the report next to the test"
    );

    let _turn = support::take_turn();
    let run = support::cargo_with(
        &ws(),
        &["test", "--workspace", "--all-features", "--locked"],
        // A snapshot that does not match must fail, not be rewritten.
        &[("CARGO_NET_OFFLINE", ""), ("INSTA_UPDATE", "no")],
    );
    run.expect_ok("`cargo test --workspace --all-features --locked` must pass");
    let passed = run
        .stdout
        .lines()
        .filter(|line| line.starts_with("test ") && line.ends_with("... ok"))
        .count();
    assert!(
        passed >= 5,
        "TASK.md asks for at least five tests in total (parser unit tests, a snapshot \
         test, a doc test); found {passed} that pass"
    );
    assert!(
        run.stdout.lines().any(|line| {
            line.starts_with("test ") && line.contains("(line ") && line.ends_with("... ok")
        }),
        "at least one doc test must run"
    );
}

#[test]
fn l10_6_fuzz_target() {
    let fuzz = ws().join("fuzz");
    let manifest = support::toml(&fuzz.join("Cargo.toml"));
    assert_eq!(
        manifest.pointer("/package/metadata/cargo-fuzz"),
        Some(&Value::Bool(true)),
        "fuzz/Cargo.toml needs `[package.metadata] cargo-fuzz = true`"
    );
    assert!(
        manifest.pointer("/dependencies/libfuzzer-sys").is_some(),
        "the fuzz project depends on libfuzzer-sys"
    );
    assert!(
        text_at(&manifest, "/dependencies/tally-core/path").is_some(),
        "the fuzz project depends on tally-core by path"
    );
    let target = manifest
        .pointer("/bin/0/path")
        .and_then(Value::as_str)
        .expect("fuzz/Cargo.toml needs a [[bin]] with a `path` for the fuzz target");
    let source = support::read(&fuzz.join(target));
    assert!(
        source.contains("#![no_main]") && source.contains("fuzz_target!"),
        "a fuzz target is `#![no_main]` and defines its entry with `fuzz_target!`"
    );
    assert!(
        source.contains("tally_core::"),
        "the fuzz target must call into tally_core"
    );
    let root = root_manifest();
    let excluded = list_at(&root, "/workspace/exclude")
        .iter()
        .any(|e| e == "fuzz");
    assert!(
        excluded || manifest.get("workspace").is_some(),
        "keep the fuzz project out of the workspace: `exclude = [\"fuzz\"]` in the \
         root manifest, or an empty [workspace] table in fuzz/Cargo.toml"
    );
}

#[test]
fn l10_7_documentation() {
    let readme = support::read(&ws().join("README.md"));
    assert!(
        readme.contains("YYYY-MM-DD") && readme.contains("report"),
        "README.md must explain the input format (`YYYY-MM-DD category amount`) and \
         show the `tally report` command"
    );
    let changelog = support::read(&ws().join("CHANGELOG.md"));
    let headings: Vec<&str> = changelog
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect();
    assert_eq!(
        headings.first().copied(),
        Some("## [Unreleased]"),
        "CHANGELOG.md starts with an `## [Unreleased]` section"
    );
    assert!(
        headings.iter().any(|h| h.starts_with("## [0.4.0] - 20")),
        "CHANGELOG.md needs a section `## [0.4.0] - YYYY-MM-DD`"
    );
    let _turn = support::take_turn();
    support::cargo_with(
        &ws(),
        &[
            "doc",
            "--workspace",
            "--no-deps",
            "--all-features",
            "--locked",
        ],
        &[("CARGO_NET_OFFLINE", ""), ("RUSTDOCFLAGS", "-D warnings")],
    )
    .expect_ok(
        "`RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps --all-features` must pass",
    );
}

#[test]
fn l10_8_policy() {
    let policy = support::toml(&ws().join("deny.toml"));
    assert_eq!(text_at(&policy, "/advisories/yanked"), Some("deny"));
    let allowed = list_at(&policy, "/licenses/allow");
    assert!(
        allowed.iter().any(|l| l == "MIT") && !allowed.iter().any(|l| l.contains("GPL")),
        "[licenses] allow must contain MIT and no GPL-family licence; found {allowed:?}"
    );
    assert_eq!(text_at(&policy, "/bans/multiple-versions"), Some("deny"));
    assert_eq!(text_at(&policy, "/sources/unknown-registry"), Some("deny"));
    assert_eq!(text_at(&policy, "/sources/unknown-git"), Some("deny"));
}

#[test]
fn l10_9_ci_pipeline() {
    let path = ws().join(".github/workflows/ci.yml");
    let doc = support::yaml(&path);
    let text = support::read(&path);
    let steps: Vec<Step> = workflow::steps(&doc);
    let triggers = workflow::triggers(&doc);
    assert!(
        triggers.iter().any(|t| t == "push") && triggers.iter().any(|t| t == "pull_request"),
        "the workflow runs on push and pull_request; `on:` has {triggers:?}"
    );
    assert_eq!(text_at(&doc, "/permissions/contents"), Some("read"));
    // `rust-toolchain.toml` (checked in the first test) pins 1.99.0. The workflow
    // either repeats that version or lets rustup read the file.
    assert!(
        text.contains("1.99.0") || workflow::installs_toolchain_from_file(&steps),
        "the workflow must use the pinned 1.99.0 toolchain: name the version, or run \
         `rustup toolchain install` without a version, which reads rust-toolchain.toml"
    );
    let floating = workflow::floating_actions(&steps);
    assert!(
        floating.is_empty(),
        "actions pinned to a branch: {floating:?}"
    );
    assert!(
        !steps.iter().any(|step| step.continue_on_error),
        "no `continue-on-error: true`"
    );
    assert!(
        workflow::find_action(&steps, "Swatinem/rust-cache").is_some()
            || workflow::find_action(&steps, "actions/cache").is_some(),
        "cache the build"
    );
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
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings"
            ]
        )
        .is_some(),
        "missing: cargo clippy --workspace --all-targets ... --locked -- -D warnings"
    );
    let test = workflow::find_command(&steps, &["cargo", "test", "--workspace", "--locked"])
        .expect("missing: cargo test --workspace ... --locked");
    let job = workflow::jobs(&doc)
        .into_iter()
        .find(|(key, _)| *key == test.job)
        .map(|(_, job)| job)
        .expect("the test step belongs to a job");
    let systems = list_at(&job, "/strategy/matrix/os");
    for family in ["ubuntu", "macos", "windows"] {
        assert!(
            systems.iter().any(|os| os.starts_with(family)),
            "the test job needs a {family} runner in its matrix; found {systems:?}"
        );
    }
    let doc_step = workflow::find_command(&steps, &["cargo", "doc", "--no-deps"])
        .expect("missing: cargo doc ... --no-deps");
    assert!(
        doc_step
            .env
            .get("RUSTDOCFLAGS")
            .is_some_and(|flags| flags.contains("-D warnings")),
        "the doc step needs `RUSTDOCFLAGS: -D warnings`"
    );
    assert!(
        workflow::find_command(&steps, &["cargo", "deny", "check"]).is_some()
            || workflow::find_action(&steps, "EmbarkStudios/cargo-deny-action").is_some(),
        "missing: cargo deny check"
    );
    for step in &steps {
        for line in &step.commands {
            let words: Vec<&str> = line.split_whitespace().collect();
            let builds = words.first() == Some(&"cargo")
                && words
                    .get(1)
                    .is_some_and(|sub| ["build", "check", "clippy", "test", "doc"].contains(sub));
            assert!(
                !builds || words.contains(&"--locked"),
                "`{line}` must use --locked, so CI builds what Cargo.lock says"
            );
        }
    }
}
