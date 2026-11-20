//! LESSON 01 — Install, verify, and a first green test
//! Run: cargo test --test chapter01 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. What does a test print when the function it calls still says `todo!()`?
//! 2. After you edit one function, which tests change from FAILED to ok?
//!
//! IMPLEMENT the tasks below, one at a time. Run the lesson after each one.
//! EXPLAIN afterward: Say what `todo!()` and `guess()` do when they run, and
//! how the test report shows you which task is still open.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch01_scratch.rs, change the
//! text that `main` prints and run it with `cargo run --example ch01_scratch`.
//! Then delete the last `"` of the text, run again, and read what the
//! compiler says. Put the `"` back so that the file compiles again.

/// A. Return the text `ready`.
///
/// How: replace the whole line `todo!(...)` with `"ready"`. Do not put a
/// semicolon after it.
fn status() -> &'static str {
    todo!("A: return the text \"ready\"")
}

/// B. Return the middle number of the Rust version that this course uses.
///
/// How: open a terminal in the course directory, run `rustc --version`, and
/// read the version. It has three numbers, like `1.23.4`. Return the middle
/// one as a plain number, for example `23`.
fn rust_minor_version() -> u32 {
    todo!("B: return the middle number printed by `rustc --version`")
}

/// C. Return the sum of `a` and `b`.
fn add(a: i32, b: i32) -> i32 {
    todo!("C: return a + b")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_status_is_ready() {
        assert_eq!(status(), "ready");
    }

    #[test]
    fn b_minor_version_matches_the_course_toolchain() {
        // Cargo.toml says which Rust version the course needs, for example
        // "1.23". The test takes the part after the dot and turns it into a
        // number. You do not need to understand this line yet.
        let wanted: u32 = env!("CARGO_PKG_RUST_VERSION")
            .split('.')
            .nth(1)
            .and_then(|minor| minor.parse().ok())
            .expect("Cargo.toml should have a rust-version like 1.23");
        assert_eq!(
            rust_minor_version(),
            wanted,
            "left: the number you returned. right: the middle number of the version this \
             course needs. Run `rustc --version` inside the course directory and read it again"
        );
    }

    #[test]
    fn c_add_returns_the_sum() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-4, 4), 0);
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(100, -1), 99);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. What is the value of this expression?
        let p1: i32 = guess();
        assert_eq!(
            p1,
            2 + 3 * 4,
            "P1: 2 + 3 * 4 (left: your prediction, right: actual)"
        );

        // P2. And of this one?
        let p2: i32 = guess();
        assert_eq!(
            p2,
            10 - 4 - 3,
            "P2: 10 - 4 - 3 (left: your prediction, right: actual)"
        );
    }
}
