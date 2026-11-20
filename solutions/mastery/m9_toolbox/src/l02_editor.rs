//! LESSON 02 — Toolchains, the editor, and reading generated code
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l02_
//! Read first: CONCEPTS.md, "Lesson 02 · Toolchains, the editor, and generated code".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a repository has a
//! `rust-toolchain.toml`, and you type `cargo +nightly test`. Which toolchain runs?
//! IMPLEMENT `active_toolchain`, then edit the two files in `fixtures/l02_setup/`.
//! Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain what rust-analyzer needs from a project before it can
//! show types, and why a team commits `rust-toolchain.toml`.
//! JUDGMENT: Commit the settings that must be the same for everyone (toolchain,
//! components, lint command). Leave personal taste (inlay hint style, key bindings) to
//! each person.

/// Why rustup picked a toolchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `cargo +nightly build`: a `+name` argument on the command line.
    CommandLine,
    /// The environment variable `RUSTUP_TOOLCHAIN`.
    Environment,
    /// `rustup override set <toolchain>` was run for this directory.
    DirectoryOverride(String),
    /// A `rust-toolchain.toml` in this directory.
    ToolchainFile(String),
    /// Nothing else applied: the default toolchain (`rustup default`).
    Default,
}

/// Everything rustup looks at when you run `cargo`, `rustc` or another proxy.
///
/// Directories are absolute paths with `/` separators and no trailing slash, for
/// example `/work/app/crates/core`. The root directory is `/`.
#[derive(Debug, Clone, Default)]
pub struct Situation<'a> {
    /// `Some("nightly")` for `cargo +nightly ...`.
    pub plus: Option<&'a str>,
    /// The value of `RUSTUP_TOOLCHAIN`, if set.
    pub env: Option<&'a str>,
    /// The directory the command runs in.
    pub cwd: &'a str,
    /// Directory overrides: `(directory, toolchain)`.
    pub overrides: &'a [(&'a str, &'a str)],
    /// `rust-toolchain.toml` files: `(directory that holds the file, channel)`.
    pub files: &'a [(&'a str, &'a str)],
    /// The default toolchain.
    pub default: &'a str,
}

/// A. The toolchain rustup selects, and why.
///
/// The rules, as rustup 1.29 applies them:
///
/// 1. A `+toolchain` argument wins over everything.
/// 2. Otherwise `RUSTUP_TOOLCHAIN` wins.
/// 3. Otherwise rustup walks from `cwd` up to `/`. In each directory it looks first
///    for a directory override, then for a `rust-toolchain.toml`. The first hit wins,
///    so a file in a nearer directory beats an override further up.
/// 4. Otherwise the default toolchain is used.
///
/// Directories match as whole path components: `/work/app` is not an ancestor of
/// `/work/app2`.
pub fn active_toolchain(situation: &Situation) -> (String, Source) {
    if let Some(name) = situation.plus {
        return (name.to_owned(), Source::CommandLine);
    }
    if let Some(name) = situation.env {
        return (name.to_owned(), Source::Environment);
    }
    let mut dir = situation.cwd;
    loop {
        if let Some((_, name)) = situation.overrides.iter().find(|(d, _)| *d == dir) {
            return (
                (*name).to_owned(),
                Source::DirectoryOverride(dir.to_owned()),
            );
        }
        if let Some((_, name)) = situation.files.iter().find(|(d, _)| *d == dir) {
            return ((*name).to_owned(), Source::ToolchainFile(dir.to_owned()));
        }
        // Step to the parent: "/work/app" -> "/work" -> "/" -> stop.
        dir = match dir.rfind('/') {
            Some(0) if dir.len() > 1 => "/",
            Some(0) | None => break,
            Some(cut) => &dir[..cut],
        };
    }
    (situation.default.to_owned(), Source::Default)
}

// B. Project setup files. Edit `fixtures/l02_setup/rust-toolchain.toml` so that:
//
//    - the channel is the exact release `1.99.0`, not a moving name such as `stable`;
//    - the profile is `minimal`;
//    - the components are rustfmt, clippy, and the one rust-analyzer needs to read
//      the standard library's source.
//
// C. Edit `fixtures/l02_setup/editor/settings.json` (rust-analyzer settings) so that:
//
//    - saving a file runs Clippy instead of plain `cargo check`;
//    - rust-analyzer analyses the code behind every Cargo feature;
//    - rust-analyzer builds into its own subdirectory of `target/`, so it never
//      blocks your terminal's `cargo build` on the build-directory lock.
//
//    CONCEPTS.md names the three settings.

/// The fixture that `fixtures/l02_expand/expanded.rs` was generated from.
#[path = "../fixtures/l02_expand/src/lib.rs"]
pub mod expand_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::{self, Value, fixture, list_at, text_at};

    fn base<'a>() -> Situation<'a> {
        Situation {
            cwd: "/work/app/crates/core",
            default: "stable",
            ..Situation::default()
        }
    }

    fn name(situation: &Situation) -> String {
        active_toolchain(situation).0
    }

    #[test]
    fn a_default_when_nothing_else_applies() {
        crate::support::within(20, "a_default_when_nothing_else_applies", || {
            assert_eq!(
                active_toolchain(&base()),
                ("stable".to_owned(), Source::Default)
            );
            let unrelated = Situation {
                files: &[("/work/other", "1.80.0")],
                overrides: &[("/home/me", "beta")],
                ..base()
            };
            assert_eq!(
                active_toolchain(&unrelated),
                ("stable".to_owned(), Source::Default),
                "files and overrides outside the ancestors of cwd do not apply"
            );
        });
    }

    #[test]
    fn a_toolchain_file_in_any_ancestor() {
        crate::support::within(20, "a_toolchain_file_in_any_ancestor", || {
            let in_cwd = Situation {
                files: &[("/work/app/crates/core", "1.99.0")],
                ..base()
            };
            assert_eq!(
                active_toolchain(&in_cwd),
                (
                    "1.99.0".to_owned(),
                    Source::ToolchainFile("/work/app/crates/core".to_owned())
                )
            );
            let in_repo_root = Situation {
                files: &[("/work/app", "1.99.0")],
                ..base()
            };
            assert_eq!(
                active_toolchain(&in_repo_root),
                (
                    "1.99.0".to_owned(),
                    Source::ToolchainFile("/work/app".to_owned())
                ),
                "the file applies to every directory below it"
            );
            let at_root = Situation {
                files: &[("/", "1.70.0")],
                ..base()
            };
            assert_eq!(
                name(&at_root),
                "1.70.0",
                "the walk must reach the root directory"
            );
            let nearest = Situation {
                files: &[("/work", "1.70.0"), ("/work/app/crates", "1.99.0")],
                ..base()
            };
            assert_eq!(name(&nearest), "1.99.0", "the nearest file wins");
        });
    }

    #[test]
    fn a_command_line_and_environment_win() {
        crate::support::within(20, "a_command_line_and_environment_win", || {
            let all = Situation {
                plus: Some("nightly"),
                env: Some("beta"),
                files: &[("/work/app", "1.99.0")],
                overrides: &[("/work/app", "1.96.0")],
                ..base()
            };
            assert_eq!(
                active_toolchain(&all),
                ("nightly".to_owned(), Source::CommandLine),
                "`+toolchain` beats everything"
            );
            let env = Situation { plus: None, ..all };
            assert_eq!(
                active_toolchain(&env),
                ("beta".to_owned(), Source::Environment),
                "RUSTUP_TOOLCHAIN beats overrides and files"
            );
        });
    }

    #[test]
    fn a_nearest_directory_wins_between_override_and_file() {
        crate::support::within(
            20,
            "a_nearest_directory_wins_between_override_and_file",
            || {
                let same_directory = Situation {
                    files: &[("/work/app", "1.99.0")],
                    overrides: &[("/work/app", "nightly")],
                    ..base()
                };
                assert_eq!(
                    active_toolchain(&same_directory),
                    (
                        "nightly".to_owned(),
                        Source::DirectoryOverride("/work/app".to_owned())
                    ),
                    "in one directory, the override is checked before the file"
                );
                // Technique: "all overrides first, then all files" is the plausible wrong
                // reading of the rule. rustup goes directory by directory.
                let file_is_nearer = Situation {
                    files: &[("/work/app/crates", "1.99.0")],
                    overrides: &[("/work", "nightly")],
                    ..base()
                };
                assert_eq!(
                    active_toolchain(&file_is_nearer),
                    (
                        "1.99.0".to_owned(),
                        Source::ToolchainFile("/work/app/crates".to_owned())
                    ),
                    "a toolchain file in a nearer directory beats an override further up"
                );
            },
        );
    }

    #[test]
    fn a_directories_match_by_component_not_by_prefix() {
        crate::support::within(20, "a_directories_match_by_component_not_by_prefix", || {
            let sibling = Situation {
                cwd: "/work/app2/src",
                files: &[("/work/app", "1.70.0")],
                overrides: &[("/work/ap", "beta")],
                ..base()
            };
            assert_eq!(
                active_toolchain(&sibling),
                ("stable".to_owned(), Source::Default),
                "`/work/app` is not an ancestor of `/work/app2/src`"
            );
            let root_cwd = Situation {
                cwd: "/",
                files: &[("/", "1.70.0")],
                ..base()
            };
            assert_eq!(name(&root_cwd), "1.70.0", "cwd may be the root itself");
        });
    }

    #[test]
    fn b_toolchain_file_pins_version_and_components() {
        let file = support::toml(&fixture("l02_setup").join("rust-toolchain.toml"));
        assert_eq!(
            text_at(&file, "/toolchain/channel"),
            Some("1.99.0"),
            "pin an exact release; `stable` means a different compiler every six weeks"
        );
        assert_eq!(
            text_at(&file, "/toolchain/profile"),
            Some("minimal"),
            "the minimal profile installs rustc, cargo and std only; list the rest"
        );
        let components = list_at(&file, "/toolchain/components");
        for needed in ["rustfmt", "clippy", "rust-src"] {
            assert!(
                components.iter().any(|c| c == needed),
                "components must include `{needed}`, found {components:?}"
            );
        }
    }

    #[test]
    fn c_editor_settings() {
        let settings = support::json(&fixture("l02_setup").join("editor/settings.json"));
        assert_eq!(
            settings.get("rust-analyzer.check.command"),
            Some(&Value::String("clippy".to_owned())),
            "check on save must run Clippy"
        );
        assert_eq!(
            settings.get("rust-analyzer.cargo.features"),
            Some(&Value::String("all".to_owned())),
            "code behind non-default features is greyed out unless features are \"all\""
        );
        let target_dir = settings.get("rust-analyzer.cargo.targetDir");
        assert!(
            matches!(target_dir, Some(Value::Bool(true) | Value::String(_))),
            "give rust-analyzer its own target directory (true, or a path)"
        );
    }

    struct NotClone;

    /// Answers "is `T: Clone`?" for a concrete type at run time. The inherent method
    /// exists only when `T: Clone`; otherwise the call falls back to the trait method.
    struct Probe<T>(std::marker::PhantomData<T>);

    trait Fallback {
        fn is_clone(&self) -> bool {
            false
        }
    }

    impl<T> Fallback for Probe<T> {}

    impl<T: Clone> Probe<T> {
        fn is_clone(&self) -> bool {
            true
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// Read `fixtures/l02_expand/src/lib.rs` (the source) and
    /// `fixtures/l02_expand/expanded.rs` (what `cargo expand` printed for it).
    #[test]
    fn p_predictions() {
        use super::expand_fixture::{Release, Shared};

        // P1. `Release` derives `PartialOrd`. Read the generated `partial_cmp`.
        // Is release 1.9 (major 1, patch 9) less than release 2.1 (major 2, patch 1)?
        let older = Release { major: 1, patch: 9 };
        let newer = Release { major: 2, patch: 1 };
        let p1: bool = false;
        assert_eq!(
            p1,
            older < newer,
            "P1: a derived comparison goes through the fields in declaration order"
        );

        // P2. `Shared<T>` derives `Clone` and holds an `Rc<T>`. `Rc<NotClone>` is
        // `Clone`. Read the generated `impl`: is `Shared<NotClone>` `Clone`?
        let _value = Shared {
            inner: std::rc::Rc::new(NotClone),
        };
        assert!(Probe::<std::rc::Rc<NotClone>>(std::marker::PhantomData).is_clone());
        let p2: bool = false;
        assert_eq!(
            p2,
            Probe::<Shared<NotClone>>(std::marker::PhantomData).is_clone(),
            "P2: a derive puts its bound on the type parameter, not on the field types"
        );

        // P3. `expanded.rs` shows the body of `bonus_demo` after expansion.
        // What does `bonus_demo()` return?
        let p3: i32 = 101;
        assert_eq!(
            p3,
            super::expand_fixture::bonus_demo(),
            "P3: expanded text loses hygiene; the two `x` are different variables"
        );
    }
}
