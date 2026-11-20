//! LESSON 08 — Capstone: tune a slow log summary until it meets its budgets
//! Read first: CONCEPTS.md, "Lesson 08". Start after: lessons 01 to 07.
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l08_
//! Bench: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone
//! Usage: cargo run --release --manifest-path mastery/m7_perf/Cargo.toml --example usage
//! Lab: LABS.md, "Lab 8" is the measuring half of this lesson.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then read `baseline::summarize` and write down,
//! in NOTES.md, the three changes that you expect to matter most, in order.
//! Check that list against a measurement before you change any code.
//! IMPLEMENT: `summarize` and `top_k` below start as calls of the supplied
//! baseline, which is correct and slow. Give them your own bodies. The
//! behavior is fixed by the baseline: for every input, your output must be
//! the same bytes. The budgets are stated in the contracts.
//! There are no signatures to fill in beyond these two. Helper types, the
//! parsing, the data structure for the services, and the way you keep the
//! slowest requests are your design.
//! Work in the loop that this module has practised: measure (the usage
//! example and the benchmark), find the largest cost (LABS.md, Lab 2, profiles
//! this very program), change one thing, run the tests, measure again.
//! EXPLAIN afterward: Which change gave the largest gain in allocations, which
//! in peak memory, and which in time on your machine? Was the order what you
//! had predicted? Name one thing in the baseline that looks slow and that you
//! left alone, and say why.
//! JUDGMENT: Stop when the budgets are met and the profile is flat. The last
//! 5% usually costs readability. Keep the baseline, or a simple version like
//! it, as a test oracle: a differential test against obviously correct code
//! is what makes aggressive tuning safe.

use std::fmt::Write as _;
use std::io::{self, Read, Write};

/// The supplied program: correct, simple and slow. Do not edit this module.
/// The tests compare your output with its output, and the benchmark compares
/// your time with its time.
pub mod baseline {
    use std::io::{self, Read, Write};

    struct Request {
        status: u16,
        latency_ms: u32,
        bytes: u64,
    }

    /// Reads an access log and writes a summary; see [`super::summarize`]
    /// for the format.
    pub fn summarize<R: Read, W: Write>(mut input: R, mut out: W, top: usize) -> io::Result<()> {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();

        let mut services: Vec<(String, Vec<Request>)> = Vec::new();
        let mut latencies: Vec<u32> = Vec::new();
        let mut skipped = 0;
        for line in &lines {
            let fields: Vec<String> = line
                .split_whitespace()
                .map(|field| field.to_string())
                .collect();
            if fields.len() != 4 {
                skipped += 1;
                continue;
            }
            let (Ok(status), Ok(latency_ms), Ok(bytes)) = (
                fields[1].parse::<u16>(),
                fields[2].parse::<u32>(),
                fields[3].parse::<u64>(),
            ) else {
                skipped += 1;
                continue;
            };
            let request = Request {
                status,
                latency_ms,
                bytes,
            };
            match services.iter_mut().find(|(name, _)| *name == fields[0]) {
                Some((_, requests)) => requests.push(request),
                None => services.push((fields[0].clone(), vec![request])),
            }
            latencies.push(latency_ms);
        }

        services.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, requests) in &services {
            let count = requests.len();
            let errors = requests.iter().filter(|r| r.status >= 500).count();
            let bytes: u64 = requests.iter().map(|r| r.bytes).sum();
            let max_ms = requests.iter().map(|r| r.latency_ms).max().unwrap_or(0);
            let line =
                format!("{name} requests={count} errors={errors} bytes={bytes} max_ms={max_ms}\n");
            out.write_all(line.as_bytes())?;
        }
        out.write_all(b"slowest_ms:")?;
        for latency in top_k(latencies, top) {
            out.write_all(format!(" {latency}").as_bytes())?;
        }
        out.write_all(b"\n")?;
        out.write_all(format!("skipped={skipped}\n").as_bytes())?;
        Ok(())
    }

    /// The `k` largest items, largest first.
    pub fn top_k<T: Ord>(items: impl IntoIterator<Item = T>, k: usize) -> Vec<T> {
        let mut all: Vec<T> = items.into_iter().collect();
        all.sort();
        all.reverse();
        all.truncate(k);
        all
    }
}

/// A deterministic access log of `lines` lines for tests, benchmarks and
/// profiles. Supplied. About 4% of the lines are malformed on purpose.
pub fn sample_log(lines: usize, seed: u64) -> String {
    const SERVICES: [&str; 12] = [
        "auth",
        "billing",
        "cart",
        "catalog",
        "checkout",
        "email",
        "gateway",
        "inventory",
        "orders",
        "payments",
        "search",
        "shipping",
    ];
    // A linear congruential generator: the same seed gives the same log on
    // every machine.
    let mut state = seed.wrapping_mul(2).wrapping_add(1);
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let mut log = String::with_capacity(lines * 28);
    for index in 0..lines {
        let service = SERVICES[next() as usize % SERVICES.len()];
        let status = match next() % 100 {
            0..=89 => 200,
            90..=95 => 404,
            96..=97 => 500,
            _ => 503,
        };
        // Most requests are fast; one in fifty is in the slow tail.
        let (roll, tail) = (next(), next());
        let latency = if roll % 50 == 0 {
            500 + tail % 9_000
        } else {
            1 + roll % 120
        };
        let bytes = 100 + next() % 50_000;
        // Writing to a `String` cannot fail.
        let _ = match index % 97 {
            13 => writeln!(log, "this is not a log line at all"),
            41 => writeln!(log, "{service} {status} fast {bytes}"),
            60 => writeln!(log, "{service} {status} {latency}"),
            77 => writeln!(log),
            _ => writeln!(log, "{service} {status} {latency} {bytes}"),
        };
    }
    log
}

/// Read an access log from `input` and write a summary to `out`.
///
/// **Input.** One request per line: `service status latency_ms bytes`,
/// separated by white space as `str::split_whitespace` defines it (that is
/// more than the space and the tab), for example `checkout 200 37 5120`.
/// `status` is a `u16`, `latency_ms` a `u32`, `bytes` a `u64`. A line that
/// does not have exactly four fields, or whose numbers do not parse, is
/// *skipped*; this includes empty lines. The input is valid UTF-8.
///
/// **Output.** One line per service, sorted by service name:
/// `name requests=N errors=E bytes=B max_ms=M`, where `E` counts the requests
/// with a status of 500 or above, `B` is the sum of `bytes`, and `M` is the
/// largest latency. Then one line `slowest_ms:` followed by the `top` largest
/// latencies of the whole log, largest first, each preceded by a space. Then
/// one line `skipped=S`. `baseline::summarize` is the reference: your output
/// must be byte for byte the same.
///
/// **Budgets.** For a log of `b` bytes with `s` different services:
/// - at most `4 * s + 16` allocations, however many lines the log has;
/// - at most 128 KiB of heap memory in use at any moment (when `top` is small);
/// - at most `b / 4096 + 4` calls of `input.read`;
/// - at most `r / 4096 + 2` calls of `out.write` for a report of `r` bytes;
/// - when the function returns `Ok`, `out.flush()` has succeeded, and every
///   error of `input` or `out` comes back to the caller.
///
/// `top` comes from a command line. It may be 0, and it may be far larger
/// than the number of lines.
pub fn summarize<R: Read, W: Write>(input: R, out: W, top: usize) -> io::Result<()> {
    baseline::summarize(input, out, top)
}

/// The `k` largest items of `items`, largest first. Equal items may come in
/// any order. `k` may be 0, and it may be far larger than the number of items.
///
/// **Budgets.** The caller's sequence can be long and `k` is small:
/// - memory in proportion to `k`, not to the number of items: at most
///   `64 * k * size_of::<T>()` bytes requested from the allocator in total;
/// - for `n` items in random order and `k = 8`, at most `3 * n` comparisons.
///   (Sorting everything needs about `n * log2(n)`.)
pub fn top_k<T: Ord>(items: impl IntoIterator<Item = T>, k: usize) -> Vec<T> {
    baseline::top_k(items, k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::l07_io::{Sink, Source};
    use crate::support::alloc::measure;
    use std::cell::Cell;
    use std::cmp::Ordering;

    fn run(summarize: impl Fn(&[u8], &mut Vec<u8>) -> io::Result<()>, log: &str) -> String {
        let mut out = Vec::new();
        summarize(log.as_bytes(), &mut out).expect("summarize failed on valid input");
        String::from_utf8(out).expect("the report is text")
    }

    fn expect_same(log: &str, top: usize) {
        let expected = run(|input, out| baseline::summarize(input, out, top), log);
        let actual = run(|input, out| summarize(input, out, top), log);
        assert_eq!(
            actual,
            expected,
            "your report (left) differs from the baseline's (right) for top = {top} and a log \
             of {} bytes that starts with {:?}",
            log.len(),
            log.lines().next().unwrap_or("")
        );
    }

    /// An item that counts how often it is compared.
    struct Counted<'a> {
        value: u32,
        comparisons: &'a Cell<u64>,
    }

    impl PartialEq for Counted<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.cmp(other) == Ordering::Equal
        }
    }
    impl Eq for Counted<'_> {}
    impl PartialOrd for Counted<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Counted<'_> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.comparisons.set(self.comparisons.get() + 1);
            self.value.cmp(&other.value)
        }
    }

    /// `n` different numbers in a fixed pseudo-random order.
    fn shuffled(n: u32) -> Vec<u32> {
        // 7919 is a prime that does not divide `n` in the tests below, so
        // this visits every number below `n` exactly once.
        (0..n).map(|i| (i * 7_919 + 13) % n).collect()
    }

    #[test]
    fn a_small_reports_match_the_format() {
        let log = "web 200 10 100\napi 500 250 40\nweb 503 7 1\nbroken line\nweb 404 99 9\n";
        let report = run(|input, out| summarize(input, out, 2), log);
        assert_eq!(
            report,
            "api requests=1 errors=1 bytes=40 max_ms=250\n\
             web requests=3 errors=1 bytes=110 max_ms=99\n\
             slowest_ms: 250 99\n\
             skipped=1\n"
        );
        assert_eq!(
            run(|input, out| summarize(input, out, 3), ""),
            "slowest_ms:\nskipped=0\n"
        );
    }

    #[test]
    fn a_same_output_as_the_baseline() {
        for (lines, seed) in [(0, 1), (1, 2), (10, 3), (500, 4), (3_000, 5)] {
            let log = sample_log(lines, seed);
            for top in [0, 1, 5, 100, usize::MAX] {
                expect_same(&log, top);
            }
        }
    }

    #[test]
    fn a_awkward_input_is_handled_like_the_baseline() {
        for log in [
            "no newline at the end 1",
            "a 200 1 1",
            "a 200 1 1\r\nb 200 2 2\r\n",
            "\n\n\n",
            "  padded   200\t5   6  \n",
            "a 200 1 1 extra\n",
            "a 99999 1 1\na 200 -1 1\na 200 1 1.5\n",
            "a +200 +1 +1\n",
            "é 200 1 1\nß 500 2 2\n",
            // White space that is not a space or a tab: a vertical tab, a
            // no-break space and an em space separate fields too.
            "a 200\u{b}1 1\nb\u{a0}200 2 2\nc 200 3\u{2003}3\n",
            "a 200 4294967295 18446744073709551615\n",
            "a 200 1 1\n\na 200 2 2",
        ] {
            expect_same(log, 3);
        }
        // Lines that are split across reads must come out the same.
        let log = sample_log(200, 9);
        let expected = run(|input, out| baseline::summarize(input, out, 4), &log);
        for chunk in [1, 5, 64] {
            let mut out = Vec::new();
            summarize(Source::new(log.as_bytes(), chunk), &mut out, 4).unwrap();
            assert_eq!(String::from_utf8(out).unwrap(), expected, "chunk {chunk}");
        }
    }

    #[test]
    fn b_allocations_do_not_grow_with_the_log() {
        for lines in [20_000, 80_000] {
            let log = sample_log(lines, 11);
            // The report goes to a writer that keeps nothing, so that only
            // your allocations are counted.
            let (result, usage) = measure(|| summarize(log.as_bytes(), io::sink(), 5));
            result.unwrap();
            let budget = 4 * 12 + 16;
            assert!(
                usage.allocations <= budget,
                "summarize made {} allocations for {lines} lines and 12 services; the \
                 budget is {budget} for any number of lines. Something still allocates \
                 per line",
                usage.allocations
            );
        }
    }

    #[test]
    fn b_memory_stays_small_while_the_log_streams_through() {
        let log = sample_log(40_000, 12);
        assert!(log.len() > 800_000, "the sample log is about a megabyte");
        let (result, usage) = measure(|| summarize(log.as_bytes(), io::sink(), 5));
        result.unwrap();
        assert!(
            usage.peak <= 128 * 1024,
            "summarize held {} bytes of heap memory at its peak for a log of {} bytes; the \
             budget is 131,072. Do not keep the text, the lines or the requests: keep \
             running totals",
            usage.peak,
            log.len()
        );
    }

    #[test]
    fn b_reads_and_writes_are_batched() {
        let log = sample_log(20_000, 13);
        let bytes = log.len();
        let mut source = Source::new(log, 64 * 1024);
        let mut sink = Sink::new();
        summarize(&mut source, &mut sink, 5).unwrap();
        assert!(
            source.reads <= bytes / 4096 + 4,
            "summarize called `read` {} times for {bytes} bytes; the budget is {}",
            source.reads,
            bytes / 4096 + 4
        );
        let report = sink.data.len();
        assert!(
            sink.writes <= report / 4096 + 2,
            "summarize called `write` {} times for a report of {report} bytes; the budget \
             is {}",
            sink.writes,
            report / 4096 + 2
        );
        assert!(
            sink.flushes >= 1,
            "summarize must flush `out` before it reports success"
        );
    }

    #[test]
    fn b_output_errors_are_reported() {
        let log = sample_log(50, 14);
        let mut sink = Sink {
            fail_write: Some((1, io::ErrorKind::BrokenPipe)),
            ..Sink::new()
        };
        let error = summarize(log.as_bytes(), &mut sink, 5)
            .expect_err("the first write to `out` failed; returning Ok would hide a lost report");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

        let mut sink = Sink {
            fail_flush: true,
            ..Sink::new()
        };
        assert!(
            summarize(log.as_bytes(), &mut sink, 5).is_err(),
            "an error of `out.flush()` must come back to the caller"
        );
    }

    #[test]
    fn c_top_k_matches_the_baseline() {
        for n in [0u32, 1, 2, 9, 100] {
            for k in [0usize, 1, 2, 8, 99, 100, 101, usize::MAX] {
                let items = shuffled(n);
                assert_eq!(
                    top_k(items.clone(), k),
                    baseline::top_k(items, k),
                    "n = {n}, k = {k}"
                );
            }
        }
        assert_eq!(
            top_k([3, 1, 3, 2, 3], 4),
            [3, 3, 3, 2],
            "equal items all count"
        );
        assert_eq!(top_k(["b", "c", "a"], 2), ["c", "b"]);
    }

    #[test]
    fn c_top_k_keeps_only_k_items() {
        let n = 10_000u32;
        let (top, usage) = measure(|| top_k(shuffled(n).into_iter().map(u64::from), 8));
        assert_eq!(
            top,
            [9_999, 9_998, 9_997, 9_996, 9_995, 9_994, 9_993, 9_992]
        );
        // `shuffled` itself allocates one Vec of 40,000 bytes inside the
        // measurement; everything above that is yours.
        let yours = usage.bytes - 40_000;
        let budget = 64 * 8 * size_of::<u64>();
        assert!(
            yours <= budget,
            "top_k asked the allocator for {yours} bytes to find the 8 largest of 10,000 \
             items; the budget is {budget}. Do not collect the items: keep only the best \
             8 seen so far"
        );
    }

    #[test]
    fn c_top_k_compares_each_item_about_once() {
        let n = 10_000u32;
        let comparisons = Cell::new(0);
        let items = shuffled(n).into_iter().map(|value| Counted {
            value,
            comparisons: &comparisons,
        });
        let top: Vec<u32> = top_k(items, 8).iter().map(|item| item.value).collect();
        assert_eq!(
            top,
            [9_999, 9_998, 9_997, 9_996, 9_995, 9_994, 9_993, 9_992]
        );
        let budget = 3 * u64::from(n);
        assert!(
            comparisons.get() <= budget,
            "top_k made {} comparisons for 10,000 items in random order; the budget is \
             {budget}. Most items are smaller than the smallest of the best 8: one \
             comparison with that one item should reject them",
            comparisons.get()
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// The questions are about the supplied baseline, not about your code.
    #[test]
    fn p_predictions() {
        // P1. Read `baseline::summarize`. For a log of 1,000 well-formed
        // lines, how many allocations does it make per line? Count what
        // happens for every line, ignore what happens only now and then, and
        // answer with a whole number.
        let log: String = (0..1_000)
            .map(|i| format!("svc{} 200 {} {}\n", i % 4, i % 50, i))
            .collect();
        let (result, usage) = measure(|| baseline::summarize(log.as_bytes(), io::sink(), 5));
        result.unwrap();
        let p1: usize = guess();
        assert_eq!(
            p1,
            usage.allocations / 1_000,
            "P1: allocations per line in the baseline, rounded down (left: your prediction, right: actual)"
        );

        // P2. The same run with a writer that counts: 4 services, top = 5.
        // How many times does the baseline call `write`?
        let mut sink = Sink::new();
        baseline::summarize(log.as_bytes(), &mut sink, 5).unwrap();
        let p2: usize = guess();
        assert_eq!(
            p2, sink.writes,
            "P2: write calls made by the baseline (left: your prediction, right: actual)"
        );

        // P3. `baseline::top_k` sorts everything. How many comparisons does
        // it make for 1,000 items that are already in ascending order? And
        // is that more than 5,000 for the same items in shuffled order?
        let comparisons = Cell::new(0);
        let counted = |values: Vec<u32>| {
            comparisons.set(0);
            let items = values.into_iter().map(|value| Counted {
                value,
                comparisons: &comparisons,
            });
            let top = baseline::top_k(items, 8);
            assert_eq!(top[0].value, 999);
            comparisons.get()
        };
        let in_order = counted((0..1_000).collect());
        let shuffled_order = counted(shuffled(1_000));
        let p3: (u64, bool) = guess();
        assert_eq!(
            p3,
            (in_order, shuffled_order > 5_000),
            "P3: (comparisons for sorted input, more than 5,000 for shuffled input?) (left: your prediction, right: actual)"
        );
    }
}
