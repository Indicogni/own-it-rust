//! LESSON 02 — Cargo: new, run, check, build, test
//! Run: cargo test --test chapter01 l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same directory).
//!
//! BEFORE CODING: do the lab from CONCEPTS.md, "Lesson 02", section "Lab: a
//! project of your own". You create a small project in a directory OUTSIDE
//! the course directory and run five Cargo commands in it. The tasks below ask
//! what you saw there.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Which command is enough when you only want to know "does it compile"?
//! 2. Which of the files and directories in a project did you write, and which
//!    did Cargo write?
//!
//! IMPLEMENT the tasks below by returning what you observed in the lab.
//! EXPLAIN afterward: Say what `cargo check`, `cargo build`, `cargo run` and
//! `cargo test` each do, and what `target/` and `Cargo.lock` are for.
//!
//! COMPILER / TRANSFER LAB (optional): In your lab project, delete the whole
//! `target` directory and run `cargo run` again. What is different in the
//! output, compared with the run before? Write one sentence in NOTES.md.

/// A. Return the line that the new program prints when you run it with
/// `cargo run`: only the program's own output, exactly as printed, without
/// the lines that Cargo prints before it.
fn fresh_program_output() -> &'static str {
    todo!("A: the line your lab program printed")
}

/// B. Return the name of the directory in which Cargo puts everything it
/// builds. Only the name, without any `/`.
fn build_directory() -> &'static str {
    todo!("B: the directory that appeared after the first build")
}

/// C. Return the name of the file that Cargo created next to `Cargo.toml`
/// the first time you ran a Cargo command in the project.
fn lock_file() -> &'static str {
    todo!("C: the file Cargo wrote next to Cargo.toml")
}

/// D. Return the complete command, two words, that looks for compile errors
/// but does not produce a program that you can run.
fn fastest_feedback_command() -> &'static str {
    todo!("D: the two-word command")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    use std::path::Path;

    /// The tests in this lesson must not show you the answers, so they do not
    /// contain them. They compare two numbers computed from your text: how
    /// many bytes it has, and the sum of those bytes. You do not need to
    /// understand how.
    fn fingerprint(text: &str) -> (usize, u32) {
        (text.len(), text.bytes().map(u32::from).sum())
    }

    #[test]
    fn a_program_output_is_recorded_exactly() {
        let answer = fresh_program_output();
        assert!(
            fingerprint(answer) == (13, 1161),
            "A: {answer:?} is not the line the new program prints. Run `cargo run` in your \
             lab project and copy the program's line exactly: capital letters, comma, \
             exclamation mark, no spaces at the ends"
        );
    }

    #[test]
    fn b_build_directory_is_named() {
        let answer = build_directory();
        assert!(
            fingerprint(answer) == (6, 647),
            "B: {answer:?} is not the directory where Cargo puts what it builds. List your \
             lab project after `cargo build` and look for the new directory (lower case, \
             no slash)"
        );
    }

    #[test]
    fn c_lock_file_is_named() {
        let answer = lock_file();
        assert!(
            fingerprint(answer) == (10, 963),
            "C: {answer:?} is not the file Cargo wrote next to Cargo.toml. List your lab \
             project and copy the file name exactly, with its capital letter and its dot"
        );
    }

    #[test]
    fn d_check_command_is_named() {
        let answer = fastest_feedback_command();
        assert!(
            fingerprint(answer) == (11, 1066),
            "D: {answer:?} is not the command. It has two lower-case words with one space \
             between them, and the first word is `cargo`"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// These two predictions are about the course repository itself. Look at
    /// the files in its top directory before you answer.
    #[test]
    fn p_predictions() {
        // P1. Open Cargo.toml in the top directory of the course. One line
        // starts with `rust-version`. What text stands between its quotes?
        let p1: &str = guess();
        assert_eq!(
            p1,
            env!("CARGO_PKG_RUST_VERSION"),
            "P1: the rust-version line of the course's Cargo.toml \
             (left: your prediction, right: actual)"
        );

        // P2. Is there a file called Cargo.lock next to that Cargo.toml?
        // Answer true or false.
        let course_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        let p2: bool = guess();
        assert_eq!(
            p2,
            course_directory.join("Cargo.lock").exists(),
            "P2: does the course have a Cargo.lock? (left: your prediction, right: actual)"
        );
    }
}
