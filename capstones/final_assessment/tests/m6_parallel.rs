//! Milestone 6: directories, many files, many threads, one deterministic
//! output (SPEC 7.3, 7.4 and 7.7).
//!
//! Nothing here measures speed, with one exception that is marked
//! `#[ignore]` and judges nothing: `speed_report` prints numbers for
//! RUBRIC.md.

// The tests start processes, which Miri cannot do.
#![cfg(not(miri))]

mod common;

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::{
    FAILED, NOTHING_SELECTED, RUN_LIMIT, SELECTED, TempDir, run_in, spawn, wait_with_limit,
};

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

/// A small tree whose names are chosen to tell the sorting rules apart.
fn tree() -> TempDir {
    let dir = TempDir::new();
    for name in [
        "top/b.txt",
        "top/a.txt",
        "top/B2.txt",
        "top/a/y.txt",
        "top/a/x.txt",
        "top/a-b.txt",
        "top/a.b/z.txt",
        "top/10.txt",
        "top/9.txt",
        "top/_u.txt",
        "top/c/d/e/deep.txt",
        "top/c/d/also.txt",
        "top/zz/last.txt",
    ] {
        dir.write(name, format!("hit in {name}\nmiss\n"));
    }
    dir
}

/// SPEC 7.3: in each directory the entries are visited in the byte order of
/// their names, and a directory is searched to its end at the place of its
/// name.
const TREE_ORDER: [&str; 13] = [
    "top/10.txt",
    "top/9.txt",
    "top/B2.txt",
    "top/_u.txt",
    "top/a/x.txt",
    "top/a/y.txt",
    "top/a-b.txt",
    "top/a.b/z.txt",
    "top/a.txt",
    "top/b.txt",
    "top/c/d/also.txt",
    "top/c/d/e/deep.txt",
    "top/zz/last.txt",
];

#[test]
fn a_directory_is_searched_with_everything_below_it() {
    let dir = tree();
    let expected: String = TREE_ORDER
        .iter()
        .map(|name| format!("{name}:hit in {name}\n"))
        .collect();
    run_in(dir.path(), &["hit", "top"], b"").expect(&expected, SELECTED);
}

#[test]
fn the_order_is_by_name_in_each_directory_and_the_same_for_every_thread_count() {
    let dir = tree();
    let names: String = TREE_ORDER.iter().map(|name| format!("{name}\n")).collect();
    let counts: String = TREE_ORDER
        .iter()
        .map(|name| format!("{name}:1\n"))
        .collect();
    for threads in ["1", "2", "3", "8", "64"] {
        run_in(dir.path(), &["-l", "-j", threads, "hit", "top"], b"").expect(&names, SELECTED);
        run_in(dir.path(), &["-c", "-j", threads, "^miss$", "top"], b"").expect(&counts, SELECTED);
    }
    // Without `-j` the program chooses the number of threads itself.
    run_in(dir.path(), &["-l", "hit", "top"], b"").expect(&names, SELECTED);
}

#[test]
fn names_below_a_directory_are_joined_with_slashes() {
    let dir = tree();
    run_in(dir.path(), &["-n", "deep", "top/c"], b"")
        .expect("top/c/d/e/deep.txt:1:hit in top/c/d/e/deep.txt\n", SELECTED);
    run_in(dir.path(), &["-l", "also|deep", "top/c/d"], b"")
        .expect("top/c/d/also.txt\ntop/c/d/e/deep.txt\n", SELECTED);
    // The operand is printed as it was given; what was found is appended.
    run_in(dir.path(), &["-l", "last", "./top/zz"], b"").expect("./top/zz/last.txt\n", SELECTED);
    run_in(dir.path(), &["-l", "hit", "top/a/../zz"], b"")
        .expect("top/a/../zz/last.txt\n", SELECTED);
    // An operand that ends in `/` does not get a second one.
    run_in(dir.path(), &["-l", "last|also", "top/zz/", "top/c/"], b"")
        .expect("top/zz/last.txt\ntop/c/d/also.txt\n", SELECTED);
    run_in(dir.path(), &["-l", "hit", "."], b"").expect(
        &TREE_ORDER
            .iter()
            .map(|name| format!("./{name}\n"))
            .collect::<String>(),
        SELECTED,
    );
}

#[test]
fn a_directory_always_shows_names_and_operands_keep_their_order() {
    let dir = tree();
    dir.write("solo/only.txt", "hit\n");
    dir.write("first.txt", "hit first\n");
    // One operand, one file in it: the name is printed all the same,
    // because the operand is a directory.
    run_in(dir.path(), &["hit", "solo"], b"").expect("solo/only.txt:hit\n", SELECTED);
    run_in(dir.path(), &["-c", "hit", "solo"], b"").expect("solo/only.txt:1\n", SELECTED);
    // Operands are searched in the order given, not in sorted order.
    run_in(dir.path(), &["-l", "hit", "top/zz", "first.txt", "top/a", "solo", "top/a"], b"").expect(
        "top/zz/last.txt\nfirst.txt\ntop/a/x.txt\ntop/a/y.txt\nsolo/only.txt\ntop/a/x.txt\ntop/a/y.txt\n",
        SELECTED,
    );
}

#[test]
fn empty_directories_and_empty_files() {
    let dir = TempDir::new();
    dir.create_dir("nothing/at/all");
    dir.write("some/empty.txt", "");
    dir.create_dir("some/hollow");
    run_in(dir.path(), &["x", "nothing"], b"").expect("", NOTHING_SELECTED);
    run_in(dir.path(), &["-c", "x", "nothing"], b"").expect("", NOTHING_SELECTED);
    run_in(dir.path(), &["-c", "", "some", "nothing"], b"")
        .expect("some/empty.txt:0\n", NOTHING_SELECTED);
}

#[test]
fn files_that_are_not_text_are_skipped_inside_a_directory() {
    let dir = TempDir::new();
    dir.write("mix/a.txt", "needle a\n");
    dir.write("mix/b.bin", b"needle \xFF\xFE binary\n");
    dir.write("mix/c.txt", "needle c\n");
    dir.write("mix/d.dat", b"\x80needle\n");
    // SPEC 7.4: found by walking a directory: skipped without a message.
    run_in(dir.path(), &["needle", "mix"], b"")
        .expect("mix/a.txt:needle a\nmix/c.txt:needle c\n", SELECTED);
    run_in(dir.path(), &["-c", "needle", "mix"], b"")
        .expect("mix/a.txt:1\nmix/c.txt:1\n", SELECTED);
    run_in(dir.path(), &["-l", "-v", "needle", "mix"], b"").expect("", NOTHING_SELECTED);
    // Named as an operand: an error, in its place among the others.
    run_in(
        dir.path(),
        &["needle", "mix/a.txt", "mix/b.bin", "mix"],
        b"",
    )
    .expect_with_stderr(
        "mix/a.txt:needle a\nmix/a.txt:needle a\nmix/c.txt:needle c\n",
        "rgx: mix/b.bin: invalid UTF-8 at byte 7\n",
        FAILED,
    );
}

#[test]
fn symbolic_links_inside_a_directory_are_passed_over() {
    // SPEC 7.3. Making a link needs special rights on some systems, so only
    // Unix runs this; elsewhere the test checks nothing.
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new();
        dir.write("real/a.txt", "hit a\n");
        dir.write("real/sub/b.txt", "hit b\n");
        dir.write("outside.txt", "hit outside\n");
        let link = |target: &str, name: &str| {
            symlink(target, dir.path().join(name))
                .unwrap_or_else(|e| panic!("cannot create the symbolic link {name}: {e}"));
        };
        // Inside the directory: a link to a file, a link to a directory, a
        // link that leads back to the directory it stands in (a program
        // that follows it never comes to an end), and one that leads
        // nowhere.
        link("../outside.txt", "real/link-to-file");
        link("sub", "real/link-to-dir");
        link("..", "real/sub/link-to-parent");
        link("no-such-file", "real/dangling");
        run_in(dir.path(), &["-j", "2", "hit", "real"], b"")
            .expect("real/a.txt:hit a\nreal/sub/b.txt:hit b\n", SELECTED);
        run_in(dir.path(), &["-c", "hit", "real/sub"], b"").expect("real/sub/b.txt:1\n", SELECTED);

        // A link that is itself an operand is treated as what it points to,
        // and keeps the name it was given.
        link("outside.txt", "file-link");
        link("real/sub", "dir-link");
        run_in(dir.path(), &["hit", "file-link"], b"").expect("hit outside\n", SELECTED);
        run_in(dir.path(), &["hit", "dir-link", "file-link"], b"")
            .expect("dir-link/b.txt:hit b\nfile-link:hit outside\n", SELECTED);
    }
}

#[test]
fn missing_operands_are_reported_in_order_among_many_files() {
    let dir = tree();
    for threads in ["1", "4", "32"] {
        run_in(
            dir.path(),
            &[
                "-j",
                threads,
                "-l",
                "hit",
                "gone-1",
                "top/a",
                "gone-2",
                "top/zz",
                "top/gone-3",
                "top/9.txt",
            ],
            b"",
        )
        .expect_error_lines(
            "top/a/x.txt\ntop/a/y.txt\ntop/zz/last.txt\ntop/9.txt\n",
            &["rgx: gone-1: ", "rgx: gone-2: ", "rgx: top/gone-3: "],
        );
    }
}

// ---------------------------------------------------------------------------
// Many files of very different sizes
// ---------------------------------------------------------------------------

/// A corpus where the files that come first in the output take the longest
/// to search, so that a program which prints each file when its thread is
/// done prints them in the wrong order.
///
/// Returns the directory and, for `needle [0-9]+$` with `-n`, the expected
/// output.
fn corpus(files: usize, first_lines: usize) -> (TempDir, String) {
    let dir = TempDir::new();
    let mut expected = String::new();
    for file in 0..files {
        // Sizes fall quickly: file 0 is the largest by far.
        let lines = (first_lines >> file.min(10)).max(3) + file % 5;
        let name = format!("data/part-{:02}/file-{file:03}.log", file / 10);
        let mut text = String::new();
        for line in 0..lines {
            if (line * 7 + file) % 11 == 0 {
                let _ = writeln!(text, "{file} needle {line}");
                let _ = writeln!(expected, "{name}:{}:{file} needle {line}", line + 1);
            } else {
                // Lines that almost match keep the engine busy.
                let _ = writeln!(
                    text,
                    "{file} needle {line} needle needle needle needle needl"
                );
            }
        }
        dir.write(&name, text);
    }
    (dir, expected)
}

#[test]
fn the_output_is_the_same_for_every_thread_count() {
    let (dir, expected) = corpus(120, 8_000);
    assert!(
        expected.lines().count() > 1_000,
        "the corpus is large enough to be worth a test"
    );
    for threads in ["1", "2", "4", "7", "16", "128"] {
        run_in(
            dir.path(),
            &["-n", "-j", threads, "needle [0-9]+$", "data"],
            b"",
        )
        .expect(&expected, SELECTED);
    }
    run_in(dir.path(), &["-n", "needle [0-9]+$", "data"], b"").expect(&expected, SELECTED);
}

#[test]
fn the_output_is_the_same_on_every_run() {
    let (dir, expected) = corpus(60, 4_000);
    for _ in 0..8 {
        run_in(
            dir.path(),
            &["-n", "-j", "8", "needle [0-9]+$", "data"],
            b"",
        )
        .expect(&expected, SELECTED);
    }
    // The other output modes go through the same ordering.
    let counts = run_in(
        dir.path(),
        &["-c", "-j", "1", "needle [0-9]+$", "data"],
        b"",
    );
    assert_eq!(counts.stdout.lines().count(), 60);
    let only = run_in(
        dir.path(),
        &["-o", "-j", "1", "needle [0-9]+$", "data"],
        b"",
    );
    for _ in 0..3 {
        run_in(
            dir.path(),
            &["-c", "-j", "8", "needle [0-9]+$", "data"],
            b"",
        )
        .expect(&counts.stdout, SELECTED);
        run_in(
            dir.path(),
            &["-o", "-j", "8", "needle [0-9]+$", "data"],
            b"",
        )
        .expect(&only.stdout, SELECTED);
    }
}

#[test]
fn more_threads_than_files_and_files_named_one_by_one() {
    let dir = TempDir::new();
    let mut args = vec!["-j", "256", "-n", "x$"];
    let names: Vec<String> = (0..40).map(|n| format!("f{n}.txt")).collect();
    let mut expected = String::new();
    // Operands in the reverse of the sorted order: argument order decides.
    for (n, name) in names.iter().enumerate().rev() {
        dir.write(name, format!("{n} x\n{n} y\n").repeat(1 + n % 3));
        for copy in 0..1 + n % 3 {
            let _ = writeln!(expected, "{name}:{}:{n} x", 2 * copy + 1);
        }
        args.push(name);
    }
    run_in(dir.path(), &args, b"").expect(&expected, SELECTED);
    run_in(dir.path(), &["-j", "256", "x", "f0.txt"], b"").expect("0 x\n", SELECTED);
}

// ---------------------------------------------------------------------------
// Large output
// ---------------------------------------------------------------------------

/// About `megabytes` of text in which every line matches `line`.
fn bulk(megabytes: usize) -> (TempDir, usize) {
    let dir = TempDir::new();
    let line = "line of text that is selected and printed again and again\n";
    let lines_per_file = megabytes * (1 << 20) / 8 / line.len();
    for file in 0..8 {
        dir.write(&format!("bulk/{file}.txt"), line.repeat(lines_per_file));
    }
    (dir, lines_per_file * 8)
}

#[test]
fn a_large_output_arrives_complete() {
    let (dir, lines) = bulk(8);
    let run = run_in(dir.path(), &["-j", "4", "line", "bulk"], b"");
    assert_eq!(run.code, Some(SELECTED));
    assert_eq!(run.stderr, "");
    assert_eq!(run.stdout.lines().count(), lines, "lines of output");
    let mut per_file = [0_usize; 8];
    let mut last_file = 0;
    for line in run.stdout.lines() {
        let (name, text) = line.split_once(':').expect("every line has a name");
        assert_eq!(
            text,
            "line of text that is selected and printed again and again"
        );
        let file: usize = name
            .strip_prefix("bulk/")
            .and_then(|n| n.strip_suffix(".txt"))
            .and_then(|n| n.parse().ok())
            .expect("a file name");
        assert!(
            file >= last_file,
            "output of {name} after output of bulk/{last_file}.txt"
        );
        last_file = file;
        per_file[file] += 1;
    }
    assert_eq!(per_file, [lines / 8; 8]);
}

#[test]
fn a_reader_that_stops_early_ends_the_program_quietly() {
    // SPEC 7.7, as in `rgx line bulk | head -1`: the program writes into a
    // pipe, the reader takes one line and closes its end.
    let (dir, _) = bulk(8);
    for threads in ["1", "4"] {
        let mut child = spawn(dir.path(), &["-j", threads, "line", "bulk"]);
        drop(child.0.stdin.take());
        let stderr = child.0.stderr.take().expect("stderr is piped");
        let stderr = std::thread::spawn(move || {
            // Nothing is expected here; a megabyte is enough to show what
            // came instead.
            let mut text = String::new();
            let _ = stderr.take(1 << 20).read_to_string(&mut text);
            text
        });
        // Read the first line on a helper thread, with a limit: a program
        // that never writes anything must fail this test, not hang it.
        let stdout = child.0.stdout.take().expect("stdout is piped");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // `take`: a first "line" without an end must not fill the memory.
            let mut stdout = BufReader::new(stdout).take(1 << 16);
            let mut first = String::new();
            let _ = stdout.read_line(&mut first);
            // The receiver gets the line and, with it, the pipe: the read
            // end stays open until the test has looked at the line.
            let _ = sender.send((first, stdout));
        });
        let Ok((first, stdout)) = receiver.recv_timeout(RUN_LIMIT) else {
            panic!(
                "TIMEOUT: with -j {threads}, rgx had not written one line of output after {} seconds",
                RUN_LIMIT.as_secs()
            );
        };
        assert_eq!(
            first, "bulk/0.txt:line of text that is selected and printed again and again\n",
            "with -j {threads}: the first line that rgx wrote into the pipe"
        );
        // Closing the read end makes the program's next write fail.
        drop(stdout);

        let Some(status) = wait_with_limit(&mut child.0, RUN_LIMIT) else {
            panic!(
                "TIMEOUT: with -j {threads}, rgx was still running {} seconds after its reader had gone away",
                RUN_LIMIT.as_secs()
            );
        };
        let stderr = stderr.join().expect("reader thread");
        assert_eq!(
            stderr, "",
            "with -j {threads}: a closed pipe is not an error to report (SPEC 7.7)"
        );
        assert_ne!(
            status.code(),
            Some(101),
            "with -j {threads}: rgx panicked when its output was closed"
        );
    }
}

// ---------------------------------------------------------------------------
// A measurement, not a test
// ---------------------------------------------------------------------------

/// Prints how long your program needs for a fixed job with 1, 2, 4 and 8
/// threads. No number makes it fail; RUBRIC.md says what to make of them.
///
/// ```text
/// cargo test --release --test m6_parallel -- --ignored --nocapture speed_report
/// ```
#[test]
#[ignore = "a measurement for RUBRIC.md: run it by name, with --release and --nocapture"]
fn speed_report() {
    let dir = TempDir::new();
    let mut megabytes = 0.0;
    for file in 0..64 {
        let mut text = String::new();
        for line in 0..12_000 {
            let _ = writeln!(
                text,
                "2024-05-{:02} host{file} service[{line}]: request {} took {} ms",
                1 + line % 28,
                line * 31 % 977,
                line % 250
            );
        }
        megabytes += text.len() as f64 / (1 << 20) as f64;
        dir.write(&format!("logs/{file:02}.log"), text);
    }
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("\nspeed_report: {megabytes:.1} MiB in 64 files; this machine offers {cores} threads");
    for pattern in [
        "took 249 ms",
        r"request [0-9]*7 took [12]?[0-9] ms$",
        r"(host|node)[0-9]+ \w+\[1?[0-9]\]",
        "no such text|^$",
    ] {
        println!("  pattern {pattern:?}");
        let mut reference: Option<(String, Duration)> = None;
        for threads in ["1", "2", "4", "8"] {
            // The best of three runs: the least disturbed one.
            let mut best = Duration::MAX;
            let mut output = String::new();
            for _ in 0..3 {
                let start = Instant::now();
                let run = run_in(dir.path(), &["-c", "-j", threads, pattern, "logs"], b"");
                best = best.min(start.elapsed());
                output = run.stdout;
            }
            let (expected, single) = reference.get_or_insert_with(|| (output.clone(), best));
            assert_eq!(
                &output, expected,
                "the output with -j {threads} differs from the output with -j 1"
            );
            println!(
                "    -j {threads}: {:7.1} ms  {:7.1} MiB/s  {:4.2} x the speed of -j 1",
                best.as_secs_f64() * 1000.0,
                megabytes / best.as_secs_f64(),
                single.as_secs_f64() / best.as_secs_f64()
            );
        }
    }
}
