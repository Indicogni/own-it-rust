//! CODE REVIEW 01 — A teammate's macro pull request
//! Run: cargo test --test chapter22 c01_
//! Then: cargo clippy --test chapter22
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and most behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every unnecessary allocation, and every place where an
//!    error is swallowed or a panic replaces propagation.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep each macro's invocation syntax; the tests use it.
//! 4. Run clippy and resolve its warnings for this file.

use std::fmt;

/// Return the larger of two expressions; ties return the first. Like a
/// function call, evaluate each argument exactly once, left to right.
macro_rules! max_of {
    ($a:expr, $b:expr) => {
        // The tuple evaluates `$a`, then `$b`, each once. `std::cmp::max`
        // would not do: it needs `Ord` and returns the second on a tie.
        match ($a, $b) {
            (a, b) => {
                if a >= b {
                    a
                } else {
                    b
                }
            }
        }
    };
}

/// Build a Vec of "#tag" Strings, one per argument, in order. Exported with
/// `#[macro_export]`, so other crates may call it too: wherever it is called,
/// it must reach THIS module's `tag_name`, even where the caller defines a
/// function with the same name.
#[macro_export]
macro_rules! ch22_tags {
    ($($tag:expr),* $(,)?) => {
        // `$crate` reaches this module's helper from any caller. vec![...]
        // allocates once, and each String moves into it without a clone.
        {
            let tags: ::std::vec::Vec<::std::string::String> =
                ::std::vec![$($crate::c01_code_review::tag_name($tag)),*];
            tags
        }
    };
}

#[doc(hidden)]
pub fn tag_name(tag: &str) -> String {
    format!("#{tag}")
}

/// Implement Display for unit newtypes: `Meters(3)` shows as "3 m". Errors
/// from the formatter must reach the caller of `write!`.
macro_rules! impl_unit_display {
    ($($ty:ident => $unit:literal),* $(,)?) => {
        $(
            impl fmt::Display for $ty {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    // Writes straight into the formatter and returns its Result.
                    write!(f, "{} {}", self.0, $unit)
                }
            }
        )*
    };
}

struct Meters(u32);
struct Seconds(f64);
impl_unit_display!(Meters => "m", Seconds => "s");

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::fmt::Write;

    #[test]
    fn a_max_of_evaluates_each_argument_once() {
        let mut calls = 0;
        let mut next = || {
            calls += 1;
            calls
        };
        let biggest = max_of!(next(), next());
        assert_eq!(
            biggest, 2,
            "return the value that was compared, not a fresh evaluation"
        );
        assert_eq!(calls, 2, "two arguments, two evaluations");
    }

    #[test]
    fn a_max_of_returns_the_first_on_a_tie() {
        let zero = max_of!(-0.0_f64, 0.0_f64);
        assert!(zero.is_sign_negative(), "-0.0 == 0.0, so keep the first");
        assert_eq!(max_of!("b", "a"), "b");
    }

    #[test]
    fn b_tags_render_in_order() {
        assert_eq!(ch22_tags!("a", "naïve",), ["#a", "#naïve"]);
        let none: Vec<String> = ch22_tags!();
        assert!(none.is_empty());
    }

    mod caller {
        fn tag_name(_: &str) -> String {
            "wrong helper".into()
        }
        pub fn run() -> Vec<String> {
            crate::ch22_tags!("x")
        }
    }

    #[test]
    fn b_tags_use_the_defining_module_helper() {
        assert_eq!(
            caller::run(),
            ["#x"],
            "the macro must call its own tag_name, not the one in the calling module"
        );
    }

    #[test]
    fn b_tags_helper_path_starts_at_dollar_crate() {
        // This file's own text. Inside one crate `crate::` and `$crate::` reach
        // the same function, so only the source shows which one was written.
        let source = include_str!("c01_code_review.rs");
        let start = source
            .find("macro_rules! ch22_tags")
            .expect("keep the macro's name");
        let end = source[start..]
            .find("fn tag_name")
            .expect("keep the helper function `tag_name` below the macro");
        let rule: String = source[start..start + end]
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .flat_map(str::chars)
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(
            rule.contains("$crate::"),
            "an exported macro must name its helper as `$crate::...::tag_name`: plain `crate::` \
             means the CALLER's crate, and a bare name is looked up in the caller's module"
        );
    }

    #[test]
    fn b_tags_do_not_clone_or_regrow() {
        // What one call of the helper costs. Measured here, so that the test
        // does not depend on how many allocations `format!` happens to make.
        let (_, per_tag) = count(|| std::hint::black_box(tag_name("t0")));
        let (_, one_format) =
            count(|| std::hint::black_box(format!("#{}", std::hint::black_box("t0"))));
        assert!(
            per_tag <= one_format,
            "tag_name makes {per_tag} allocations, but one format! call makes {one_format}: \
             do not build a String only to pass it to format!"
        );
        let (tags, allocations) =
            count(|| ch22_tags!("t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7"));
        assert_eq!(tags.len(), 8);
        assert!(
            allocations <= per_tag * 8 + 1,
            "8 tags cost {allocations} allocations, but 8 calls of tag_name cost {} and the Vec \
             needs only one more: remove the clone of each String, and allocate the Vec once \
             at its final size",
            per_tag * 8
        );
    }

    #[test]
    fn c_units_display() {
        assert_eq!(Meters(3).to_string(), "3 m");
        assert_eq!(Seconds(1.5).to_string(), "1.5 s");
    }

    /// A writer that always fails, like a closed pipe.
    struct Refuse;
    impl Write for Refuse {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    #[test]
    fn c_display_propagates_formatter_errors() {
        assert!(write!(Refuse, "{}", Meters(3)).is_err());
    }

    #[test]
    fn c_display_writes_without_a_temporary_string() {
        let mut out = String::with_capacity(64);
        let (result, allocations) = count(|| write!(out, "{}", Meters(42)));
        assert!(result.is_ok());
        assert_eq!(out, "42 m");
        assert_eq!(allocations, 0, "write straight into the formatter");
    }
}
