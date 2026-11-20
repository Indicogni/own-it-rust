//! LESSON 01 — Cargo in depth: workspaces, features, profiles, build scripts
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l01_
//! Read first: CONCEPTS.md, "Lesson 01 · Cargo in depth".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: two crates in one workspace ask for
//! different features of the same dependency. How many times is it compiled?
//! IMPLEMENT: this lesson has no Rust bodies to fill in. You repair a real workspace,
//! `fixtures/l01_ws/`, by editing its manifests, its build script and two `cfg`
//! attributes. The tasks below say what must be true afterwards; the tests run
//! `cargo metadata`, `cargo check`, `cargo tree` and `cargo run` on the workspace.
//! EXPLAIN afterward: Explain feature unification to a teammate who asks "why does
//! my crate build alone but behave differently in the workspace build?".
//! JUDGMENT: Inherit what must stay the same in every member (version, edition,
//! licence, lints, dependency versions). Keep per-crate what is really per-crate
//! (name, description, features).
//!
//! The workspace has four crates in `crates/` and an automation crate in `xtask/`:
//!
//! ```text
//! fixtures/l01_ws/
//!   Cargo.toml              the workspace root (a "virtual manifest": no [package])
//!   crates/meter/           library; optional JSON output; a build script
//!   crates/meter-cli/       binary that uses `meter`
//!   crates/tinyjson/        stand-in for a JSON crate
//!   crates/winconsole/      stand-in for a Windows-only helper crate
//!   xtask/                  automation written in Rust (not yet a member)
//! ```
//!
//! TASKS (edit only files under `fixtures/l01_ws/`):
//!
//! ```text
//! A. The workspace loads without any `warning:` from Cargo. Set the resolver that
//!    edition 2024 expects. Move `version` (0.3.0), `edition`, `license`,
//!    `rust-version` (1.99) and `publish` into `[workspace.package]`, and make every
//!    member inherit all five.
//! B. Declare `meter`, `tinyjson` and `winconsole` once, in
//!    `[workspace.dependencies]`. Members refer to them with `workspace = true`.
//! C. Put `unsafe_code = "forbid"` in `[workspace.lints.rust]`, and make every member
//!    inherit the workspace lints.
//! D. `meter` gets a feature named `json` that switches on the optional dependency
//!    `tinyjson`. There must be no feature named `tinyjson`. `meter-cli` gets its own
//!    feature `json` (on by default) that forwards to `meter/json`. Without it,
//!    `meter-cli --caps` prints `json: off` and still builds.
//! E. `winconsole` is a dependency of `meter-cli` only when the target is Windows.
//! F. The release profile (`lto = "thin"`, `codegen-units = 1`) lives where Cargo
//!    reads it.
//! G. The build script of `meter` declares the cfg it sets, and reruns when the
//!    environment variable `METER_FAST` changes. Use the `cargo::` directive syntax.
//! H. `xtask` becomes a workspace member, but not a default member. Add the Cargo
//!    alias that makes `cargo xtask ci --dry-run` work (`.cargo/config.toml`).
//! ```

#[cfg(test)]
mod tests {
    use crate::guess;
    use crate::support::{self, Value, cargo, cargo_with, fixture, list_at, text_at};
    use std::path::PathBuf;

    const MEMBERS: [&str; 5] = [
        "crates/meter",
        "crates/meter-cli",
        "crates/tinyjson",
        "crates/winconsole",
        "xtask",
    ];

    fn ws() -> PathBuf {
        fixture("l01_ws")
    }

    fn root_manifest() -> Value {
        support::toml(&ws().join("Cargo.toml"))
    }

    fn member_manifest(member: &str) -> Value {
        support::toml(&ws().join(member).join("Cargo.toml"))
    }

    fn metadata() -> Value {
        let run = cargo(&ws(), &["metadata", "--format-version", "1", "--no-deps"]);
        run.expect_ok("`cargo metadata` must be able to load the workspace");
        serde_json::from_str(&run.stdout).expect("cargo metadata prints JSON")
    }

    fn package<'a>(metadata: &'a Value, name: &str) -> &'a Value {
        metadata["packages"]
            .as_array()
            .and_then(|packages| packages.iter().find(|p| p["name"] == name))
            .unwrap_or_else(|| panic!("the workspace has no member named `{name}`"))
    }

    #[test]
    fn a_workspace_loads_without_cargo_warnings() {
        let run = cargo(&ws(), &["metadata", "--format-version", "1", "--no-deps"]);
        run.expect_ok("`cargo metadata` must be able to load the workspace");
        let warnings = support::cargo_warnings(&run);
        assert!(
            warnings.is_empty(),
            "Cargo still warns about the workspace. Each warning names its fix:\n{}",
            run.stderr
        );
    }

    #[test]
    fn a_package_fields_are_inherited() {
        let root = root_manifest();
        assert_eq!(
            text_at(&root, "/workspace/resolver"),
            Some("3"),
            "a virtual workspace has no edition, so it must name the resolver itself"
        );
        for (key, expected) in [
            ("version", "0.3.0"),
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
        assert_eq!(
            root.pointer("/workspace/package/publish"),
            Some(&Value::Bool(false)),
            "[workspace.package] must set `publish = false`"
        );
        for member in MEMBERS {
            let manifest = member_manifest(member);
            for key in ["version", "edition", "license", "rust-version", "publish"] {
                assert_eq!(
                    manifest.pointer(&format!("/package/{key}/workspace")),
                    Some(&Value::Bool(true)),
                    "{member}/Cargo.toml must inherit `{key}` with `{key}.workspace = true`"
                );
            }
        }
        let metadata = metadata();
        for name in ["meter", "meter-cli", "tinyjson", "winconsole", "xtask"] {
            assert_eq!(
                package(&metadata, name)["version"],
                "0.3.0",
                "after inheritance every member has the workspace version"
            );
        }
    }

    #[test]
    fn b_dependencies_are_declared_once() {
        let root = root_manifest();
        for (name, path) in [
            ("meter", "crates/meter"),
            ("tinyjson", "crates/tinyjson"),
            ("winconsole", "crates/winconsole"),
        ] {
            assert_eq!(
                text_at(&root, &format!("/workspace/dependencies/{name}/path")),
                Some(path),
                "[workspace.dependencies] must declare `{name}` with its path"
            );
        }
        let meter = member_manifest("crates/meter");
        assert_eq!(
            meter.pointer("/dependencies/tinyjson/workspace"),
            Some(&Value::Bool(true)),
            "meter must take `tinyjson` from the workspace table"
        );
        assert_eq!(
            meter.pointer("/dependencies/tinyjson/optional"),
            Some(&Value::Bool(true)),
            "`optional = true` stays in the member: it is this crate's decision"
        );
        let cli = member_manifest("crates/meter-cli");
        assert_eq!(
            cli.pointer("/dependencies/meter/workspace"),
            Some(&Value::Bool(true)),
            "meter-cli must take `meter` from the workspace table"
        );
        for member in MEMBERS {
            let text = support::read(&ws().join(member).join("Cargo.toml"));
            assert!(
                !text.contains("path ="),
                "{member}/Cargo.toml still has a `path = ...` dependency; \
                 paths belong in [workspace.dependencies] only"
            );
        }
    }

    #[test]
    fn c_lints_are_inherited() {
        let root = root_manifest();
        assert_eq!(
            text_at(&root, "/workspace/lints/rust/unsafe_code"),
            Some("forbid"),
            "[workspace.lints.rust] must forbid unsafe code"
        );
        for member in MEMBERS {
            assert_eq!(
                member_manifest(member).pointer("/lints/workspace"),
                Some(&Value::Bool(true)),
                "{member}/Cargo.toml needs `[lints] workspace = true`; \
                 lints are not inherited unless a member asks for them"
            );
        }
    }

    #[test]
    fn d_optional_dependency_has_a_named_feature() {
        let metadata = metadata();
        let features = &package(&metadata, "meter")["features"];
        assert_eq!(
            list_at(features, "/json"),
            ["dep:tinyjson"],
            "meter's feature `json` must enable the optional dependency with `dep:tinyjson`"
        );
        assert!(
            features.get("tinyjson").is_none(),
            "meter still has an implicit feature named `tinyjson`; \
             `dep:tinyjson` in another feature removes it"
        );
        assert_eq!(
            list_at(features, "/default"),
            [] as [&str; 0],
            "a library should not turn on optional output formats by default"
        );
        let cli = &package(&metadata, "meter-cli")["features"];
        assert_eq!(
            list_at(cli, "/json"),
            ["meter/json"],
            "meter-cli's feature `json` must forward to `meter/json`"
        );
        assert_eq!(
            list_at(cli, "/default"),
            ["json"],
            "meter-cli enables `json` by default"
        );
    }

    #[test]
    fn d_every_feature_combination_builds_without_warnings() {
        let _turn = support::take_turn();
        for flags in [
            &["--workspace", "--all-targets", "--all-features"][..],
            &["--workspace", "--all-targets", "--no-default-features"][..],
            &["-p", "meter", "--features", "json"][..],
        ] {
            let mut args = vec!["check", "--message-format=json"];
            args.extend_from_slice(flags);
            let run = cargo(&ws(), &args);
            run.expect_ok("the workspace must build with every feature combination");
            let warnings = run.diagnostics("warning");
            assert!(
                warnings.is_empty(),
                "`cargo {}` must be free of compiler warnings, found: {:?}\n{}",
                args.join(" "),
                support::codes(&warnings),
                warnings[0].rendered
            );
        }
    }

    #[test]
    fn d_features_change_the_binary() {
        let _turn = support::take_turn();
        let run = cargo(
            &ws(),
            &["run", "-q", "-p", "meter-cli", "--", "--json", "2s"],
        );
        run.expect_ok("meter-cli must run with its default features");
        assert_eq!(run.stdout.trim(), r#"{"millis":2000}"#);

        let run = cargo(
            &ws(),
            &[
                "run",
                "-q",
                "-p",
                "meter-cli",
                "--no-default-features",
                "--",
                "--caps",
            ],
        );
        run.expect_ok("meter-cli must also build without its default features");
        assert!(
            run.stdout.contains("json: off"),
            "without the `json` feature, `--caps` must print `json: off`; got:\n{}",
            run.stdout
        );
    }

    #[test]
    fn e_windows_helper_is_target_specific() {
        let metadata = metadata();
        let dependencies = package(&metadata, "meter-cli")["dependencies"]
            .as_array()
            .expect("dependencies is a list");
        let winconsole = dependencies
            .iter()
            .find(|d| d["name"] == "winconsole")
            .expect("meter-cli must still depend on winconsole (for Windows)");
        assert_eq!(
            winconsole["target"], "cfg(windows)",
            "winconsole must sit under [target.'cfg(windows)'.dependencies]"
        );
        // Technique: the dependency graph for a Linux target must not contain it at all.
        let linux = cargo(
            &ws(),
            &[
                "tree",
                "-p",
                "meter-cli",
                "--target",
                "x86_64-unknown-linux-gnu",
            ],
        );
        linux.expect_ok("`cargo tree --target` works without installing the target");
        assert!(
            !linux.stdout.contains("winconsole"),
            "a Linux build must not even resolve winconsole:\n{}",
            linux.stdout
        );
        let windows = cargo(
            &ws(),
            &[
                "tree",
                "-p",
                "meter-cli",
                "--target",
                "x86_64-pc-windows-msvc",
            ],
        );
        assert!(
            windows.stdout.contains("winconsole"),
            "a Windows build must still get winconsole:\n{}",
            windows.stdout
        );
    }

    #[test]
    fn f_release_profile_is_in_the_workspace_root() {
        let root = root_manifest();
        assert_eq!(
            text_at(&root, "/profile/release/lto"),
            Some("thin"),
            "[profile.release] in the root manifest must set `lto = \"thin\"`"
        );
        assert_eq!(
            root.pointer("/profile/release/codegen-units")
                .and_then(Value::as_i64),
            Some(1),
            "[profile.release] in the root manifest must set `codegen-units = 1`"
        );
        for member in MEMBERS {
            assert!(
                member_manifest(member).get("profile").is_none(),
                "{member}/Cargo.toml has a [profile] table. Cargo ignores profiles \
                 outside the workspace root, so that table does nothing."
            );
        }
    }

    #[test]
    fn g_build_script_declares_its_cfg_and_its_inputs() {
        let script = support::read(&ws().join("crates/meter/build.rs"));
        assert!(
            !script.contains("\"cargo:r"),
            "build.rs still uses the old one-colon `cargo:` syntax; use `cargo::`"
        );
        assert!(
            script.contains("cargo::rustc-check-cfg=cfg(has_fast_path)"),
            "build.rs must declare the cfg it may set, with `cargo::rustc-check-cfg`"
        );

        // Technique: build with the variable, without it, and with it again. A build
        // script that does not name its inputs is not rerun, and the second or third
        // build silently keeps the old answer.
        let _turn = support::take_turn();
        let caps = |fast: &str| {
            let run = cargo_with(
                &ws(),
                &["run", "-q", "-p", "meter-cli", "--", "--caps"],
                &[("METER_FAST", fast)],
            );
            run.expect_ok("meter-cli must build");
            run.stdout
        };
        assert!(
            caps("1").contains("fast-path: on"),
            "with METER_FAST set, the build script must switch the fast path on"
        );
        assert!(
            caps("").contains("fast-path: off"),
            "after METER_FAST was removed, the build still says `on`: \
             the build script did not rerun (cargo::rerun-if-env-changed)"
        );
        assert!(
            caps("1").contains("fast-path: on"),
            "after METER_FAST came back, the build still says `off`: \
             the build script did not rerun (cargo::rerun-if-env-changed)"
        );
    }

    #[test]
    fn h_xtask_is_a_member_with_an_alias() {
        let metadata = metadata();
        let id_of = |name: &str| package(&metadata, name)["id"].clone();
        let xtask = id_of("xtask");
        let defaults = metadata["workspace_default_members"]
            .as_array()
            .expect("default members is a list");
        assert!(
            !defaults.contains(&xtask),
            "xtask must not be a default member: plain `cargo build` should skip it"
        );
        for name in ["meter", "meter-cli"] {
            assert!(
                defaults.contains(&id_of(name)),
                "`{name}` must stay a default member"
            );
        }
        let _turn = support::take_turn();
        let run = cargo(&ws(), &["xtask", "ci", "--dry-run"]);
        run.expect_ok("`cargo xtask ci --dry-run` must work: add an [alias] in .cargo/config.toml");
        let steps: Vec<&str> = run.stdout.lines().collect();
        assert_eq!(
            steps.first().copied(),
            Some("cargo fmt --all --check"),
            "the dry run prints the commands CI would run"
        );
        assert_eq!(steps.len(), 3, "the xtask plans three CI steps");
    }

    /// The `shared` feature list that a built binary of `fixtures/l01_unify` reports.
    fn reported(output: &str, label: &str) -> String {
        // Output looks like `app: fast / build script: codegen`.
        output
            .split(" / ")
            .find_map(|part| part.trim().strip_prefix(label))
            .unwrap_or_else(|| panic!("no `{label}` in output {output:?}"))
            .trim()
            .to_owned()
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// Read `fixtures/l01_unify/` first: three small crates. `shared` has the
    /// features `codegen`, `fast` and `safe`, and `shared::features()` reports the
    /// ones it was compiled with, in that order, joined with `+` (for example
    /// `"fast+safe"`; `"none"` when there are none). `app` asks for `fast` as a normal
    /// dependency and for `codegen` as a build dependency. `tool` asks for `safe`.
    #[test]
    fn p_predictions() {
        let unify = fixture("l01_unify");
        let _turn = support::take_turn();

        // P1. `cargo run -p app` builds only `app` and what it needs.
        // Which features does `app` see in `shared`?
        let alone = cargo(&unify, &["run", "-q", "-p", "app"]);
        alone.expect_ok("the unify fixture must build");
        let p1: &str = guess();
        assert_eq!(
            p1,
            reported(&alone.stdout, "app:"),
            "P1: built alone, a crate gets exactly the features it asked for"
        );

        // P2. In the same run: which features did `shared` have inside app's build
        // script? (The build script is compiled for the host, as a separate program.)
        let p2: &str = guess();
        assert_eq!(
            p2,
            reported(&alone.stdout, "build script:"),
            "P2: build dependencies are resolved apart from normal dependencies"
        );

        // P3. Now `cargo build --workspace` builds `app` and `tool` in one go, and we
        // run the `app` binary it produced. Which features does `app` see in `shared`?
        cargo(&unify, &["build", "-q", "--workspace"]).expect_ok("the workspace must build");
        let binary = support::nested_target_dir(&unify)
            .join("debug")
            .join(format!("app{}", std::env::consts::EXE_SUFFIX));
        let together = support::run(
            std::process::Command::new(&binary),
            "the app binary from the workspace build".to_owned(),
            std::time::Duration::from_secs(60),
        );
        let p3: &str = guess();
        assert_eq!(
            p3,
            reported(&together.stdout, "app:"),
            "P3: one build, one copy of `shared`, with the union of the requested features"
        );
    }
}
