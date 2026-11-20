//! LESSON 05 — Features that only add, and optional dependencies
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same package).
//!
//! The library of this lesson is a real package: `fixtures/l05/swatch/`, with a
//! manifest. You change its `Cargo.toml` and its `src/lib.rs`. The tests run a
//! second `cargo` on it, on three programs of other people in
//! `fixtures/l05/apps/`, and on a fourth in `fixtures/l05/both/`. You do not
//! change those programs. The first run takes a few seconds.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then read `fixtures/l05/swatch/` and
//! consider: `apps/basic` and `apps/hd` ask for different features of `swatch`.
//! When both are built by one `cargo` command, how many copies of `swatch` are
//! compiled, and with which features?
//!
//! IMPLEMENT the contract below. Keep the supplied tests; as your edge case, add
//! a test to one of the apps.
//! EXPLAIN afterward: `swatch` 0.3.0 has three different feature mistakes. For
//! each one, describe the program that it breaks, and say why the author of
//! `swatch` could run `cargo test` every day and never see it.
//!
//! # The contract
//!
//! | Feature | In `default` | What it adds                                             |
//! | ------- | ------------ | -------------------------------------------------------- |
//! | `names` | yes          | `Color::named`                                           |
//! | `json`  | no           | `Color::to_json`; turns on the optional dependency `tinyjson` |
//! | `wide`  | no           | the type `Color16`                                       |
//!
//! - `swatch` has these three features and `default`, and no others. There is
//!   no feature with the name of the dependency, and no feature that takes
//!   something away.
//! - `Color` is the same in every configuration: three public `u8` fields,
//!   `new`, and `hex`, which gives `#rrggbb`.
//! - `Color16` has three public `u16` fields `r`, `g`, `b`, the constructor
//!   `Color16::new(r, g, b)`, and `hex`, which gives `#rrrrggggbbbb`. It derives
//!   what `Color` derives. `Color16::from(color)` widens a `Color` by repeating
//!   every byte: `0x80` becomes `0x8080`.
//! - When `wide` and `json` are both on, `Color16` also has `to_json`, with the
//!   same form as `Color::to_json`.
//! - `swatch` compiles with every one of the eight combinations of the three
//!   features.
//! - The version in the manifest is the smallest one that is correct for this
//!   release. `swatch` is at 0.3.0, and the release takes two feature names
//!   away (`no-names` and one that you did not write) and changes what `wide`
//!   means. Lesson 01 has the rule.
//!
//! Useful commands, from `fixtures/l05/swatch/`:
//!
//! ```text
//! cargo check --no-default-features --features json,wide
//! cargo tree --manifest-path ../apps/Cargo.toml -e features -i swatch
//! ```

#[cfg(test)]
mod tests {
    use crate::support::{self, CargoRun};
    use std::collections::BTreeMap;

    /// Runs `cargo ARGS` in the package `swatch`.
    fn swatch(args: &[&str]) -> CargoRun {
        support::cargo("fixtures/l05/swatch", "l05", args, &[])
    }

    /// Runs `cargo ARGS` in the workspace of the three apps.
    fn apps(args: &[&str]) -> CargoRun {
        support::cargo("fixtures/l05/apps", "l05", args, &[])
    }

    /// The feature table in the output of `cargo metadata`: each feature with
    /// the list of what it turns on.
    fn feature_table(metadata: &str) -> BTreeMap<String, Vec<String>> {
        let start = metadata
            .find("\"features\":{")
            .expect("cargo metadata prints the features of the package")
            + "\"features\":{".len();
        let end = start
            + metadata[start..]
                .find('}')
                .expect("the feature table is closed");
        let unquote = |text: &str| text.trim().trim_matches('"').to_string();
        let mut table = BTreeMap::new();
        // Entries look like `"json":["dep:tinyjson"]`, separated by commas.
        for entry in metadata[start..end].split(']') {
            if let Some((name, list)) = entry.trim_start_matches(',').split_once(":[") {
                let list = list
                    .split(',')
                    .map(unquote)
                    .filter(|item| !item.is_empty())
                    .collect();
                table.insert(unquote(name), list);
            }
        }
        table
    }

    #[test]
    fn a_feature_tables_are_read_from_cargo_metadata() {
        let table = feature_table(r#"{"name":"x","features":{"a":[],"b":["a","dep:y"]},"id":1}"#);
        assert_eq!(table.len(), 2);
        assert!(table["a"].is_empty());
        assert_eq!(table["b"], ["a", "dep:y"]);
    }

    #[test]
    fn a_the_features_are_the_ones_of_the_contract() {
        let run = swatch(&["metadata", "--no-deps", "--format-version", "1"]);
        assert!(
            run.ok,
            "cargo cannot read fixtures/l05/swatch/Cargo.toml:\n{}",
            support::head(&run.stderr, 30)
        );
        let table = feature_table(&run.stdout);
        let names: Vec<&str> = table.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            ["default", "json", "names", "wide"],
            "swatch must have exactly these features. A feature `tinyjson` appears by \
             itself when a dependency is optional and no feature mentions it as \
             `dep:tinyjson`; then the name of a dependency is part of your interface"
        );
        assert_eq!(
            table["default"],
            ["names"],
            "`names` must be on by default, so that a feature adds the table of names; \
             a feature that removes something breaks when another package turns it on"
        );
        assert_eq!(
            table["json"],
            ["dep:tinyjson"],
            "`json` must turn on the optional dependency, and nothing else"
        );
        assert!(
            table["names"].is_empty() && table["wide"].is_empty(),
            "`names` and `wide` do not need other features"
        );
    }

    #[test]
    fn b_every_combination_of_features_compiles() {
        let all = ["names", "json", "wide"];
        for mask in 0..8_u8 {
            let chosen: Vec<&str> = (0..3)
                .filter(|bit| mask & (1 << bit) != 0)
                .map(|bit| all[bit])
                .collect();
            let list = chosen.join(",");
            let run = swatch(&[
                "check",
                "--quiet",
                "--no-default-features",
                "--features",
                &list,
            ]);
            assert!(
                run.ok,
                "swatch does not compile with exactly these features: [{list}]. Cargo may \
                 give a package any combination, because it adds up what all its users \
                 ask for:\n{}",
                support::head(&run.stderr, 30)
            );
        }
    }

    #[test]
    fn c_each_app_works_when_it_is_built_alone() {
        for app in ["basic", "exporter", "hd"] {
            let run = apps(&["test", "--quiet", "--package", app]);
            assert!(
                run.ok,
                "fixtures/l05/apps/{app} does not build or pass with the features it asks \
                 for:\n{}",
                support::head(&run.all(), 40)
            );
        }
    }

    /// The test that the author of `swatch` 0.3.0 never ran.
    #[test]
    fn c_the_apps_work_when_they_are_built_together() {
        let run = apps(&["test", "--quiet", "--workspace"]);
        assert!(
            run.ok,
            "The three apps do not build or pass in one `cargo test --workspace`. In that \
             build Cargo compiles swatch once, with the features of all three apps \
             (names, json and wide). A feature that changes or removes an item then \
             changes it for an app that never asked:\n{}",
            support::head(&run.all(), 40)
        );
    }

    #[test]
    fn d_a_feature_that_is_off_adds_nothing() {
        let run = support::cargo("fixtures/l05/probe", "l05", &["check", "--quiet"], &[]);
        assert!(
            !run.ok,
            "fixtures/l05/probe turns all features of swatch off and still compiles. \
             `Color::named`, `to_json` and `Color16` must each exist only behind its feature"
        );
        for (item, feature) in [("named", "names"), ("to_json", "json"), ("Color16", "wide")] {
            assert!(
                run.stderr.contains(&format!("`{item}`")),
                "`{item}` exists although the feature `{feature}` is off. Put \
                 `#[cfg(feature = \"{feature}\")]` on it. Cargo said:\n{}",
                support::head(&run.stderr, 40)
            );
        }
    }

    /// The fourth user: alone in its build, with two features and no default.
    #[test]
    fn d_two_features_together_add_one_more_item() {
        let run = support::cargo("fixtures/l05/both", "l05", &["test", "--quiet"], &[]);
        assert!(
            run.ok,
            "fixtures/l05/both asks for `wide` and `json`, and for nothing else, and does \
             not build or pass. If Cargo cannot find a feature, repair the manifest of \
             swatch first. If `to_json` is missing: with both features on, `Color16` has a \
             `to_json` of its own, a method with `#[cfg(feature = \"json\")]` inside the \
             `impl` block of `Color16`:\n{}",
            support::head(&run.all(), 40)
        );
    }

    #[test]
    fn e_the_version_tells_cargo_what_kind_of_release_this_is() {
        let run = swatch(&["metadata", "--no-deps", "--format-version", "1"]);
        assert!(
            run.ok,
            "cargo cannot read fixtures/l05/swatch/Cargo.toml:\n{}",
            support::head(&run.stderr, 30)
        );
        let version = support::json_string(&run.stdout, "version").unwrap_or_default();
        support::assert_answer(
            "swatch version",
            &version,
            0x6a9d_6eac_0695_11c9,
            "A manifest that says `features = [\"no-names\"]` no longer resolves, and \
             `wide` no longer changes `Color`. Which class of change is the removal of \
             a feature (lesson 01)? Which of the three numbers is the incompatible one \
             for a library at 0.3.0?",
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// Read the workspace `fixtures/l05/demo/` first: the library `lamp` gives
    /// 900 lumens with its feature `bright` and 400 without. The program
    /// `reader` uses `lamp` without features, the program `stage` asks for
    /// `bright`. Both print `lamp::lumens()`.
    #[test]
    fn p_predictions() {
        let demo = |args: &[&str]| support::cargo("fixtures/l05/demo", "l05-demo", args, &[]);
        let number = |run: &CargoRun| -> u32 {
            assert!(run.ok, "the demo workspace must build:\n{}", run.stderr);
            run.stdout
                .trim()
                .parse()
                .expect("the program prints one number")
        };

        // P1. `cargo run --package reader`: only `reader` is built. What does it
        // print?
        let p1: u32 = crate::guess();
        let alone = demo(&["run", "--quiet", "--package", "reader"]);
        assert_eq!(
            p1,
            number(&alone),
            "P1: nobody in this build asked for `bright`"
        );

        // P2. `cargo build --workspace`, and then the program `reader` that this
        // build produced is started. Its source and its manifest are unchanged.
        // What does it print now?
        let p2: u32 = crate::guess();
        let together = demo(&["build", "--quiet", "--workspace"]);
        assert!(
            together.ok,
            "the demo workspace must build:\n{}",
            together.stderr
        );
        let reader = support::cargo_target_dir("l05-demo")
            .join("debug")
            .join(format!("reader{}", std::env::consts::EXE_SUFFIX));
        let output = std::process::Command::new(&reader)
            .output()
            .unwrap_or_else(|e| panic!("cannot start {}: {e}", reader.display()));
        let printed: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .expect("the program prints one number");
        assert_eq!(
            p2, printed,
            "P2: one build has one copy of `lamp`, with the features of everybody in \
             the build. `stage` asked for `bright`, so `reader` got it too"
        );
    }
}
