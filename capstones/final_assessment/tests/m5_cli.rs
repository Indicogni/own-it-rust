//! Milestone 5: the `rgx` program on standard input and on named files
//! (SPEC 7.1, 7.2 and 7.4 to 7.6).
//!
//! Every test runs the binary in an empty temporary directory and compares
//! standard output, standard error and the exit status with the SPEC.

// The tests start processes, which Miri cannot do.
#![cfg(not(miri))]

mod common;

use common::{FAILED, NOTHING_SELECTED, SELECTED, TempDir, run, run_in};

const POEM: &str = "\
the fog comes
on little cat feet.
It sits looking
over harbor and city
on silent haunches
and then moves on.
";

// ---------------------------------------------------------------------------
// Standard input
// ---------------------------------------------------------------------------

#[test]
fn prints_the_lines_that_match() {
    run(&["on"], POEM).expect(
        "on little cat feet.\non silent haunches\nand then moves on.\n",
        SELECTED,
    );
    run(&["^on"], POEM).expect("on little cat feet.\non silent haunches\n", SELECTED);
    run(&[r"\bcit(y|ies)$"], POEM).expect("over harbor and city\n", SELECTED);
    run(&["s{2}|[A-Z]"], POEM).expect("It sits looking\n", SELECTED);
}

#[test]
fn exit_status_1_when_nothing_is_selected() {
    run(&["dog"], POEM).expect("", NOTHING_SELECTED);
    run(&["x"], "").expect("", NOTHING_SELECTED);
    run(&[""], "").expect("", NOTHING_SELECTED);
}

#[test]
fn each_line_is_searched_on_its_own() {
    // `^` and `$` are the ends of a line; a pattern never sees two lines.
    run(&["^b$"], "a\nb\nc\n").expect("b\n", SELECTED);
    run(&[r"a\nb"], "a\nb\n").expect("", NOTHING_SELECTED);
    run(&["a.b"], "a\nb\n").expect("", NOTHING_SELECTED);
    run(&["[^x]{3}"], "a\nb\n").expect("", NOTHING_SELECTED);
    run(&["^$"], "a\n\nb\n\n").expect("\n\n", SELECTED);
    run(&[""], "a\n\nb\n").expect("a\n\nb\n", SELECTED);
}

#[test]
fn the_last_line_needs_no_line_feed_and_gets_one_in_the_output() {
    run(&["b"], "a\nb").expect("b\n", SELECTED);
    run(&["b$"], "b").expect("b\n", SELECTED);
    run(&[""], "no end").expect("no end\n", SELECTED);
    // A final line feed does not start another, empty line.
    run(&["-c", ""], "a\nb\n").expect("2\n", SELECTED);
    run(&["-c", ""], "a\nb").expect("2\n", SELECTED);
    run(&["-c", ""], "\n").expect("1\n", SELECTED);
    run(&["-c", ""], "").expect("0\n", NOTHING_SELECTED);
}

#[test]
fn a_carriage_return_is_an_ordinary_character() {
    run(&["b$"], "ab\r\nab\n").expect("ab\n", SELECTED);
    run(&[r"b\r$"], "ab\r\nab\n").expect("ab\r\n", SELECTED);
    run(&["-c", "."], "a\rb\rc").expect("1\n", SELECTED);
}

#[test]
fn text_is_utf8() {
    let text = "naïve café\nplain text\n日本語のテキスト\nemoji 😀 here\n";
    run(&["é$"], text).expect("naïve café\n", SELECTED);
    run(&["^.{8}$"], text).expect("日本語のテキスト\n", SELECTED);
    run(&["[^ -~]"], text).expect("naïve café\n日本語のテキスト\nemoji 😀 here\n", SELECTED);
    run(&[r"\u{1F600}"], text).expect("emoji 😀 here\n", SELECTED);
    run(&["-o", "[ぁ-ん]"], text).expect("の\n", SELECTED);
    // The pattern itself may contain any character.
    run(&["日本語"], text).expect("日本語のテキスト\n", SELECTED);
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

#[test]
fn line_numbers() {
    run(&["-n", "on"], POEM).expect(
        "2:on little cat feet.\n5:on silent haunches\n6:and then moves on.\n",
        SELECTED,
    );
    run(&["--line-number", "fog"], POEM).expect("1:the fog comes\n", SELECTED);
    run(&["on", "-n"], POEM).expect(
        "2:on little cat feet.\n5:on silent haunches\n6:and then moves on.\n",
        SELECTED,
    );
    run(&["-n", "^$"], "\n\nx\n\n").expect("1:\n2:\n4:\n", SELECTED);
}

#[test]
fn invert_match() {
    run(&["-v", "on"], POEM).expect(
        "the fog comes\nIt sits looking\nover harbor and city\n",
        SELECTED,
    );
    run(&["--invert-match", "-n", "e"], POEM).expect("3:It sits looking\n", SELECTED);
    run(&["-v", "."], POEM).expect("", NOTHING_SELECTED);
    run(&["-v", "x"], "").expect("", NOTHING_SELECTED);
    run(&["-v", "a"], "a\n\nb\n").expect("\nb\n", SELECTED);
}

#[test]
fn count() {
    run(&["-c", "on"], POEM).expect("3\n", SELECTED);
    run(&["--count", "s"], POEM).expect("4\n", SELECTED);
    // A count of zero is still printed; the status says nothing was selected.
    run(&["-c", "dog"], POEM).expect("0\n", NOTHING_SELECTED);
    run(&["-c", "-v", "on"], POEM).expect("3\n", SELECTED);
    // Lines are counted, not matches.
    run(&["-c", "o"], "foo boo\nxyz\nzoo\n").expect("2\n", SELECTED);
    // `-n` has nothing to number here.
    run(&["-c", "-n", "on"], POEM).expect("3\n", SELECTED);
}

#[test]
fn files_with_matches_names_standard_input() {
    run(&["-l", "fog"], POEM).expect("(standard input)\n", SELECTED);
    run(&["--files-with-matches", "dog"], POEM).expect("", NOTHING_SELECTED);
    run(&["-l", "-v", "fog"], POEM).expect("(standard input)\n", SELECTED);
    run(&["-l", "-n", "fog"], POEM).expect("(standard input)\n", SELECTED);
}

#[test]
fn only_matching() {
    run(&["-o", "[a-z]*on[a-z]*"], POEM).expect("on\non\non\n", SELECTED);
    run(&["-o", "o[a-z]"], POEM).expect("og\nom\non\noo\nov\nor\non\nov\non\n", SELECTED);
    run(&["--only-matching", "-n", r"\b[a-z]{7,}\b"], POEM)
        .expect("3:looking\n5:haunches\n", SELECTED);
    run(&["-o", "dog"], POEM).expect("", NOTHING_SELECTED);
    // The matches of one line are those of `find_iter` (SPEC 4.4).
    run(&["-o", "a|ab"], "abab\n").expect("a\na\n", SELECTED);
    run(&["-o", "aa"], "aaaaa\n").expect("aa\naa\n", SELECTED);
    run(&["-o", "é+|[0-9]+"], "éé12é\n").expect("éé\n12\né\n", SELECTED);
}

#[test]
fn only_matching_never_prints_an_empty_match() {
    // SPEC 7.2: a line with only empty matches is selected (the status is
    // 0) but contributes no output.
    run(&["-o", "x*"], "abc\n").expect("", SELECTED);
    run(&["-o", "b*"], "abba\nccc\n").expect("bb\n", SELECTED);
    run(&["-o", "-n", "[0-9]*"], "a1b22\n\nc\n").expect("1:1\n1:22\n", SELECTED);
    run(&["-o", "^"], "").expect("", NOTHING_SELECTED);
}

#[test]
fn ignore_case_folds_ascii_letters() {
    let text = "Fog\nFOG\nfog\nfig\n";
    run(&["-i", "fog"], text).expect("Fog\nFOG\nfog\n", SELECTED);
    run(&["--ignore-case", "FOG"], text).expect("Fog\nFOG\nfog\n", SELECTED);
    run(&["fog"], text).expect("fog\n", SELECTED);
    run(&["-i", "f[n-p]g"], text).expect("Fog\nFOG\nfog\n", SELECTED);
    run(&["-i", "^F[A-H]G$"], "fig\nfbg\nFIG\n").expect("fbg\n", SELECTED);
    // The haystack is not changed: the output has the original letters.
    run(&["-i", "-o", "o+"], "fOoO\n").expect("OoO\n", SELECTED);
}

#[test]
fn ignore_case_in_classes() {
    // SPEC 3.9: a class gets the other case of its letters *before* it is
    // negated, so `[^a]` refuses `A` too.
    run(&["-i", "-o", "[^a]"], "aAbB\n").expect("b\nB\n", SELECTED);
    run(&["-i", "-o", "[^A-Y]+"], "abcxyzXYZ09\n").expect("z\nZ09\n", SELECTED);
    // The range `X-c` holds X Y Z [ \ ] ^ _ ` a b c; folding adds x y z
    // and A B C.
    run(&["-i", "-o", "[X-c]+"], "ABCD xyzw _^\n").expect("ABC\nxyz\n_^\n", SELECTED);
    run(&["-o", "[X-c]+"], "ABCD xyzw _^\n").expect("_^\n", SELECTED);
    run(&["-i", "-o", r"[\d\W]+"], "aB1 !c\n").expect("1 !\n", SELECTED);
    run(&["-i", "-o", r"\u{41}\u{62}"], "ab AB aB Ab\n").expect("ab\nAB\naB\nAb\n", SELECTED);
}

#[test]
fn ignore_case_leaves_other_letters_alone() {
    // SPEC 3.9: only A-Z and a-z are folded.
    run(&["-i", "é"], "É\né\n").expect("é\n", SELECTED);
    run(&["-i", "É"], "É\né\n").expect("É\n", SELECTED);
    run(&["-i", "σ"], "Σ\nς\nσ\n").expect("σ\n", SELECTED);
    run(&["-i", "[à-þ]"], "À\nà\n").expect("à\n", SELECTED);
    // U+212A KELVIN SIGN and U+017F LONG S are not `k` and `s`.
    run(&["-i", "-c", "k"], "K\nk\n\u{212A}\n").expect("2\n", SELECTED);
    run(&["-i", "-c", "s"], "S\ns\n\u{17F}\n").expect("2\n", SELECTED);
    run(&["-i", "-c", "[^k]"], "K\nk\n\u{212A}\n").expect("1\n", SELECTED);
    // Mixed: the ASCII letters of the pattern fold, the others do not.
    run(&["-i", "CAFÉ"], "café\ncafÉ\nCAFÉ\n").expect("cafÉ\nCAFÉ\n", SELECTED);
}

#[test]
fn options_combine_and_may_stand_anywhere() {
    run(&["-i", "-v", "-n", "ON"], POEM).expect(
        "1:the fog comes\n3:It sits looking\n4:over harbor and city\n",
        SELECTED,
    );
    run(&["ON", "-n", "-v", "-i"], POEM).expect(
        "1:the fog comes\n3:It sits looking\n4:over harbor and city\n",
        SELECTED,
    );
    run(&["-i", "-c", "-v", "IT"], POEM).expect("3\n", SELECTED);
    run(&["-i", "-i", "-n", "-n", "FOG"], POEM).expect("1:the fog comes\n", SELECTED);
}

#[test]
fn after_two_dashes_nothing_is_an_option() {
    run(&["--", "-n"], "a -n b\nc\n").expect("a -n b\n", SELECTED);
    run(&["-n", "--", "-v"], "x\n-v\n").expect("2:-v\n", SELECTED);
    run(&["--", "--"], "a -- b\n").expect("a -- b\n", SELECTED);
    // Not even a request for help.
    run(&["--", "-h"], "a -h b\nc\n").expect("a -h b\n", SELECTED);
    run(&["-c", "--", "--help"], "a\n--help\n").expect("1\n", SELECTED);
    // A lone `-` is not an option.
    run(&["-"], "a-b\nab\n").expect("a-b\n", SELECTED);
    // `--` also ends the options for the paths.
    let dir = TempDir::new();
    dir.write("-c", "x\n");
    run_in(dir.path(), &["x", "--", "-c"], b"").expect("x\n", SELECTED);
}

#[test]
fn help() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["x", "--help"],
        // SPEC 7.1: nothing else is checked, even if other arguments are
        // wrong, and wherever the argument stands before a `--`.
        &["--no-such-option", "-h"],
        &["-c", "-l", "--help", "-j", "0"],
        &["x", "-j", "--help"],
        &["-h", "--", "x"],
    ] {
        let help = run(args, "");
        assert_eq!(help.code, Some(0), "rgx {args:?} must exit with status 0");
        assert!(
            help.stdout.starts_with("Usage: rgx"),
            "the help text must start with `Usage: rgx` (SPEC 7.1); it starts with {:?}",
            help.stdout.lines().next().unwrap_or("")
        );
        assert_eq!(
            help.stderr, "",
            "the help text goes to standard output only"
        );
    }
}

// ---------------------------------------------------------------------------
// Named files
// ---------------------------------------------------------------------------

fn fish() -> TempDir {
    let dir = TempDir::new();
    dir.write("one.txt", "one fish\ntwo fish\n");
    dir.write("two.txt", "red fish\nblue fish\nno end");
    dir.write("empty.txt", "");
    dir
}

#[test]
fn one_file_is_printed_without_its_name() {
    let dir = fish();
    run_in(dir.path(), &["two", "one.txt"], b"").expect("two fish\n", SELECTED);
    run_in(dir.path(), &["-n", "fish", "one.txt"], b"")
        .expect("1:one fish\n2:two fish\n", SELECTED);
    run_in(dir.path(), &["-c", "fish", "one.txt"], b"").expect("2\n", SELECTED);
    run_in(dir.path(), &["fish", "empty.txt"], b"").expect("", NOTHING_SELECTED);
    run_in(dir.path(), &["end$", "two.txt"], b"").expect("no end\n", SELECTED);
    // `-l` prints a name in every case: that is all it prints.
    run_in(dir.path(), &["-l", "fish", "one.txt"], b"").expect("one.txt\n", SELECTED);
}

#[test]
fn a_named_file_means_standard_input_is_not_read() {
    let dir = fish();
    run_in(
        dir.path(),
        &["fish", "one.txt"],
        b"a fish on standard input\n",
    )
    .expect("one fish\ntwo fish\n", SELECTED);
}

#[test]
fn several_files_are_printed_with_their_names_in_argument_order() {
    let dir = fish();
    run_in(dir.path(), &["fish", "one.txt", "two.txt"], b"").expect(
        "one.txt:one fish\none.txt:two fish\ntwo.txt:red fish\ntwo.txt:blue fish\n",
        SELECTED,
    );
    run_in(dir.path(), &["-n", "e fish|no", "two.txt", "one.txt"], b"").expect(
        "two.txt:2:blue fish\ntwo.txt:3:no end\none.txt:1:one fish\n",
        SELECTED,
    );
    run_in(
        dir.path(),
        &["-o", "-n", "[a-z]+ f", "two.txt", "one.txt"],
        b"",
    )
    .expect(
        "two.txt:1:red f\ntwo.txt:2:blue f\none.txt:1:one f\none.txt:2:two f\n",
        SELECTED,
    );
    run_in(
        dir.path(),
        &["-v", "fish", "two.txt", "empty.txt", "one.txt"],
        b"",
    )
    .expect("two.txt:no end\n", SELECTED);
    // The same file twice is two files.
    run_in(dir.path(), &["two", "one.txt", "one.txt"], b"")
        .expect("one.txt:two fish\none.txt:two fish\n", SELECTED);
}

#[test]
fn count_and_list_with_several_files() {
    let dir = fish();
    run_in(
        dir.path(),
        &["-c", "e fish", "one.txt", "empty.txt", "two.txt"],
        b"",
    )
    .expect("one.txt:1\nempty.txt:0\ntwo.txt:1\n", SELECTED);
    run_in(dir.path(), &["-c", "shark", "one.txt", "two.txt"], b"")
        .expect("one.txt:0\ntwo.txt:0\n", NOTHING_SELECTED);
    run_in(
        dir.path(),
        &["-l", "o fish|no", "two.txt", "empty.txt", "one.txt"],
        b"",
    )
    .expect("two.txt\none.txt\n", SELECTED);
    run_in(dir.path(), &["-l", "blue", "one.txt", "two.txt"], b"").expect("two.txt\n", SELECTED);
    run_in(dir.path(), &["-l", "shark", "one.txt", "two.txt"], b"").expect("", NOTHING_SELECTED);
    run_in(
        dir.path(),
        &["-l", "-v", "fish", "one.txt", "two.txt", "empty.txt"],
        b"",
    )
    .expect("two.txt\n", SELECTED);
}

#[test]
fn the_name_is_printed_as_it_was_given() {
    let dir = fish();
    dir.write("sub/deep/three.txt", "three fish\n");
    run_in(dir.path(), &["three", "sub/deep/three.txt", "one.txt"], b"")
        .expect("sub/deep/three.txt:three fish\n", SELECTED);
    run_in(
        dir.path(),
        &["-c", "fish", "./one.txt", "sub/../one.txt"],
        b"",
    )
    .expect("./one.txt:2\nsub/../one.txt:2\n", SELECTED);
    // An absolute path is printed as an absolute path.
    let absolute = dir.path().join("one.txt");
    let absolute = absolute
        .to_str()
        .expect("the temporary directory has a UTF-8 name");
    run_in(dir.path(), &["-l", "fish", absolute, "two.txt"], b"")
        .expect(&format!("{absolute}\ntwo.txt\n"), SELECTED);
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[test]
fn a_pattern_error_shows_the_place() {
    run(&["a(b|c"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 1: missing closing parenthesis\n    a(b|c\n     ^\n",
        FAILED,
    );
    run(&["*"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 0: nothing to repeat\n    *\n    ^\n",
        FAILED,
    );
    run(&[r"ab\q"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 2: unknown escape\n    ab\\q\n      ^\n",
        FAILED,
    );
    run(&["x{2,1}", "-n"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 1: invalid repetition\n    x{2,1}\n     ^\n",
        FAILED,
    );
    run(&["abc\\"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 3: incomplete escape\n    abc\\\n       ^\n",
        FAILED,
    );
    run(&["((a{1000}){1000}){1000}"], POEM).expect_with_stderr(
        "",
        "rgx: pattern error at byte 0: pattern too large\n    ((a{1000}){1000}){1000}\n    ^\n",
        FAILED,
    );
}

#[test]
fn the_caret_counts_characters_while_the_message_counts_bytes() {
    // SPEC 7.5: `é` is two bytes and one column.
    run(&["é(a"], "").expect_with_stderr(
        "",
        "rgx: pattern error at byte 2: missing closing parenthesis\n    é(a\n     ^\n",
        FAILED,
    );
    run(&["日本語[^"], "").expect_with_stderr(
        "",
        "rgx: pattern error at byte 9: missing closing bracket\n    日本語[^\n       ^\n",
        FAILED,
    );
    run(&["😀+*"], "").expect_with_stderr(
        "",
        "rgx: pattern error at byte 5: nothing to repeat\n    😀+*\n      ^\n",
        FAILED,
    );
}

#[test]
fn a_pattern_error_comes_before_any_file_is_touched() {
    let dir = fish();
    run_in(dir.path(), &["-c", "[z-a]", "one.txt", "missing.txt"], b"").expect_with_stderr(
        "",
        "rgx: pattern error at byte 1: invalid class range\n    [z-a]\n     ^\n",
        FAILED,
    );
}

#[test]
fn usage_errors() {
    let cases: &[&[&str]] = &[
        // No pattern.
        &[],
        &["-n"],
        &["-i", "-v"],
        &["--"],
        // Unknown options.
        &["--no-such-option", "x"],
        &["-z", "x"],
        &["x", "--color"],
        // SPEC 7.1: every option is an argument of its own, spelled in full.
        &["-in", "x"],
        &["-j4", "x"],
        &["--threads=4", "x"],
        &["--count=1", "x"],
        &["--ignore", "x"],
        // `--threads` needs a number from 1 to 256.
        &["x", "-j"],
        &["x", "--threads"],
        &["x", "-j", "0"],
        &["x", "-j", "257"],
        &["x", "-j", "many"],
        &["x", "-j", "-1"],
        &["x", "-j", "1.5"],
        &["x", "-j", "+4"],
        &["x", "-j", " 4"],
        &["x", "--threads", ""],
        &["x", "-j", "99999999999999999999999"],
        // Output modes that contradict each other.
        &["-c", "-l", "x"],
        &["-c", "-o", "x"],
        &["-l", "-o", "x"],
        &["--count", "--only-matching", "--files-with-matches", "x"],
        &["-o", "-v", "x"],
        &["--invert-match", "x", "--only-matching"],
    ];
    for args in cases {
        // Input that would match, to show that nothing is searched.
        let run = run(args, "x\n");
        assert_eq!(
            run.stdout, "",
            "rgx {args:?}: a usage error prints nothing on standard output"
        );
        assert_eq!(
            run.code,
            Some(FAILED),
            "rgx {args:?}: a usage error exits with status 2"
        );
        assert!(
            run.stderr.starts_with("rgx: usage error: ") && run.stderr.ends_with('\n'),
            "rgx {args:?}: standard error must start with `rgx: usage error: ` (SPEC 7.5); it is {:?}",
            run.stderr
        );
    }
}

#[test]
fn a_pattern_that_is_not_unicode_is_a_usage_error() {
    // SPEC 7.1. Only Unix lets a test hand over such an argument in a
    // portable way, so on other systems this test checks nothing.
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // 0xFF is never part of UTF-8. A program that reads its arguments
        // with `std::env::args` panics here; `args_os` is the one to use.
        let pattern = OsStr::from_bytes(b"caf\xFF");
        let dir = TempDir::new();
        let run = run_in(dir.path(), &[OsStr::new("-n"), pattern], b"cafe\n");
        assert_eq!(
            run.stdout, "",
            "a usage error prints nothing on standard output"
        );
        assert_eq!(
            run.code,
            Some(FAILED),
            "a pattern that is not valid Unicode is a usage error: status 2, not a crash"
        );
        assert!(
            run.stderr.starts_with("rgx: usage error: ") && run.stderr.lines().count() == 1,
            "standard error must be one line that starts with `rgx: usage error: ` (SPEC 7.5); it is {:?}",
            run.stderr
        );
    }
}

#[test]
fn a_usage_error_comes_before_a_pattern_error() {
    let run = run(&["a(", "-j", "0"], "");
    assert!(
        run.stderr.starts_with("rgx: usage error: "),
        "standard error is {:?}",
        run.stderr
    );
    assert_eq!(run.code, Some(FAILED));
}

#[test]
fn valid_thread_counts_are_accepted() {
    for threads in ["1", "2", "64", "256", "007"] {
        run(&["-j", threads, r"on\.$"], POEM).expect("and then moves on.\n", SELECTED);
        run(&["--threads", threads, "-c", "on"], POEM).expect("3\n", SELECTED);
    }
    // The last one counts.
    run(&["-j", "1", "-j", "4", "fog"], POEM).expect("the fog comes\n", SELECTED);
}

#[test]
fn a_file_that_cannot_be_read_is_reported_and_the_others_are_searched() {
    let dir = fish();
    run_in(dir.path(), &["two", "missing.txt"], b"")
        .expect_error_lines("", &["rgx: missing.txt: "]);
    run_in(
        dir.path(),
        &["two", "one.txt", "missing.txt", "two.txt"],
        b"",
    )
    .expect_error_lines("one.txt:two fish\n", &["rgx: missing.txt: "]);
    run_in(
        dir.path(),
        &["-c", "fish", "nope/a.txt", "one.txt", "also-missing"],
        b"",
    )
    .expect_error_lines("one.txt:2\n", &["rgx: nope/a.txt: ", "rgx: also-missing: "]);
    // Status 2 also when nothing else was selected, and with `-l`.
    run_in(dir.path(), &["shark", "one.txt", "missing.txt"], b"")
        .expect_error_lines("", &["rgx: missing.txt: "]);
    run_in(dir.path(), &["-l", "fish", "missing.txt", "one.txt"], b"")
        .expect_error_lines("one.txt\n", &["rgx: missing.txt: "]);
}

#[test]
fn a_named_file_that_is_not_utf8_is_an_error_with_the_offset() {
    let dir = fish();
    // 0xFF never occurs in UTF-8; C3 28 is a two-byte sequence cut short;
    // ED A0 80 would be a surrogate; F4 90 80 80 is above U+10FFFF.
    dir.write("ff.bin", b"fish\nfish \xFF fish\n");
    dir.write("cut.bin", b"two\xC3(\n");
    dir.write("surrogate.bin", b"\xED\xA0\x80two\n");
    dir.write("high.bin", b"ok\n\xF4\x90\x80\x80\n");
    dir.write("tail.bin", b"two \xE6\x97");
    run_in(dir.path(), &["fish", "ff.bin"], b"").expect_with_stderr(
        "",
        "rgx: ff.bin: invalid UTF-8 at byte 10\n",
        FAILED,
    );
    run_in(
        dir.path(),
        &[
            "-c",
            "two",
            "cut.bin",
            "one.txt",
            "surrogate.bin",
            "high.bin",
            "tail.bin",
        ],
        b"",
    )
    .expect_with_stderr(
        "one.txt:1\n",
        "rgx: cut.bin: invalid UTF-8 at byte 3\n\
             rgx: surrogate.bin: invalid UTF-8 at byte 0\n\
             rgx: high.bin: invalid UTF-8 at byte 3\n\
             rgx: tail.bin: invalid UTF-8 at byte 4\n",
        FAILED,
    );
    // No line of such a file is printed, not even one before the bad byte.
    run_in(dir.path(), &["-n", "fish", "ff.bin", "one.txt"], b"").expect_with_stderr(
        "one.txt:1:one fish\none.txt:2:two fish\n",
        "rgx: ff.bin: invalid UTF-8 at byte 10\n",
        FAILED,
    );
}

#[test]
fn standard_input_that_is_not_utf8_is_an_error_too() {
    let dir = TempDir::new();
    run_in(dir.path(), &["a"], b"a\nb\xFFc\n").expect_with_stderr(
        "",
        "rgx: (standard input): invalid UTF-8 at byte 3\n",
        FAILED,
    );
}

#[test]
fn a_bom_is_just_a_character() {
    // SPEC 9: no byte order mark is recognised. U+FEFF is part of line 1.
    let dir = TempDir::new();
    dir.write("bom.txt", "\u{FEFF}first\nsecond\n");
    run_in(dir.path(), &["-c", "^first", "bom.txt"], b"").expect("0\n", NOTHING_SELECTED);
    run_in(dir.path(), &["-n", r"^\u{FEFF}first$", "bom.txt"], b"")
        .expect("1:\u{FEFF}first\n", SELECTED);
}
