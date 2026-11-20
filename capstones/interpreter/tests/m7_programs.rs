//! Milestone 7: whole programs, and enough speed to be useful.
//!
//! The programs are in `programs/`. Each `NAME.tn` has its expected output
//! in `NAME.out`, and its standard input in `NAME.in` if it reads any. Run
//! one by hand with `cargo run -- programs/NAME.tn`.
//!
//! The speed tests (SPEC 13.4) do not measure anything precisely. Each one
//! gives the interpreter two minutes for work that the reference solution
//! does in well under a second in an unoptimised build. They fail only for
//! an interpreter that does something very expensive at every step, such as
//! copying a list each time it is read.

mod common;

use common::{LONG_TIMEOUT, OK, run_script_for, run_script_with_input};

/// Runs a program from `programs/` and compares its output with the
/// expected text.
#[track_caller]
fn check_program(name: &str, source: &str, input: &str, expected: &str) {
    // The expected text has `\n` line ends, whatever the version control
    // system did to the file on this machine.
    let expected = expected.replace("\r\n", "\n");
    let run = run_script_with_input(source, input.as_bytes());
    if run.stdout != expected || !run.stderr.is_empty() || run.code != Some(OK) {
        eprintln!("programs/{name}.tn does not produce programs/{name}.out:");
    }
    run.expect(&expected, "", OK);
}

macro_rules! program {
    ($name:literal) => {
        check_program(
            $name,
            include_str!(concat!("../programs/", $name, ".tn")),
            "",
            include_str!(concat!("../programs/", $name, ".out")),
        )
    };
    ($name:literal, with_input) => {
        check_program(
            $name,
            include_str!(concat!("../programs/", $name, ".tn")),
            include_str!(concat!("../programs/", $name, ".in")),
            include_str!(concat!("../programs/", $name, ".out")),
        )
    };
}

#[test]
fn program_primes() {
    program!("primes");
}

#[test]
fn program_queens() {
    program!("queens");
}

#[test]
fn program_bank() {
    program!("bank");
}

#[test]
fn program_rpn() {
    program!("rpn");
}

#[test]
fn program_life() {
    program!("life");
}

#[test]
fn program_wordfreq() {
    program!("wordfreq", with_input);
}

// ----- speed ---------------------------------------------------------------

#[test]
fn speed_recursive_calls() {
    // About 250 000 calls.
    let script = "fn fib(n) { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }\nprint(fib(25));";
    run_script_for(script, b"", LONG_TIMEOUT).expect("75025\n", "", OK);
}

#[test]
fn speed_loops_and_arithmetic() {
    let script = "
        var i = 0;
        var sum = 0;
        while i < 1000000 { sum = sum + i % 7; i = i + 1; }
        var count = 0;
        for a in 0..300 { for b in 0..300 { if (a * b) % 7 == 3 { count = count + 1; } } }
        print(sum, count);";
    run_script_for(script, b"", LONG_TIMEOUT).expect("2999997 11008\n", "", OK);
}

#[test]
fn speed_reading_a_list_does_not_copy_it() {
    // 100 000 elements, each read once through a binding and once through a
    // function argument. If a list were copied whenever it is read or
    // passed, this would be 20 000 000 000 element copies.
    let script = "
        let xs = [];
        for i in 0..100000 { push(xs, i); }
        fn at(list, i) { list[i] }
        var sum = 0;
        for i in 0..len(xs) { sum = sum + xs[i] + at(xs, i); }
        print(len(xs), sum);";
    run_script_for(script, b"", LONG_TIMEOUT).expect("100000 9999900000\n", "", OK);
}

#[test]
fn a_source_text_of_one_mebibyte_is_accepted() {
    // SPEC 13.5. Half of the text is 8192 short statements with a comment,
    // the other half is one string literal. A lexer that finds "the
    // character at position i" by counting from the start of the text
    // each time needs minutes for this; one that moves forward needs
    // milliseconds.
    let statement = "n = n + 1; // one of many statements that make this file larger\n";
    assert_eq!(statement.len(), 64, "the test is built from 64-byte lines");
    let script = format!(
        "var n = 0;\n{}print(n, len(\"{}\"));\n",
        statement.repeat(8192),
        "x".repeat(500_000)
    );
    assert!(
        script.len() > 1_000_000 && script.len() <= 1024 * 1024,
        "the script is just under 1 MiB, it has {} bytes",
        script.len()
    );
    run_script_for(&script, b"", LONG_TIMEOUT).expect("8192 500000\n", "", OK);
}

#[test]
fn speed_closures_and_nested_blocks() {
    let script = "
        fn make_counter() { var n = 0; fn() { n = n + 1; n } }
        let next = make_counter();
        var last = 0;
        { { { { { { { { { {
            for i in 0..200000 { last = next(); }
        } } } } } } } } } }
        print(last);";
    run_script_for(script, b"", LONG_TIMEOUT).expect("200000\n", "", OK);
}

#[test]
fn speed_a_lot_of_output_arrives_completely() {
    let script = "for i in 0..200000 { print(\"line\", i); }\nprint(\"end\");";
    let run = run_script_for(script, b"", LONG_TIMEOUT);
    assert_eq!(
        run.code,
        Some(OK),
        "the script must succeed; stderr: {:?}",
        run.stderr
    );
    let mut lines = run.stdout.lines();
    assert_eq!(lines.next(), Some("line 0"), "the first line of output");
    assert_eq!(
        lines.next_back(),
        Some("end"),
        "the last line of output: is the output flushed at the end?"
    );
    assert_eq!(
        lines.next_back(),
        Some("line 199999"),
        "the line before the last"
    );
    assert_eq!(
        run.stdout.lines().count(),
        200_001,
        "every line must arrive, each exactly once"
    );
}
