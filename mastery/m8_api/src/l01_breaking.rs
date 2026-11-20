//! LESSON 01 — Version numbers as promises: which change breaks a user?
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same package).
//!
//! PREDICT before running: this lesson is mostly prediction. The tests `s01_` to
//! `s13_` each apply one change to the library `fixtures/l01/shelf.rs` (version
//! 1.0.0) and compile the program `fixtures/l01/app.rs`, which was written for
//! 1.0.0, against the result. Read both files first. For every change, replace the
//! two `guess()` calls: what happens to the program (`"ok"`, `"panic"`, or the
//! compiler's error code such as `"E0308"`), and which class the Cargo SemVer
//! reference gives the change. The reference lists changes that can make a
//! build fail. It has no entry for S08 (behaviour) and S09 (a lost trait
//! implementation); classify those two with the definition of a major change
//! in CONCEPTS. Then answer `p_predictions`.
//!
//! IMPLEMENT tasks A and B. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Two of the thirteen changes are in a class that does not match
//! what happened to the program: one breaks nothing here and is still "major",
//! one breaks the build and is still not "major". Which are they, and what does
//! that tell you about the difference between "this change broke somebody" and
//! "this change needs a major version"?
//!
//! COMPILER / TRANSFER LAB (optional): copy `fixtures/l01/shelf.rs` and `app.rs`
//! to a scratch directory, apply change S07 by hand, and build the pair with
//! `rustc --edition 2024 --crate-type rlib --crate-name shelf shelf.rs` and
//! `rustc --edition 2024 --extern shelf=libshelf.rlib app.rs`. Then repair
//! `app.rs`. That repair is the work you hand to every user of a major release.
//!
//! Reference: <https://doc.rust-lang.org/cargo/reference/semver.html>

/// A version number `major.minor.patch` without pre-release or build parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }
}

/// How large a release is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    /// The release contains a change that can break users.
    Major,
    /// The release adds something, and breaks nobody.
    Minor,
    /// The release only repairs something.
    Patch,
}

/// A. Whether Cargo may use version `candidate` for a dependency that a manifest
/// declares as `name = "requirement"` (a plain version, also called a caret
/// requirement).
///
/// Cargo treats two versions as compatible when their left-most non-zero number
/// is the same. So `candidate` is allowed when it is not older than
/// `requirement` and is compatible with it:
///
/// - `1.2.3` allows `1.2.3`, `1.2.9` and `1.7.0`, but not `1.2.2` or `2.0.0`;
/// - `0.2.3` allows `0.2.3` and `0.2.9`, but not `0.3.0`;
/// - `0.0.3` allows only `0.0.3`.
pub fn caret_allows(requirement: Version, candidate: Version) -> bool {
    todo!("Implement this task")
}

/// B. The version number of the next release of a library that is now at
/// `current`, for a release of size `bump`.
///
/// - From 1.0.0 on: a major release gives `(major + 1).0.0`, a minor release
///   `major.(minor + 1).0`, a patch release `major.minor.(patch + 1)`.
/// - `0.y.z` with `y > 0`: Cargo reads `y` as the incompatible number. A major
///   release gives `0.(y + 1).0`; minor and patch releases give `0.y.(z + 1)`.
/// - `0.0.z`: every release gives `0.0.(z + 1)`, and Cargo treats every one of
///   them as incompatible.
///
/// The numbers in the tests are small; overflow is not part of the task.
pub fn next_version(current: Version, bump: Bump) -> Version {
    todo!("Implement this task")
}

/// The class that the Cargo SemVer reference gives a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The change requires a major release.
    Major,
    /// The change requires only a minor release.
    Minor,
    /// The change can break some programs, and many projects still accept it
    /// in a minor release.
    PossiblyBreaking,
}

/// One change to `fixtures/l01/shelf.rs`. Every pair in `edits` replaces the
/// first text, which occurs exactly once in the file, by the second.
pub struct Change {
    pub title: &'static str,
    pub edits: &'static [(&'static str, &'static str)],
}

/// C. The thirteen changes. Each one is applied alone to version 1.0.0. You do not
/// edit this table; you predict its effect in the tests `s01_` to `s13_`.
/// The `title` of an entry says what changes. The `edits` are the exact
/// replacements in the text of the library, for when you need the detail.
pub const CHANGES: [Change; 13] = [
    Change {
        title: "S01: a new public function, `sold_out`",
        edits: &[(
            "/// The number of pieces in `items`.\n",
            "/// The names of the items of which nothing is left.\n\
             pub fn sold_out(items: &[Item]) -> Vec<&str> {\n    \
                 items.iter().filter(|item| item.qty == 0).map(|item| item.name.as_str()).collect()\n\
             }\n\n\
             /// The number of pieces in `items`.\n",
        )],
    },
    Change {
        title: "S02: a new public field, `Item::tag`",
        edits: &[
            (
                "    pub qty: u32,\n}",
                "    pub qty: u32,\n    pub tag: Option<String>,\n}",
            ),
            // The library's own code is adjusted, so that the library still compiles.
            (
                "                qty,\n            }),",
                "                qty,\n                tag: None,\n            }),",
            ),
        ],
    },
    Change {
        title: "S03: a new variant, `Level::Reserved`",
        edits: &[("    Plenty,\n}", "    Plenty,\n    Reserved,\n}")],
    },
    Change {
        title: "S04: a new field in the variant `Event::Taken`",
        edits: &[
            (
                "    Taken { name: String, left: u32 },",
                "    Taken { name: String, left: u32, at: u64 },",
            ),
            (
                "            left: item.qty,\n",
                "            left: item.qty,\n            at: 0,\n",
            ),
        ],
    },
    Change {
        title: "S05: a new parameter for `Shelf::take`",
        edits: &[(
            "    pub fn take(&mut self, name: &str, qty: u32) -> Option<Event> {\n        \
                 let item = self.items.iter_mut().find(|item| item.name == name)?;\n        \
                 item.qty = item.qty.checked_sub(qty)?;",
            "    pub fn take(&mut self, name: &str, qty: u32, all_or_nothing: bool) -> Option<Event> {\n        \
                 let item = self.items.iter_mut().find(|item| item.name == name)?;\n        \
                 item.qty = match item.qty.checked_sub(qty) {\n            \
                     Some(left) => left,\n            \
                     None if all_or_nothing => return None,\n            \
                     None => 0,\n        \
                 };",
        )],
    },
    Change {
        title: "S06: `total` accepts anything that yields `&Item`; a slice is still accepted",
        edits: &[(
            "pub fn total(items: &[Item]) -> u32 {\n    items.iter()",
            "pub fn total<'a>(items: impl IntoIterator<Item = &'a Item>) -> u32 {\n    items.into_iter()",
        )],
    },
    Change {
        title: "S07: `Shelf::names` returns an iterator instead of a vector",
        edits: &[(
            "    pub fn names(&self) -> Vec<String> {\n        \
                 let mut names: Vec<String> = self.items.iter().map(|item| item.name.clone()).collect();\n        \
                 names.sort();\n        \
                 names\n",
            "    pub fn names(&self) -> impl Iterator<Item = String> {\n        \
                 let mut names: Vec<String> = self.items.iter().map(|item| item.name.clone()).collect();\n        \
                 names.sort();\n        \
                 names.into_iter()\n",
        )],
    },
    Change {
        title: "S08: `Shelf::names` no longer sorts (its documentation still says it does)",
        edits: &[("        names.sort();\n", "")],
    },
    Change {
        title: "S09: `Level` no longer derives `Copy`",
        edits: &[(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Level",
            "#[derive(Debug, Clone, PartialEq, Eq)]\npub enum Level",
        )],
    },
    Change {
        title: "S10: `Item` becomes `#[non_exhaustive]`",
        edits: &[(
            "#[derive(Debug, Clone, PartialEq)]\npub struct Item",
            "#[derive(Debug, Clone, PartialEq)]\n#[non_exhaustive]\npub struct Item",
        )],
    },
    Change {
        title: "S11: a new private field, `Shelf::changes`",
        edits: &[(
            "    items: Vec<Item>,\n}",
            "    items: Vec<Item>,\n    changes: u64,\n}",
        )],
    },
    Change {
        title: "S12: a new inherent method, `Shelf::count`",
        edits: &[(
            "    /// The items, in the order",
            "    /// The number of different items.\n    \
                 pub fn count(&self) -> usize {\n        \
                     self.items.len()\n    \
                 }\n\n    \
                 /// The items, in the order",
        )],
    },
    Change {
        title: "S13: the method `Shelf::is_empty` is removed (\"nobody we know calls it\")",
        edits: &[(
            "    /// Whether the shelf has no items at all.\n    \
                 pub fn is_empty(&self) -> bool {\n        \
                     self.items.is_empty()\n    \
                 }\n\n",
            "",
        )],
    },
];

/// The text of `shelf.rs` after `change`. Panics when an edit does not apply to
/// exactly one place, which would mean that the fixture was edited.
pub fn apply(version_1: &str, change: &Change) -> String {
    let mut text = version_1.to_string();
    for (old, new) in change.edits {
        assert_eq!(
            text.matches(old).count(),
            1,
            "{}: the text to replace must occur exactly once in fixtures/l01/shelf.rs:\n{old}",
            change.title
        );
        text = text.replacen(old, new, 1);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{self, Build};

    const V: fn(u64, u64, u64) -> Version = Version::new;

    #[test]
    fn a_caret_from_1_0_on_fixes_the_major_number() {
        assert!(caret_allows(V(1, 2, 3), V(1, 2, 3)));
        assert!(caret_allows(V(1, 2, 3), V(1, 2, 9)));
        assert!(caret_allows(V(1, 2, 3), V(1, 7, 0)));
        assert!(
            !caret_allows(V(1, 2, 3), V(1, 2, 2)),
            "a requirement is a lower bound: 1.2.2 is older than 1.2.3"
        );
        assert!(
            !caret_allows(V(1, 2, 3), V(1, 1, 9)),
            "1.1.9 is older than 1.2.3, although its patch number is larger"
        );
        assert!(!caret_allows(V(1, 2, 3), V(2, 0, 0)));
        assert!(!caret_allows(V(1, 2, 3), V(0, 9, 9)));
    }

    #[test]
    fn a_caret_before_1_0_fixes_the_leftmost_nonzero_number() {
        assert!(caret_allows(V(0, 2, 3), V(0, 2, 3)));
        assert!(caret_allows(V(0, 2, 3), V(0, 2, 9)));
        assert!(
            !caret_allows(V(0, 2, 3), V(0, 3, 0)),
            "before 1.0.0 the minor number is the incompatible one"
        );
        assert!(
            !caret_allows(V(0, 2, 3), V(1, 2, 3)),
            "1.2.3 is not compatible with 0.2.3, although minor and patch are equal"
        );
        assert!(caret_allows(V(0, 0, 3), V(0, 0, 3)));
        assert!(
            !caret_allows(V(0, 0, 3), V(0, 0, 4)),
            "0.0.z versions are compatible only with themselves"
        );
        assert!(!caret_allows(V(0, 0, 3), V(0, 1, 0)));
        assert!(caret_allows(V(0, 0, 0), V(0, 0, 0)));
        assert!(!caret_allows(V(0, 0, 0), V(0, 0, 1)));
    }

    #[test]
    fn b_next_version_from_1_0_on() {
        assert_eq!(next_version(V(1, 4, 2), Bump::Major), V(2, 0, 0));
        assert_eq!(next_version(V(1, 4, 2), Bump::Minor), V(1, 5, 0));
        assert_eq!(next_version(V(1, 4, 2), Bump::Patch), V(1, 4, 3));
    }

    #[test]
    fn b_next_version_before_1_0() {
        assert_eq!(next_version(V(0, 4, 2), Bump::Major), V(0, 5, 0));
        assert_eq!(
            next_version(V(0, 4, 2), Bump::Minor),
            V(0, 4, 3),
            "before 1.0.0 an addition goes into the patch number: \
             0.5.0 would tell Cargo that the release is incompatible"
        );
        assert_eq!(next_version(V(0, 4, 2), Bump::Patch), V(0, 4, 3));
        for bump in [Bump::Major, Bump::Minor, Bump::Patch] {
            assert_eq!(next_version(V(0, 0, 7), bump), V(0, 0, 8));
        }
    }

    /// The two functions agree: a user who wrote the current version as the
    /// requirement gets the next release automatically exactly when the
    /// release is not a major one.
    #[test]
    fn b_a_release_reaches_users_exactly_when_it_is_compatible() {
        for major in 0..3 {
            for minor in 0..3 {
                for patch in 0..3 {
                    let current = V(major, minor, patch);
                    assert!(
                        !caret_allows(current, next_version(current, Bump::Major)),
                        "{current:?}: a major release must be outside the caret range"
                    );
                    for bump in [Bump::Minor, Bump::Patch] {
                        let next = next_version(current, bump);
                        assert!(next > current);
                        assert_eq!(
                            caret_allows(current, next),
                            (major, minor) != (0, 0),
                            "{current:?} -> {next:?} ({bump:?}): only 0.0.z has no \
                             compatible next release"
                        );
                    }
                }
            }
        }
    }

    /// Applies change `number` (1 to 13) to shelf 1.0.0, compiles `app.rs`
    /// against the result, runs it, and compares with the two predictions.
    #[track_caller]
    fn check(number: usize, outcome: &str, class: Class, class_digest: u64) {
        let change = &CHANGES[number - 1];
        let changed = apply(&support::read("fixtures/l01/shelf.rs"), change);
        let library = support::library_from_text("shelf", &changed, &Build::default())
            .unwrap_or_else(|errors| {
                panic!(
                    "{}: the changed library does not compile:\n{errors}",
                    change.title
                )
            });
        let build = Build {
            externs: &[("shelf", &library)],
            ..Build::default()
        };
        let label = format!("l01-s{number:02}");
        let run = support::program(
            &label,
            &support::package_dir().join("fixtures/l01/app.rs"),
            &build,
        );
        assert!(
            support::same_outcome(outcome, &run.outcome),
            "{}\n  You predicted `{outcome}` for app.rs. With the changed library it gives `{}`:\n{}",
            change.title,
            run.outcome,
            support::head(&run.output, 24)
        );
        support::assert_answer(
            &format!("S{number:02} class"),
            &format!("{class:?}"),
            class_digest,
            "Look for this kind of change in the list at the top of the Cargo SemVer \
             reference (S08 and S09 are not on it: use the definition of a major \
             change). The class is a statement about every possible user, not only \
             about app.rs.",
        );
    }

    #[test]
    fn s00_app_is_correct_for_version_1() {
        let library = support::library_from_file("shelf", "fixtures/l01/shelf.rs");
        let build = Build {
            externs: &[("shelf", &library)],
            ..Build::default()
        };
        let run = support::program(
            "l01-s00",
            &support::package_dir().join("fixtures/l01/app.rs"),
            &build,
        );
        assert_eq!(
            run.outcome, "ok",
            "app.rs must work with shelf 1.0.0:\n{}",
            run.output
        );
    }

    #[test]
    fn s01_new_function() {
        let s01_outcome: &str = crate::guess();
        let s01_class: Class = crate::guess();
        check(1, s01_outcome, s01_class, 0x6bbc_5251_688e_41e4);
    }

    #[test]
    fn s02_new_public_field() {
        let s02_outcome: &str = crate::guess();
        let s02_class: Class = crate::guess();
        check(2, s02_outcome, s02_class, 0xef79_5717_aece_7c93);
    }

    #[test]
    fn s03_new_variant() {
        let s03_outcome: &str = crate::guess();
        let s03_class: Class = crate::guess();
        check(3, s03_outcome, s03_class, 0x5bfd_9090_8eac_00e2);
    }

    #[test]
    fn s04_new_variant_field() {
        let s04_outcome: &str = crate::guess();
        let s04_class: Class = crate::guess();
        check(4, s04_outcome, s04_class, 0xe9a5_28a1_1040_48bd);
    }

    #[test]
    fn s05_new_parameter() {
        let s05_outcome: &str = crate::guess();
        let s05_class: Class = crate::guess();
        check(5, s05_outcome, s05_class, 0xb845_2958_6cc4_c38c);
    }

    #[test]
    fn s06_generalized_parameter() {
        let s06_outcome: &str = crate::guess();
        let s06_class: Class = crate::guess();
        check(6, s06_outcome, s06_class, 0xfb08_be18_1c5c_ef7b);
    }

    #[test]
    fn s07_changed_return_type() {
        let s07_outcome: &str = crate::guess();
        let s07_class: Class = crate::guess();
        check(7, s07_outcome, s07_class, 0x7449_117f_48b0_4726);
    }

    #[test]
    fn s08_changed_behaviour() {
        // Not on the list of the reference: use the definition of a major change.
        let s08_outcome: &str = crate::guess();
        let s08_class: Class = crate::guess();
        check(8, s08_outcome, s08_class, 0xdcca_5dfc_7e32_a801);
    }

    #[test]
    fn s09_removed_copy() {
        // Not on the list of the reference: use the definition of a major change.
        let s09_outcome: &str = crate::guess();
        let s09_class: Class = crate::guess();
        check(9, s09_outcome, s09_class, 0x7e18_37e8_5162_e0f0);
    }

    #[test]
    fn s10_non_exhaustive_added_later() {
        let s10_outcome: &str = crate::guess();
        let s10_class: Class = crate::guess();
        check(10, s10_outcome, s10_class, 0x0454_c296_4b99_142e);
    }

    #[test]
    fn s11_new_private_field() {
        let s11_outcome: &str = crate::guess();
        let s11_class: Class = crate::guess();
        check(11, s11_outcome, s11_class, 0x3a4d_8fea_6e19_2cb3);
    }

    #[test]
    fn s12_new_inherent_method() {
        let s12_outcome: &str = crate::guess();
        let s12_class: Class = crate::guess();
        check(12, s12_outcome, s12_class, 0x6148_fcbe_353f_cf61);
    }

    #[test]
    fn s13_removed_method() {
        let s13_outcome: &str = crate::guess();
        let s13_class: Class = crate::guess();
        check(13, s13_outcome, s13_class, 0x5f1e_ff9c_1c2b_b905);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Version numbers are compared number by number, not as text.
        // What does each comparison give?
        let p1_numbers: bool = crate::guess();
        let p1_text: bool = crate::guess();
        assert_eq!(
            p1_numbers,
            Version::new(1, 10, 0) > Version::new(1, 9, 9),
            "P1: the derived ordering compares major, then minor, then patch"
        );
        assert_eq!(
            p1_text,
            "1.10.0" > "1.9.9",
            "P1: text is compared character by character, and '1' comes before '9'"
        );

        // P2. A manifest says `shelf = "0.3.1"`. The registry has the versions
        // 0.3.0, 0.3.1, 0.3.9, 0.4.0 and 1.0.0. How many of the five may Cargo
        // choose?
        let p2: usize = crate::guess();
        support::assert_answer(
            "P2 allowed versions",
            &p2.to_string(),
            0x2e29_e443_7832_1058,
            "Before 1.0.0 the minor number is the incompatible one, and a requirement \
             is also a lower bound.",
        );

        // P3. A library is at 0.9.4 and removes a public function. What are the
        // three numbers of the smallest correct next version?
        let p3: (u64, u64, u64) = crate::guess();
        support::assert_answer(
            "P3 next version",
            &format!("{p3:?}"),
            0x9630_5787_7fbb_af35,
            "A removal is a major change. Which number is the incompatible one \
             before 1.0.0? The number after 9 is not 0.",
        );
    }
}
