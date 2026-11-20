//! Milestone 6: the command line, `exit`, `input` and the REPL.
//!
//! SPEC sections 2 (command line, exit statuses, output), 10 (`input` and
//! `exit`) and 12 (the REPL).

mod common;

use common::{
    COMPILE_ERROR, Interactive, OK, RUNTIME_ERROR, TIMEOUT, TempDir, USAGE_OR_IO_ERROR, diagnostic,
    diagnostic_in, expect_runtime_error, run_and_close_output, run_in, run_repl, run_script,
    run_script_with_input,
};

// ----- the command line ----------------------------------------------------

#[test]
fn more_than_one_argument_is_a_usage_error() {
    let dir = TempDir::new();
    dir.write("a.tn", "print(\"a\");");
    dir.write("b.tn", "print(\"b\");");
    let run = run_in(
        dir.path(),
        &["a.tn", "b.tn"],
        b"",
        TIMEOUT,
        "tansy a.tn b.tn",
    );
    run.expect_stderr_start("", "usage: tansy [script]\n", USAGE_OR_IO_ERROR);
}

#[test]
fn an_unknown_option_is_a_usage_error() {
    let dir = TempDir::new();
    let run = run_in(
        dir.path(),
        &["--frobnicate"],
        b"",
        TIMEOUT,
        "tansy --frobnicate",
    );
    run.expect_stderr_start("", "usage: tansy [script]\n", USAGE_OR_IO_ERROR);
    // A single `-` is not a script name either, and `--help` is only help
    // when it is the one argument.
    for args in [&["-"][..], &["-x"], &["--help", "extra"], &["-h", "-h"]] {
        let what = format!("tansy {}", args.join(" "));
        let run = run_in(dir.path(), args, b"", TIMEOUT, &what);
        run.expect_stderr_start("", "usage: tansy [script]\n", USAGE_OR_IO_ERROR);
    }
}

#[test]
fn help_prints_the_usage_on_standard_output() {
    let dir = TempDir::new();
    for option in ["--help", "-h"] {
        let run = run_in(
            dir.path(),
            &[option],
            b"",
            TIMEOUT,
            &format!("tansy {option}"),
        );
        assert!(
            run.stdout.starts_with("usage: tansy [script]\n")
                && run.stderr.is_empty()
                && run.code == Some(OK),
            "`tansy {option}` must print a text that starts with the usage line on standard output and exit with 0.\n\
             stdout: {:?}\nstderr: {:?}\nstatus: {:?}",
            run.stdout,
            run.stderr,
            run.code
        );
    }
}

#[test]
fn a_script_that_cannot_be_read() {
    let dir = TempDir::new();
    let run = run_in(
        dir.path(),
        &["missing.tn"],
        b"",
        TIMEOUT,
        "tansy missing.tn (the file does not exist)",
    );
    run.expect_stderr_start("", "error: cannot read missing.tn", USAGE_OR_IO_ERROR);
    assert_eq!(
        run.stderr.lines().count(),
        1,
        "the message is one line; got {:?}",
        run.stderr
    );
}

#[test]
fn a_directory_is_not_a_script() {
    let dir = TempDir::new();
    dir.write("folder/inside.tn", "print(1);");
    let run = run_in(
        dir.path(),
        &["folder"],
        b"",
        TIMEOUT,
        "tansy folder (a directory)",
    );
    run.expect_stderr_start("", "error: cannot read folder", USAGE_OR_IO_ERROR);
}

#[test]
fn a_script_that_is_not_utf8() {
    let dir = TempDir::new();
    dir.write("latin1.tn", b"print(\"caf\xE9\");\n");
    let run = run_in(
        dir.path(),
        &["latin1.tn"],
        b"",
        TIMEOUT,
        "tansy latin1.tn (the file is not valid UTF-8)",
    );
    run.expect_stderr_start("", "error: cannot read latin1.tn", USAGE_OR_IO_ERROR);
}

#[test]
fn diagnostics_show_the_path_as_it_was_given() {
    let dir = TempDir::new();
    dir.write("scripts/broken.tn", "print(1);\nprint(2 / 0);\n");
    let run = run_in(
        dir.path(),
        &["scripts/broken.tn"],
        b"",
        TIMEOUT,
        "tansy scripts/broken.tn",
    );
    run.expect(
        "1\n",
        "scripts/broken.tn:2:7: error: division by zero\n    print(2 / 0);\n          ^^^^^\n",
        RUNTIME_ERROR,
    );
}

#[test]
fn a_script_runs_whatever_its_file_name_is() {
    let dir = TempDir::new();
    dir.write("no_extension", "print(\"ran\");");
    run_in(
        dir.path(),
        &["no_extension"],
        b"",
        TIMEOUT,
        "tansy no_extension",
    )
    .expect("ran\n", "", OK);
}

// ----- exit ----------------------------------------------------------------

#[test]
fn exit_stops_the_script_with_the_given_status() {
    run_script("print(\"before\");\nexit(3);\nprint(\"after\");").expect("before\n", "", 3);
    run_script("exit(0);\nprint(\"after\");").expect("", "", 0);
    run_script("exit(125);").expect("", "", 125);
}

#[test]
fn exit_works_from_inside_calls_and_loops() {
    let script = "
        fn check(n) { if n == 3 { print(\"stopping at\", n); exit(42); } }
        for i in 0..10 { print(i); check(i); }
        print(\"not reached\");";
    run_script(script).expect("0\n1\n2\n3\nstopping at 3\n", "", 42);
}

#[test]
fn exit_checks_its_argument() {
    expect_runtime_error(
        "exit(126);",
        "",
        &diagnostic(
            1,
            6,
            "exit: expected a code from 0 to 125, found 126",
            "exit(126);",
            3,
        ),
    );
    expect_runtime_error(
        "exit(-1);",
        "",
        &diagnostic(
            1,
            6,
            "exit: expected a code from 0 to 125, found -1",
            "exit(-1);",
            2,
        ),
    );
    expect_runtime_error(
        "exit(\"3\");",
        "",
        &diagnostic(1, 6, "exit: expected int, found str", "exit(\"3\");", 3),
    );
    expect_runtime_error(
        "exit();",
        "",
        &diagnostic(1, 1, "expected 1 argument, found 0", "exit();", 6),
    );
}

// ----- input ---------------------------------------------------------------

#[test]
fn input_reads_one_line_without_its_line_ending() {
    let run = run_script_with_input(
        "print(input());\nprint(input());\nprint([input()]);",
        b"first\nsecond line\n\n",
    );
    run.expect("first\nsecond line\n[\"\"]\n", "", OK);
}

#[test]
fn input_gives_nil_at_the_end_of_input() {
    run_script_with_input("print(input(), input(), input());", b"only\n").expect(
        "only nil nil\n",
        "",
        OK,
    );
    run_script_with_input("print(input());", b"").expect("nil\n", "", OK);
}

#[test]
fn input_accepts_windows_line_endings_and_a_last_line_without_one() {
    let run = run_script_with_input(
        "print([input(), input(), input(), input()]);",
        b"one\r\ntwo\r\nthree",
    );
    run.expect("[\"one\", \"two\", \"three\", nil]\n", "", OK);
}

#[test]
fn input_gives_strings_that_int_can_parse() {
    let script = "
        var sum = 0;
        var skipped = [];
        while true {
            let line = input();
            if line == nil { break; }
            let n = int(line);
            if n == nil { push(skipped, line); } else { sum = sum + n; }
        }
        print(sum, skipped);";
    run_script_with_input(script, b"10\n-3\nnot a number\n35\n 7\n").expect(
        "42 [\"not a number\", \" 7\"]\n",
        "",
        OK,
    );
}

#[test]
fn input_takes_no_argument_and_rejects_bytes_that_are_not_utf8() {
    expect_runtime_error(
        "input(\"> \");",
        "",
        &diagnostic(1, 1, "expected 0 arguments, found 1", "input(\"> \");", 11),
    );
    let run = run_script_with_input("print(input());\nprint(input());", b"fine\ncaf\xE9\n");
    run.expect(
        "fine\n",
        &diagnostic(2, 7, "input: invalid UTF-8", "print(input());", 7),
        RUNTIME_ERROR,
    );
}

#[test]
fn output_is_flushed_before_input_waits() {
    // A script that asks a question and waits for the answer. The test
    // sends the answer only after it has seen the question, so the question
    // must not sit in a buffer while `input()` waits (SPEC 2.3).
    let mut script = Interactive::start_script(
        "print(\"name?\");\nlet name = input();\nprint(\"hello,\", name);\nprint(\"again?\");\nprint([input()]);",
    );
    script.expect("name?\n");
    script.send("Tansy\n");
    script.expect("hello, Tansy\nagain?\n");
    script.send("no\n");
    script.expect("[\"no\"]\n");
    script.finish().expect("", "", OK);
}

// ----- the REPL: one whole session at a time -------------------------------

/// The prompt for a new entry and for the next line of an unfinished one.
const P: &str = "> ";
const C: &str = ". ";

#[test]
fn the_repl_prints_the_value_of_each_entry() {
    // After the last line the input ends: the REPL writes a line feed and
    // exits with 0.
    run_repl("1 + 2\n").expect(&format!("{P}3\n{P}\n"), "", OK);
    run_repl("").expect(&format!("{P}\n"), "", OK);
    // A last line without a line feed is a line like any other (SPEC 12.2).
    run_repl("1 + 2").expect(&format!("{P}3\n{P}\n"), "", OK);
}

#[test]
fn the_repl_shows_values_in_quoted_form_and_shows_nothing_for_nil() {
    let input = "\"text\"\n[1, \"a\"]\nnil\n1 == 1\nprint\n\"tab\\there\"\n";
    let output =
        format!("{P}\"text\"\n{P}[1, \"a\"]\n{P}{P}true\n{P}<fn print>\n{P}\"tab\\there\"\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn statements_have_no_value_and_print_writes_as_usual() {
    let input = "let a = 5\na * 2;\nprint(\"hello\", a)\nvar b = a\nb = b + 1\nb\n";
    let output = format!("{P}{P}{P}hello 5\n{P}{P}{P}6\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn bindings_and_functions_stay_between_entries() {
    let input =
        "var count = 0\nfn bump() { count = count + 1; count }\nbump()\nbump()\ncount + 100\n";
    let output = format!("{P}{P}{P}1\n{P}2\n{P}102\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn an_entry_goes_on_while_a_delimiter_is_open() {
    let input = "fn area(w, h) {\n  w * h\n}\narea(\n  6,\n  7\n)\n[1,\n2]\n";
    let output = format!("{P}{C}{C}{P}{C}{C}{C}42\n{P}{C}[1, 2]\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn delimiters_in_strings_and_comments_do_not_keep_an_entry_open() {
    let input = "\"(\"\n1 // {\n\"[{(\" + \"}\"\n";
    let output = format!("{P}\"(\"\n{P}1\n{P}\"[{{(}}\"\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn empty_entries_do_nothing() {
    let input = "\n   \n// just a comment\n;\n7\n";
    let output = format!("{P}{P}{P}{P}{P}7\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn an_error_is_reported_and_the_session_goes_on() {
    let input = "let a = 10\na +\na / 0\nb\na\n";
    let output = format!("{P}{P}{P}{P}{P}10\n{P}\n");
    let errors = [
        // The entry `a +` is line 2. It ends with its line feed, so "end of
        // input" is the start of line 3, which is still empty at that time.
        diagnostic_in(
            "<repl>",
            3,
            1,
            "expected expression, found end of input",
            "",
            1,
        ),
        diagnostic_in("<repl>", 3, 1, "division by zero", "a / 0", 5),
        diagnostic_in("<repl>", 4, 1, "unknown name `b`", "b", 1),
    ];
    run_repl(input).expect(&output, &errors.concat(), OK);
}

#[test]
fn lines_are_numbered_through_the_whole_session() {
    let input = "let ok = 1\nfn f() {\n  ok +\n    \"text\"\n}\nf()\nprint(\n  1,\n  2 2)\n";
    let output = format!("{P}{P}{C}{C}{C}{P}{P}{C}{C}{P}\n");
    let errors = [
        // The error happens while the entry `f()` (line 6) runs, but its
        // place is in the function, which was typed on lines 2 to 5.
        diagnostic_in(
            "<repl>",
            3,
            3,
            "cannot apply `+` to int and str",
            "  ok +",
            4,
        ),
        diagnostic_in(
            "<repl>",
            9,
            5,
            "expected `,` or `)`, found `2`",
            "  2 2)",
            1,
        ),
    ];
    run_repl(input).expect(&output, &errors.concat(), OK);
}

#[test]
fn lines_read_by_input_are_not_numbered() {
    // Line 1 is `let data = input()`. The next line is data, not source,
    // so `1 / 0` is line 2 of the session.
    let input = "let data = input()\nthis is not Tansy (\n1 / 0\ndata\n";
    let output = format!("{P}{P}{P}\"this is not Tansy (\"\n{P}\n");
    run_repl(input).expect(
        &output,
        &diagnostic_in("<repl>", 2, 1, "division by zero", "1 / 0", 5),
        OK,
    );
}

#[test]
fn an_entry_with_a_syntax_or_name_error_declares_nothing() {
    let input = "let a = 1; let b = nope\na\nlet a = 2; let b = 3\na + b\n";
    let output = format!("{P}{P}{P}{P}5\n{P}\n");
    let errors = [
        diagnostic_in(
            "<repl>",
            1,
            20,
            "unknown name `nope`",
            "let a = 1; let b = nope",
            4,
        ),
        diagnostic_in("<repl>", 2, 1, "unknown name `a`", "a", 1),
    ];
    run_repl(input).expect(&output, &errors.concat(), OK);
}

#[test]
fn a_runtime_error_keeps_what_the_entry_did_before_it() {
    let input = "var n = 1\nn = 2; n = n / 0; n = 3\nn\n";
    let output = format!("{P}{P}{P}2\n{P}\n");
    run_repl(input).expect(
        &output,
        &diagnostic_in(
            "<repl>",
            2,
            12,
            "division by zero",
            "n = 2; n = n / 0; n = 3",
            5,
        ),
        OK,
    );
}

#[test]
fn a_binding_whose_declaration_failed_is_known_but_has_no_value() {
    let input = "let z = 1 / 0\nz\nlet z = 5\nz\n";
    let output = format!("{P}{P}{P}{P}5\n{P}\n");
    let errors = [
        diagnostic_in("<repl>", 1, 9, "division by zero", "let z = 1 / 0", 5),
        diagnostic_in(
            "<repl>",
            2,
            1,
            "`z` is used before it is initialized",
            "z",
            1,
        ),
    ];
    run_repl(input).expect(&output, &errors.concat(), OK);
}

#[test]
fn a_later_entry_may_declare_a_top_level_name_again() {
    let input = "let x = 1\nfn get() { x }\nlet x = \"new\"\nget()\nvar x = x + \"er\"\nx = x + \"!\"\nget()\n";
    let output = format!("{P}{P}{P}{P}\"new\"\n{P}{P}{P}\"newer!\"\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn a_new_declaration_decides_whether_the_name_can_be_assigned() {
    // `v` becomes a `let` binding in entry 3, so entry 4 is refused. The
    // function of entry 2 was checked when `v` was a `var` and still works.
    let input = "var v = 1\nfn bump() { v = v + 1; v }\nlet v = 10\nv = 3\nbump()\nv\n";
    let output = format!("{P}{P}{P}{P}{P}11\n{P}11\n{P}\n");
    run_repl(input).expect(
        &output,
        &diagnostic_in(
            "<repl>",
            4,
            1,
            "cannot assign to `v`: it is not declared with `var`",
            "v = 3",
            1,
        ),
        OK,
    );
}

#[test]
fn one_entry_still_cannot_declare_a_name_twice() {
    let input = "let y = 1; let y = 2\ny\n";
    let output = format!("{P}{P}{P}\n");
    let errors = [
        diagnostic_in(
            "<repl>",
            1,
            16,
            "`y` is already declared in this block",
            "let y = 1; let y = 2",
            1,
        ),
        diagnostic_in("<repl>", 2, 1, "unknown name `y`", "y", 1),
    ];
    run_repl(input).expect(&output, &errors.concat(), OK);
}

#[test]
fn an_entry_can_use_names_it_declares_further_down() {
    // Both functions are in one entry, so each knows the other.
    let input = "fn even(n) { n == 0 or odd(n - 1) } fn odd(n) { n != 0 and even(n - 1) }\neven(10)\nodd(10)\n";
    let output = format!("{P}{P}true\n{P}false\n{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn a_closing_delimiter_too_many_is_an_error_and_not_an_open_entry() {
    let input = "1 }\n2\n";
    let output = format!("{P}{P}2\n{P}\n");
    run_repl(input).expect(
        &output,
        &diagnostic_in("<repl>", 1, 3, "expected expression, found `}`", "1 }", 1),
        OK,
    );
}

#[test]
fn a_lexical_error_ends_the_entry_even_with_a_delimiter_open() {
    let input = "print(\"abc\n1\n";
    let output = format!("{P}{P}1\n{P}\n");
    run_repl(input).expect(
        &output,
        &diagnostic_in("<repl>", 1, 7, "unterminated string", "print(\"abc", 1),
        OK,
    );
}

#[test]
fn an_unfinished_entry_at_the_end_of_input_is_evaluated_as_it_is() {
    let input = "print(\"shown\")\nprint((1 + 2)\n";
    let output = format!("{P}shown\n{P}{C}\n");
    let error = diagnostic_in(
        "<repl>",
        3,
        1,
        "expected `,` or `)`, found end of input",
        "",
        1,
    );
    run_repl(input).expect(&output, &error, OK);
}

#[test]
fn exit_ends_the_session_at_once() {
    run_repl("print(1)\nexit(4)\nprint(2)\n").expect(&format!("{P}1\n{P}"), "", 4);
}

#[test]
fn input_in_the_repl_reads_the_next_line() {
    let input = "let name = input()\nTansy\n\"hello, \" + name\nlet rest = [input(), input()]\nonly one more line\n";
    // The last `input()` finds the end of input, and so does the REPL then.
    let output = format!("{P}{P}\"hello, Tansy\"\n{P}{P}\n");
    run_repl(input).expect(&output, "", OK);
}

#[test]
fn windows_line_endings_in_the_repl() {
    run_repl("1 +\r\n2\r\n\"a\"\r\n").expect(
        &format!("{P}{P}2\n{P}\"a\"\n{P}\n"),
        &diagnostic_in(
            "<repl>",
            2,
            1,
            "expected expression, found end of input",
            "",
            1,
        ),
        OK,
    );
}

#[test]
fn bytes_that_are_not_utf8_become_replacement_characters_in_the_repl() {
    // SPEC 12.2. The byte E9 is "é" in Latin-1 and nothing in UTF-8.
    let dir = TempDir::new();
    let input = b"\"caf\xE9\"\nlen(\"\xE9\xE9\")\n\xE9\n";
    let run = run_in(
        dir.path(),
        &[],
        input,
        TIMEOUT,
        "the REPL with input that is not valid UTF-8: \"caf<E9>\", len(\"<E9><E9>\"), <E9>",
    );
    run.expect(
        &format!("{P}\"caf\u{FFFD}\"\n{P}2\n{P}{P}\n"),
        &diagnostic_in(
            "<repl>",
            3,
            1,
            "unexpected character '\u{FFFD}'",
            "\u{FFFD}",
            1,
        ),
        OK,
    );
}

// ----- the REPL: a conversation --------------------------------------------

#[test]
fn the_repl_answers_each_entry_before_it_reads_the_next() {
    // The input stays open the whole time, so every `expect` below can only
    // succeed if the REPL evaluates an entry as soon as it is complete and
    // flushes its output before it waits for more.
    let mut repl = Interactive::start();
    repl.expect("> ");
    repl.send("let x = 20 + 1\n");
    repl.expect("> ");
    repl.send("x * 2\n");
    repl.expect("42\n> ");
    repl.send("fn greet(name) {\n");
    repl.expect(". ");
    repl.send("  print(\"hello,\", name);\n");
    repl.expect(". ");
    repl.send("}\n");
    repl.expect("> ");
    repl.send("greet(\"you\")\n");
    repl.expect("hello, you\n> ");
    // `input()` waits for a line in the middle of an entry.
    repl.send("let reply = input()\n");
    repl.send("forty-two\n");
    repl.expect("> ");
    repl.send("reply\n");
    repl.expect("\"forty-two\"\n> ");
    repl.finish().expect("\n", "", OK);
}

#[test]
fn errors_arrive_while_the_session_is_open() {
    let mut repl = Interactive::start();
    repl.expect("> ");
    repl.send("1 / 0\n");
    repl.expect("> ");
    repl.send("\"still here\"\n");
    repl.expect("\"still here\"\n> ");
    repl.finish().expect(
        "\n",
        &diagnostic_in("<repl>", 1, 1, "division by zero", "1 / 0", 5),
        OK,
    );
}

// ----- output that cannot be written ---------------------------------------

#[test]
fn a_closed_standard_output_ends_the_run_with_status_3_and_no_panic() {
    // The script prints far more than a pipe holds. The test reads a little
    // and then closes its end, like `tansy script.tn | head -1` does.
    let run = run_and_close_output(
        "for i in 0..2000000 { print(\"line\", i); }\nprint(\"end\");\n",
        b"line 0\n".len(),
    );
    assert_eq!(
        run.stdout, "line 0\n",
        "the script must start printing; the test read {:?}",
        run.stdout
    );
    assert!(
        !run.stderr.contains("panicked"),
        "the interpreter panicked when its output was closed. `println!` panics when it cannot write; \
         write through a handle and treat the error as SPEC 2.4 says.\nstderr: {}",
        run.stderr
    );
    assert_eq!(
        run.code,
        Some(USAGE_OR_IO_ERROR),
        "a failed write to standard output must end the run with status 3 (SPEC 2.4); stderr was: {:?}",
        run.stderr
    );
    assert!(
        run.stderr.starts_with("error: cannot write output"),
        "standard error must start with `error: cannot write output`; it was: {:?}",
        run.stderr
    );
}

// ----- a session of nonsense -----------------------------------------------

/// A small deterministic pseudo-random generator (xorshift64*), so that the
/// test below feeds the same lines on every machine and every run.
struct Random(u64);

impl Random {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % n
    }
}

#[test]
fn two_thousand_lines_of_nonsense_do_not_break_the_session() {
    // Lines made of random tokens: almost all of them are syntax errors,
    // name errors or runtime errors. Whatever they are, the REPL must
    // report them and go on. There are no loops and no `input` or `exit`
    // among the tokens, so every line ends by itself.
    const TOKENS: &[&str] = &[
        "let",
        "var",
        "fn",
        "if",
        "else",
        "return",
        "break",
        "continue",
        "and",
        "or",
        "not",
        "true",
        "false",
        "nil",
        "in",
        "a",
        "b",
        "c",
        "f",
        "g",
        "print",
        "len",
        "push",
        "pop",
        "str",
        "int",
        "type",
        "0",
        "1",
        "2",
        "7",
        "9223372036854775807",
        "99999999999999999999",
        "\"s\"",
        "\"\"",
        "\"un",
        "\"\\q\"",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        ",",
        ";",
        "..",
        "+",
        "-",
        "*",
        "/",
        "%",
        "=",
        "==",
        "!=",
        "<",
        "<=",
        ">",
        ">=",
        "@",
        "é",
    ];
    let mut random = Random(0x7A_4E_53_59);
    let mut input = String::from(
        "var a = 1; var b = [1, 2]; var c = \"text\"; fn f(x) { x } fn g(x, y) { [x, y] }\n",
    );
    for _ in 0..2000 {
        let mut open: Vec<&str> = Vec::new();
        let mut line = String::new();
        for _ in 0..1 + random.below(12) {
            let token = TOKENS[random.below(TOKENS.len())];
            match token {
                "(" => open.push(")"),
                "[" => open.push("]"),
                "{" => open.push("}"),
                _ => {}
            }
            line.push_str(token);
            line.push(' ');
        }
        // Close what was opened, so that each line is one entry (SPEC 12.2:
        // an entry goes on only while more delimiters are open than closed).
        line.extend(open.iter().rev().flat_map(|closer| [*closer, " "]));
        input.push_str(&line);
        input.push('\n');
    }
    input.push_str("fn answer(n) { n + 1 } print(\"still\", \"alive\", answer(41))\n");

    let run = run_repl(&input);
    assert_eq!(
        run.code,
        Some(OK),
        "the REPL must survive every line and exit with 0 at the end of input.\nstderr ends with: {:?}",
        tail(&run.stderr)
    );
    assert!(
        !run.stderr.contains("panicked"),
        "the interpreter panicked:\n{}",
        tail(&run.stderr)
    );
    assert!(
        run.stdout.ends_with("still alive 42\n> \n"),
        "after 2000 lines of nonsense the REPL must still evaluate a correct entry; its output ends with: {:?}",
        tail(&run.stdout)
    );
    let errors = run
        .stderr
        .lines()
        .filter(|line| line.contains(": error: "))
        .count();
    assert!(
        errors >= 1500,
        "most of the 2000 lines are errors and each must be reported; found {errors} error headers"
    );
    assert!(
        run.stderr
            .lines()
            .filter(|line| line.contains(": error: "))
            .all(|line| line.starts_with("<repl>:")),
        "every error in the REPL is reported in the format of SPEC 11.2, for the file name `<repl>`"
    );
}

fn tail(text: &str) -> &str {
    let mut start = text.len().saturating_sub(600);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

#[test]
fn exit_statuses_are_distinct() {
    // One place that states the whole table of SPEC 2.2.
    assert_eq!(run_script("print(1);").code, Some(OK), "success is 0");
    assert_eq!(
        run_script("print(1 / 0);").code,
        Some(RUNTIME_ERROR),
        "a runtime error is 1"
    );
    assert_eq!(
        run_script("print(1 /);").code,
        Some(COMPILE_ERROR),
        "a syntax error is 2"
    );
    assert_eq!(
        run_script("print(nope);").code,
        Some(COMPILE_ERROR),
        "a name error is 2"
    );
    assert_eq!(run_script("exit(9);").code, Some(9), "`exit(n)` is n");
}
