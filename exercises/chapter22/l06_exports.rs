//! LESSON 06 — Exports and hygiene
//! Run: cargo test --test chapter22 l06_
//! Then: cargo test --manifest-path exercises/chapter22/export_lab/Cargo.toml
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does an exported macro know the caller has imported its helper? Can a caller variable with the same spelling capture an internal temporary?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN: Explain $crate resolution and the different roles of hygienic locals and referenced item paths.
//! COMPILER LAB (examples/ch22_scratch.rs): Move the macro invocation into a nested module with a same-named helper. Then imagine the library dependency is renamed in Cargo.toml; explain why $crate still works.

/// A. Format a name using a helper that an exported macro can reach.
/// It must return "event:<name>".
pub fn format_event(name: &str) -> String {
    todo!("A: implement the contract")
}
/// B. Name the helper by a path that starts at the crate which DEFINES this
/// macro, so that the macro also works when another crate calls it. Evaluate
/// name once. Keep the chapter-specific export name: an exported macro lives
/// at the crate root, and a short name could collide there.
///
/// Inside this crate, `crate::` and `$crate::` name the same place, so a
/// run-time test here cannot tell them apart. The test
/// `b_helper_path_starts_at_dollar_crate` therefore reads this file's text,
/// and the real proof is the second command above: it calls the macro
/// from another crate that renames this one.
#[macro_export]
macro_rules! ch22_event {
    ($name:expr) => {{ todo!("B: implement the contract") }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    mod caller {
        fn format_event(_: &str) -> String {
            "wrong helper".into()
        }
        pub fn invoke() -> String {
            let name = "caller";
            let out: String = crate::ch22_event!(name);
            assert_eq!(name, "caller");
            out
        }
    }
    #[test]
    fn a_format_event_prefixes_the_name() {
        assert_eq!(format_event("x"), "event:x");
        assert_eq!(format_event(""), "event:");
        assert_eq!(format_event("naïve"), "event:naïve");
    }
    #[test]
    fn b_exported_macro_resolves_its_own_helper() {
        assert_eq!(
            caller::invoke(),
            "event:caller",
            "the macro must reach its own format_event, not the one in the calling module"
        );
        let mut calls = 0;
        let out: String = crate::ch22_event!({
            calls += 1;
            "once"
        });
        assert_eq!(out, "event:once");
        assert_eq!(calls, 1, "the name expression must run exactly once");
        let owned = String::from("owned");
        let out: String = crate::ch22_event!(&owned);
        assert_eq!(out, "event:owned");
    }

    #[test]
    fn b_helper_path_starts_at_dollar_crate() {
        // This file's own text. Inside one crate `crate::` and `$crate::` reach
        // the same function, so only the source shows which one was written.
        let source = include_str!("l06_exports.rs");
        let start = source
            .find("macro_rules! ch22_event")
            .expect("keep the macro's name");
        let end = source[start..]
            .find("#[cfg(test)]")
            .expect("keep the tests module");
        let rule: String = source[start..start + end]
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .flat_map(str::chars)
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(
            rule.contains("$crate::"),
            "write the helper's path as `$crate::...::format_event`: plain `crate::` means the \
             CALLER's crate, so the macro would break when another crate uses it"
        );
    }

    fn helper() -> &'static str {
        "definition site"
    }
    macro_rules! call_helper {
        () => {
            (helper(), $crate::l06_exports::tests::helper())
        };
    }
    macro_rules! here {
        () => {
            module_path!()
        };
    }
    mod user {
        fn helper() -> &'static str {
            "call site"
        }
        pub fn run() -> ((&'static str, &'static str), &'static str) {
            (call_helper!(), here!())
        }
    }
    macro_rules! with_count {
        ($e:expr) => {{
            let count = 100;
            let _ = count;
            $e
        }};
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `call_helper!` is defined beside the first `helper` but invoked in
        // `user`, which has its own. Which helper does each half of the pair call?
        let ((plain, crate_path), _) = user::run();
        let p1: (&str, &str) = guess();
        assert_eq!(p1, (plain, crate_path), "P1: item paths in an expansion");

        // P2. The macro declares `count = 100`; our expression also says `count`.
        let count = 1;
        let p2: i32 = guess();
        assert_eq!(p2, with_count!(count + 1), "P2: whose `count`?");

        // P3. `here!()` wraps `module_path!()` and is defined in `tests`, but
        // invoked inside `user`. Does the path end with "::user"?
        let (_, path) = user::run();
        let p3: bool = guess();
        assert_eq!(p3, path.ends_with("::user"), "P3: module_path!");
    }
}
