//! LESSON 02 — Reproducing a bug report
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l02_
//! Read first: CONCEPTS.md, "Lesson 02", then `fixture/issues/0014-menu-output-wrong.md`.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: The reporter attached a 30-line
//! Lanefile. Which lines can you delete before you even run it? What must a fix
//! leave unchanged?
//! IMPLEMENT: A. reduce the report to a minimal case. B. fix the bug in the fixture,
//! in `fixture/crates/tasklane-core/src/vars.rs`, without changing any other
//! behaviour. C. add your minimal case to the fixture's own tests: a `#[test]` in the
//! `tests` module at the end of that file. Preserve the tests of this file.
//! EXPLAIN afterward: State the bug in one sentence that names the input, the
//! expected output and the actual output. Explain why each `b_unchanged_` test exists
//! and which careless fix it would catch.

/// The smallest input that shows a bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repro {
    /// The text given to `expand`.
    pub template: &'static str,
    /// The variables that are set, as `(name, value)` pairs.
    pub vars: &'static [(&'static str, &'static str)],
    /// What the documentation of `expand` says the result must be.
    pub expected: &'static str,
}

/// A. The minimal reproduction of issue 14: at most three characters of template, no
/// variables, and no braces. It must fail on the released version (the copy in
/// [`v0_4_0`]) and pass once you have fixed the fixture.
pub fn repro() -> Repro {
    todo!("Implement this task")
}

/// The code of `tasklane_core::vars::expand` exactly as released in 0.4.0. It is here
/// so that the tests can check your reproduction against the version the reporter
/// used. Do not edit it; fix the fixture instead.
pub mod v0_4_0 {
    use tasklane_core::{Error, Vars};

    /// `expand` as released in tasklane 0.4.0.
    pub fn expand(template: &str, vars: &Vars) -> Result<String, Error> {
        let mut out = String::with_capacity(template.len());
        let mut chars = template.chars();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('{') => {
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(part) => name.push(part),
                            None => return Err(Error::UnterminatedVariable(template.to_string())),
                        }
                    }
                    let value = vars.get(&name).ok_or(Error::UnknownVariable(name))?;
                    out.push_str(value);
                }
                Some('$') => out.push('$'),
                _ => out.push('$'),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{count_tests, read};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;
    use tasklane_core::{Error, Vars, expand};

    fn vars(pairs: &[(&str, &str)]) -> Vars {
        let mut vars = Vars::new();
        for (name, value) in pairs {
            vars.set(name, value);
        }
        vars
    }

    #[test]
    fn a_repro_is_minimal() {
        let repro = repro();
        assert!(
            repro.template.chars().count() <= 3,
            "A: `{}` can be made shorter; delete characters while the bug still shows",
            repro.template
        );
        assert!(
            repro.vars.is_empty(),
            "A: the bug shows without any variable; remove them from the reproduction"
        );
        assert!(
            !repro.template.contains(['{', '}']),
            "A: the bug shows without braces; remove them from the reproduction"
        );
    }

    #[test]
    fn a_repro_states_the_documented_result() {
        let repro = repro();
        // Without braces the documentation leaves one rule: `$$` gives `$`.
        assert_eq!(
            repro.expected,
            repro.template.replace("$$", "$"),
            "A: `expected` must be what the documentation of `expand` promises"
        );
    }

    #[test]
    fn a_repro_fails_on_the_released_version() {
        let repro = repro();
        assert_ne!(
            v0_4_0::expand(repro.template, &vars(repro.vars)),
            Ok(repro.expected.to_string()),
            "A: version 0.4.0 already gives the expected result, so this input does not show the bug"
        );
    }

    #[test]
    fn b_fix_makes_the_repro_pass() {
        let repro = repro();
        assert_eq!(
            expand(repro.template, &vars(repro.vars)),
            Ok(repro.expected.to_string()),
            "B: the fixture still gives the wrong result for your reproduction"
        );
    }

    #[test]
    fn b_fix_covers_the_original_report() {
        let menu = vars(&[("SHOP", "Corner Cafe")]);
        assert_eq!(
            expand("${SHOP}: coffee $3, tea $2, cake $4.50", &menu).unwrap(),
            "Corner Cafe: coffee $3, tea $2, cake $4.50"
        );
        assert_eq!(expand("100$ off", &menu).unwrap(), "100$ off");
        assert_eq!(expand("$\u{e9}t\u{e9}", &menu).unwrap(), "$\u{e9}t\u{e9}");
        assert_eq!(expand("$x$y$", &menu).unwrap(), "$x$y$");
        assert_eq!(expand("$5${SHOP}$", &menu).unwrap(), "$5Corner Cafe$");
    }

    #[test]
    fn b_unchanged_double_dollar_is_still_an_escape() {
        let v = vars(&[("A", "apple")]);
        assert_eq!(expand("$$", &v).unwrap(), "$");
        assert_eq!(expand("$${A}", &v).unwrap(), "${A}");
        assert_eq!(expand("$$$$", &v).unwrap(), "$$");
        assert_eq!(expand("$$5", &v).unwrap(), "$5");
        assert_eq!(
            expand("$$${A}", &v).unwrap(),
            "$apple",
            "an escaped dollar followed by a real variable"
        );
    }

    #[test]
    fn b_unchanged_variables_still_expand() {
        let v = vars(&[("A", "apple"), ("B", "bee")]);
        assert_eq!(expand("${A}${B}", &v).unwrap(), "applebee");
        assert_eq!(expand("${A}$", &v).unwrap(), "apple$");
        assert_eq!(expand("${A} and ${B}.", &v).unwrap(), "apple and bee.");
        assert_eq!(expand("", &v).unwrap(), "");
    }

    #[test]
    fn b_unchanged_values_are_not_expanded_again() {
        let v = vars(&[("A", "${B}"), ("B", "bee"), ("C", "$$")]);
        assert_eq!(expand("${A}", &v).unwrap(), "${B}");
        assert_eq!(expand("${C}", &v).unwrap(), "$$");
    }

    #[test]
    fn b_unchanged_mistakes_are_still_errors() {
        let v = vars(&[("A", "apple")]);
        assert_eq!(
            expand("${NOPE}", &v),
            Err(Error::UnknownVariable("NOPE".to_string())),
            "an unknown variable must stay an error; do not turn it into literal text"
        );
        assert_eq!(
            expand("${}", &v),
            Err(Error::UnknownVariable(String::new()))
        );
        assert_eq!(
            expand("cost ${A", &v),
            Err(Error::UnterminatedVariable("cost ${A".to_string())),
            "a missing `}}` must stay an error"
        );
        assert_eq!(
            expand("${", &v),
            Err(Error::UnterminatedVariable("${".to_string()))
        );
    }

    #[test]
    fn c_fixture_has_a_regression_test() {
        let source = read("fixture/crates/tasklane-core/src/vars.rs");
        assert!(
            count_tests(&source) >= 6,
            "C: add your minimal case as a `#[test]` in the fixture's vars.rs, next to \
             the five tests that are already there. A fix without a test can break again."
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer. Read
    /// `v0_4_0::expand` above and work out what the released code does. All three
    /// calls return `Ok`.
    #[test]
    fn p_predictions() {
        let none = Vars::new();

        // P1. v0_4_0::expand("100$ off", &none)
        let p1: &str = guess();
        assert_eq!(
            p1,
            v0_4_0::expand("100$ off", &none).unwrap(),
            "P1: what 0.4.0 does with the character after a lone `$`"
        );

        // P2. v0_4_0::expand("$$$5", &none)
        let p2: &str = guess();
        assert_eq!(
            p2,
            v0_4_0::expand("$$$5", &none).unwrap(),
            "P2: an escape followed by a lone `$`"
        );

        // P3. v0_4_0::expand("a$", &none)
        let p3: &str = guess();
        assert_eq!(
            p3,
            v0_4_0::expand("a$", &none).unwrap(),
            "P3: a `$` at the very end"
        );
    }
}
