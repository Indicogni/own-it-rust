//! LESSON 08 — Dependencies and upgrades
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l08_
//!      cargo test --manifest-path mastery/m10_contributing/Cargo.toml --test downstream pinned
//! Do not pass `--locked` in this lesson: the upgrade changes `Cargo.lock`, and that
//! is the point.
//! Read first: CONCEPTS.md, "Lesson 08", then `fixture/issues/0019-bitflags-2.md`.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: The fixture pins its dependencies with
//! `=`. What does `cargo update` do for it? Which part of an upgrade does the
//! compiler check for you, and which part does it not?
//! IMPLEMENT: A. find the duplicated dependency. B. read the dependency's changelog
//! for the major release. C. do the upgrade in the fixture: one version for the
//! whole workspace, the code adapted, the behaviour of tasklane unchanged for its
//! users, and a changelog entry. `downstream/pinned` must still build: it calls what
//! a user of 0.4.0 may call, including things that tasklane itself never calls.
//! There is no `todo!()` for task C; the work is in `fixture/`. Preserve the tests.
//! EXPLAIN afterward: Explain which change the compiler found for you, which change
//! only a test found, and why this upgrade could have been a breaking change for
//! the users of tasklane although tasklane's own source hardly changed.

/// A. Run `cargo tree --manifest-path mastery/m10_contributing/fixture/Cargo.toml
/// --duplicates` (short: `-d`). Which crate is in the tree in two versions?
pub fn duplicated_crate() -> &'static str {
    todo!("Implement this task")
}

/// B. The changelog of that crate, section for the release that starts the new
/// major version. Find it from the crate's page on crates.io ("Repository"), or on
/// docs.rs.
pub mod changelog {
    /// The old API had an `unsafe` method `from_bits_unchecked`. What is the name of
    /// the safe method that replaces it?
    pub fn replaces_from_bits_unchecked() -> &'static str {
        todo!("Implement this task")
    }

    /// "Generated flags types now derive fewer traits." How many traits does the
    /// changelog tell you to derive yourself to stay compatible?
    pub fn traits_to_derive() -> usize {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, list_items, mentions_issue, read, section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;
    use tasklane_core::{TaskFlags, describe_flags};

    #[test]
    fn a_duplicated_crate() {
        assert_answer("A", duplicated_crate(), 0xc095_ea8c_0a34_8203);
    }

    #[test]
    fn b_changelog_facts() {
        assert_answer(
            "B, replaces_from_bits_unchecked",
            changelog::replaces_from_bits_unchecked(),
            0xf643_e8a0_8a81_646a,
        );
        assert_answer(
            "B, traits_to_derive",
            &changelog::traits_to_derive().to_string(),
            0xaf63_b54c_8601_aa47,
        );
    }

    /// The versions of `name` in the lock file of this package. Cargo rewrites that
    /// file before it builds this test, so it always describes what was built.
    fn locked_versions(name: &str) -> Vec<String> {
        let lock = read("Cargo.lock");
        let wanted = format!("name = \"{name}\"");
        let mut versions = Vec::new();
        let mut lines = lock.lines();
        while let Some(line) = lines.next() {
            if line == wanted
                && let Some(version) = lines.next()
            {
                versions.push(
                    version
                        .trim_start_matches("version = ")
                        .trim_matches('"')
                        .to_string(),
                );
            }
        }
        versions
    }

    #[test]
    fn c_one_version_in_the_lock_file() {
        let versions = locked_versions("bitflags");
        assert_eq!(
            versions.len(),
            1,
            "C: the build still contains bitflags in these versions: {versions:?}"
        );
        assert!(
            versions[0].starts_with("2."),
            "C: the workspace must agree on the 2.x line, not on {}",
            versions[0]
        );
    }

    #[test]
    fn c_one_place_decides_the_version() {
        let workspace = read("fixture/Cargo.toml");
        let line = workspace
            .lines()
            .find(|line| line.trim_start().starts_with("bitflags"))
            .expect("C: fixture/Cargo.toml lists bitflags under [workspace.dependencies]");
        assert!(
            line.contains("\"=2."),
            "C: pin the workspace dependency to an exact 2.x version with `=`, as the \
             project does for every dependency. Found: `{line}`"
        );
        let cli = read("fixture/crates/tasklane-cli/Cargo.toml");
        let line = cli
            .lines()
            .find(|line| line.trim_start().starts_with("bitflags"))
            .expect("C: tasklane-cli still depends on bitflags");
        assert!(
            line.contains("workspace = true"),
            "C: tasklane-cli must take the version from the workspace, so that the \
             two can never drift apart again. Found: `{line}`"
        );
    }

    #[test]
    fn c_flags_look_the_same_to_users() {
        assert_eq!(
            describe_flags(TaskFlags::SILENT | TaskFlags::HIDDEN),
            "silent, hidden",
            "C: the text that `tasklane list` shows must not change with the upgrade"
        );
        assert_eq!(describe_flags(TaskFlags::all()), "silent, optional, hidden");
        assert_eq!(describe_flags(TaskFlags::OPTIONAL), "optional");
        assert_eq!(describe_flags(TaskFlags::empty()), "");

        let site = include_str!("../fixture/examples/site.lane");
        let args = ["list".to_string()];
        let (mut out, mut err) = (Vec::new(), Vec::new());
        tasklane_cli::run_with(&args, &|_| Ok(site.to_string()), &mut out, &mut err);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "clean [silent]\n\
             pages - Writes the two pages.\n\
             stats [silent, optional] - Word counts. May fail without failing the build.\n\
             site - Builds build/site.txt.\n"
        );
    }

    #[test]
    fn c_flag_helpers_still_work() {
        assert_eq!((TaskFlags::SILENT | TaskFlags::OPTIONAL).count(), 2);
        assert_eq!(TaskFlags::empty().count(), 0);
        assert_eq!(TaskFlags::from_keyword("hidden"), Some(TaskFlags::HIDDEN));
        assert_eq!(TaskFlags::from_keyword("HIDDEN"), None);
        assert_eq!(TaskFlags::all().bits(), 0b111);
    }

    #[test]
    fn c_changelog_mentions_the_upgrade() {
        let changelog = read("fixture/CHANGELOG.md");
        let unreleased = section(&changelog, "## [Unreleased]")
            .expect("fixture/CHANGELOG.md has a `## [Unreleased]` section");
        let entries = list_items(unreleased);
        let entry = entries
            .iter()
            .find(|entry| entry.to_lowercase().contains("bitflags"))
            .unwrap_or_else(|| {
                panic!(
                    "C: users of the library see the bitflags version in their own \
                     dependency tree. Add a line under `### Changed` in `## [Unreleased]`."
                )
            });
        assert!(mentions_issue(entry, 19), "C: reference the issue: `(#19)`");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `tasklane_cli::args::Show` is declared with bitflags 2 already, so it shows
    /// what the upgrade will do to `TaskFlags`. Read the bitflags 2 documentation on
    /// docs.rs first.
    #[test]
    fn p_predictions() {
        use tasklane_cli::args::Show;

        // P1. format!("{:?}", Show::LOG | Show::SUMMARY)
        let p1: &str = guess();
        assert_eq!(
            p1,
            format!("{:?}", Show::LOG | Show::SUMMARY),
            "P1: the derived Debug output of a bitflags 2 type"
        );

        // P2. format!("{:?}", Show::empty())
        let p2: &str = guess();
        assert_eq!(
            p2,
            format!("{:?}", Show::empty()),
            "P2: the Debug output of an empty set of flags"
        );

        // P3. bitflags 2 gives every flags type a method `from_name`.
        //     (Show::from_name("LOG").is_some(), Show::from_name("log").is_some())
        let p3: (bool, bool) = guess();
        assert_eq!(
            p3,
            (
                Show::from_name("LOG").is_some(),
                Show::from_name("log").is_some()
            ),
            "P3: `from_name` takes the name of the constant, exactly as written"
        );
    }
}
