//! LESSON 07 — Buffering: fewer, larger reads and writes
//! Read first: CONCEPTS.md, "Lesson 07". Start after: lesson 03; chapter 18
//! (`Read`, `Write`, `BufRead`) and chapter 13 (`Drop`).
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a program prints 100,000 short
//! lines with `println!`. How many times does it ask the operating system to
//! write, and what changes when you wrap `stdout().lock()` in a `BufWriter`?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! The tests never touch a file. They use the supplied `Sink` and `Source`,
//! which count every call of `write`, `flush` and `read`, and which can be
//! told to accept only a few bytes or to fail. Each such call stands for a
//! system call, the expensive step that buffering exists to avoid.
//! EXPLAIN afterward: Why must `write_table` call `flush` itself although
//! `BufWriter` flushes when it is dropped? In `Batcher`, why can you not use
//! `write_all` to empty the buffer, and what must be true of the buffer after
//! a failed write? Why does `sum_lines` reuse one `String`?
//! JUDGMENT: Use `BufReader` and `BufWriter` around every file, pipe and
//! socket that you read or write in small pieces; the default 8 KiB is a good
//! size. Do not put a buffer around something that is already in memory (a
//! `Vec<u8>`, a `&[u8]`), and do not stack two buffers. When you buffer
//! output that a person or another program waits for, decide where to flush:
//! per message for a protocol, per line for a terminal, once at the end for a
//! file. Write a buffer type yourself only to learn how one works, as in C.

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

// ---------------------------------------------------------------------------
// Instruments. Supplied; read them, do not edit them.
// ---------------------------------------------------------------------------

/// A writer that keeps what it receives, counts the calls, and misbehaves on
/// request.
#[derive(Debug, Default)]
pub struct Sink {
    /// Every byte that was accepted, in order.
    pub data: Vec<u8>,
    /// The number of bytes that each successful `write` call accepted.
    pub accepted: Vec<usize>,
    /// Calls of `write`, including the failed ones.
    pub writes: usize,
    /// Calls of `flush`, including the failed ones.
    pub flushes: usize,
    /// Accept at most this many bytes per call (a *short write*).
    pub limit: Option<usize>,
    /// Accept at most this many bytes in total; after that, `write` returns
    /// `Ok(0)`, like a device that is full.
    pub room: Option<usize>,
    /// The `write` call with this number (the first call is number 1) fails
    /// with this kind of error and accepts nothing.
    pub fail_write: Option<(usize, io::ErrorKind)>,
    /// Every `flush` call fails.
    pub fail_flush: bool,
}

impl Sink {
    pub fn new() -> Sink {
        Sink::default()
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

impl Write for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if let Some((call, kind)) = self.fail_write
            && call == self.writes
        {
            return Err(io::Error::new(kind, "the sink was told to fail this write"));
        }
        let mut n = data.len().min(self.limit.unwrap_or(usize::MAX));
        if let Some(room) = self.room {
            n = n.min(room - self.data.len());
        }
        self.data.extend_from_slice(&data[..n]);
        self.accepted.push(n);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            return Err(io::Error::other("the sink was told to fail this flush"));
        }
        Ok(())
    }
}

/// A reader that hands out its bytes in pieces and counts the calls.
#[derive(Debug)]
pub struct Source {
    data: Vec<u8>,
    position: usize,
    /// Hand out at most this many bytes per call.
    chunk: usize,
    /// Calls of `read`, including the last one that reports the end.
    pub reads: usize,
}

impl Source {
    pub fn new(data: impl Into<Vec<u8>>, chunk: usize) -> Source {
        assert!(chunk > 0, "a read must be able to return something");
        Source {
            data: data.into(),
            position: 0,
            chunk,
            reads: 0,
        }
    }
}

impl Read for Source {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        let rest = &self.data[self.position..];
        let n = rest.len().min(buffer.len()).min(self.chunk);
        buffer[..n].copy_from_slice(&rest[..n]);
        self.position += n;
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Your tasks.
// ---------------------------------------------------------------------------

/// A. Write one line per row to `out`: the id, a tab, the name, and `\n`.
///
/// Contract:
/// - `out` is slow: every call of `out.write` costs a system call. For a
///   table of `b` bytes, make at most `b / 4096 + 2` such calls.
/// - Do not build a `String` per row; at most two allocations in total.
/// - When this function returns `Ok`, every byte has been handed to `out` and
///   `out.flush()` has succeeded. Any error of `out`, in `write` or in
///   `flush`, comes back to the caller.
pub fn write_table<W: Write>(out: W, rows: &[(u32, &str)]) -> io::Result<()> {
    let mut out = BufWriter::new(out);
    for (id, name) in rows {
        // `write!` formats straight into the buffer of the `BufWriter`.
        writeln!(out, "{id}\t{name}")?;
    }
    // Dropping a `BufWriter` also writes what is left, but it has to ignore
    // an error. Only an explicit flush can report one.
    out.flush()
}

/// B. `input` holds one unsigned decimal number per line. Return their sum.
///
/// Contract:
/// - White space around a number (spaces, tabs, `\r`) is ignored, and so are
///   lines that hold nothing else; `str::trim` does this. The last line may
///   lack its `\n`.
/// - A line that `str::parse::<u64>` does not accept gives an error of kind
///   `InvalidData`. Errors of `input` come back unchanged. The sum fits in a
///   `u64`.
/// - `input` is slow: for `b` bytes, call `input.read` at most
///   `b / 4096 + 4` times.
/// - The input can be far larger than memory. Hold at most 128 KiB of heap
///   memory at any moment, and make at most 8 allocations, however many lines
///   there are.
pub fn sum_lines<R: Read>(input: R) -> io::Result<u64> {
    let mut reader = BufReader::new(input);
    // One `String` serves every line: `clear` keeps its capacity.
    let mut line = String::new();
    let mut sum = 0u64;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(sum);
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let number: u64 = text
            .parse()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        sum += number;
    }
}

/// C. A buffered writer that you build yourself: it collects small writes and
/// hands them to the inner writer in large pieces.
///
/// Contract:
/// - `Batcher::with_capacity(capacity, inner)` allocates a buffer of
///   `capacity` bytes. The buffer never grows, and no call that succeeds
///   allocates anything else.
/// - `write(data)`:
///   1. if `data` does not fit into the free part of the buffer, first
///      *drain* the buffer (see below); an error ends the call, and then no
///      byte of `data` has been taken;
///   2. then, if `data.len() >= capacity`, pass `data` to `inner.write` in one
///      call and return what that call returns;
///   3. otherwise copy `data` into the buffer and return `Ok(data.len())`.
/// - *Draining* hands the buffered bytes to `inner.write`, in order, until
///   the buffer is empty. `inner.write` may accept fewer bytes than offered
///   (call it again with the rest), may fail with `ErrorKind::Interrupted`
///   (try again), and may return `Ok(0)` (stop with an error of kind
///   `WriteZero`). When draining stops with an error, the bytes that `inner`
///   accepted are gone from the buffer and all the others are still in it, in
///   order: a later drain continues exactly where this one stopped, so no
///   byte is lost and none is written twice.
/// - `flush()` drains and then calls `inner.flush()`.
/// - `buffered()` is the number of bytes in the buffer.
/// - `into_inner()` drains and returns the inner writer. If draining fails,
///   it returns the error.
/// - Dropping a `Batcher` drains once and ignores an error. After a
///   successful `into_inner` nothing is written again.
///
/// The fields are yours; the placeholder below only makes the file compile.
pub struct Batcher<W: Write> {
    /// `None` only after `into_inner` has moved the writer out. A type with
    /// a `Drop` impl cannot give up a field in any other way.
    inner: Option<W>,
    /// The buffered bytes. Its capacity is fixed: nothing is ever pushed
    /// beyond `capacity`, so the `Vec` never grows.
    buffer: Vec<u8>,
    capacity: usize,
}

impl<W: Write> Batcher<W> {
    pub fn with_capacity(capacity: usize, inner: W) -> Batcher<W> {
        Batcher {
            inner: Some(inner),
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    pub fn into_inner(mut self) -> io::Result<W> {
        // On an error `self` is dropped here, and `drop` tries once more.
        self.drain()?;
        Ok(self
            .inner
            .take()
            .expect("the writer is present until into_inner takes it"))
    }

    fn inner(&mut self) -> &mut W {
        // `into_inner` consumes the `Batcher`, so no method can run after it
        // has taken the writer; only `drop` sees `None`.
        self.inner
            .as_mut()
            .expect("the writer is present until into_inner takes it")
    }

    /// Hands the buffered bytes to the inner writer. On an error, exactly the
    /// bytes that were not accepted stay in the buffer.
    fn drain(&mut self) -> io::Result<()> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(());
        };
        // `write_all` would be shorter, but after an error it cannot say how
        // many bytes got through. Count them here.
        let mut written = 0;
        let result = loop {
            if written == self.buffer.len() {
                break Ok(());
            }
            match inner.write(&self.buffer[written..]) {
                // An error made from a kind alone needs no heap memory;
                // `io::Error::new` with a message would allocate.
                Ok(0) => break Err(io::ErrorKind::WriteZero.into()),
                Ok(n) => written += n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => break Err(error),
            }
        };
        // Runs on success and on failure: remove what was accepted, keep the
        // rest at the front for the next attempt.
        self.buffer.drain(..written);
        result
    }
}

impl<W: Write> Write for Batcher<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.capacity - self.buffer.len() {
            self.drain()?;
        }
        if data.len() >= self.capacity {
            // Too large to gain anything from a copy into the buffer. The
            // buffer is empty here, so the order of the bytes is kept.
            self.inner().write(data)
        } else {
            self.buffer.extend_from_slice(data);
            Ok(data.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain()?;
        self.inner().flush()
    }
}

impl<W: Write> Drop for Batcher<W> {
    fn drop(&mut self) {
        // The last chance to deliver the bytes. There is nobody to report an
        // error to, which is why callers should flush before they drop.
        // (`std::io::BufWriter` goes one step further: it remembers when the
        // inner writer panicked in the middle of a write, and then skips this
        // step, so that no byte is offered twice during unwinding.)
        let _ = self.drain();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::measure;
    use std::cell::RefCell;
    use std::io::LineWriter;
    use std::rc::Rc;

    /// A `Sink` that a test can still look at while a writer owns a handle.
    #[derive(Clone, Default)]
    struct Shared(Rc<RefCell<Sink>>);

    impl Write for Shared {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().write(data)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.borrow_mut().flush()
        }
    }

    fn rows(n: u32) -> Vec<(u32, &'static str)> {
        let names = ["ada", "grace hopper", "", "linus", "barbara"];
        (0..n).map(|id| (id, names[id as usize % 5])).collect()
    }

    #[test]
    fn a_table_is_written() {
        let mut sink = Sink::new();
        write_table(&mut sink, &[(1, "ada"), (20, ""), (300, "a\tb")]).unwrap();
        assert_eq!(sink.text(), "1\tada\n20\t\n300\ta\tb\n");
        let mut empty = Sink::new();
        write_table(&mut empty, &[]).unwrap();
        assert_eq!(empty.text(), "");
    }

    #[test]
    fn a_few_large_writes() {
        let rows = rows(5_000);
        let mut sink = Sink::new();
        // Room for everything a wasteful solution could do, so that the
        // sink's own record keeping never counts against the budget.
        sink.data.reserve(200_000);
        sink.accepted.reserve(200_000);
        let (result, usage) = measure(|| write_table(&mut sink, &rows));
        result.unwrap();
        let allocations = usage.allocations;
        let bytes = sink.data.len();
        assert_eq!(sink.text().lines().count(), 5_000);
        assert!(
            sink.writes <= bytes / 4096 + 2,
            "write_table called `write` {} times for {bytes} bytes; the budget is {}. Put a \
             buffer between the formatting and `out`",
            sink.writes,
            bytes / 4096 + 2
        );
        assert!(
            allocations <= 2,
            "write_table made {allocations} allocations for 5,000 rows; format straight into \
             the buffered writer instead of building a String per row"
        );
        assert!(
            sink.flushes >= 1,
            "write_table must flush `out` before it reports success"
        );
    }

    #[test]
    fn a_errors_are_reported_not_dropped() {
        // The whole table fits into any buffer, so the failing write happens
        // only when the buffer is emptied at the end.
        let mut sink = Sink {
            fail_write: Some((1, io::ErrorKind::BrokenPipe)),
            ..Sink::new()
        };
        let error = write_table(&mut sink, &rows(3)).expect_err(
            "the only write failed, so no byte arrived; returning Ok would hide the loss. \
             A buffered writer that is merely dropped cannot report this",
        );
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

        let mut sink = Sink {
            fail_flush: true,
            ..Sink::new()
        };
        assert!(
            write_table(&mut sink, &rows(3)).is_err(),
            "an error of `out.flush()` must come back to the caller"
        );
        assert_eq!(sink.text().lines().count(), 3);
    }

    #[test]
    fn b_numbers_are_added() {
        let sum = |text: &str| sum_lines(text.as_bytes());
        assert_eq!(sum("").unwrap(), 0);
        assert_eq!(sum("7").unwrap(), 7);
        assert_eq!(sum("1\n2\n3\n").unwrap(), 6);
        assert_eq!(sum("10\r\n  20 \r\n\n\t\n30").unwrap(), 60);
        assert_eq!(sum("18446744073709551615\n0\n").unwrap(), u64::MAX);
        for bad in ["1\nx\n3\n", "-4\n", "1 2\n", "99999999999999999999999\n"] {
            let error = sum(bad).expect_err("not a number");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{bad:?}");
        }
    }

    #[test]
    fn b_lines_may_span_reads() {
        let text: String = (1..=2_000).map(|n| format!("{n}\n")).collect();
        for chunk in [1, 7, 4096] {
            let mut source = Source::new(text.as_bytes(), chunk);
            assert_eq!(sum_lines(&mut source).unwrap(), 2_001_000, "chunk {chunk}");
        }
    }

    #[test]
    fn b_large_input_within_the_budgets() {
        // About one mebibyte: 150,000 lines of seven bytes.
        let text = "123456\n".repeat(150_000);
        let bytes = text.len();
        let mut source = Source::new(text, 64 * 1024);
        let (sum, usage) = measure(|| sum_lines(&mut source));
        assert_eq!(sum.unwrap(), 123_456 * 150_000);
        assert!(
            source.reads <= bytes / 4096 + 4,
            "sum_lines called `read` {} times for {bytes} bytes; the budget is {}. Read \
             through a buffer",
            source.reads,
            bytes / 4096 + 4
        );
        assert!(
            usage.allocations <= 8,
            "sum_lines made {} allocations for 150,000 lines; the budget is 8. Reuse one \
             String for every line",
            usage.allocations
        );
        assert!(
            usage.peak <= 128 * 1024,
            "sum_lines held {} bytes of heap memory at its peak for an input of {bytes} \
             bytes; the budget is 131,072. Do not load the whole input",
            usage.peak
        );
    }

    #[test]
    fn c_small_writes_are_collected() {
        let sink = Shared::default();
        let mut batcher = Batcher::with_capacity(12, sink.clone());
        assert_eq!(batcher.write(b"aaaaa").unwrap(), 5);
        assert_eq!(batcher.write(b"bbbbb").unwrap(), 5);
        assert_eq!(batcher.buffered(), 10);
        assert_eq!(
            sink.0.borrow().writes,
            0,
            "ten bytes fit into a buffer of twelve; nothing may reach the inner writer yet"
        );
        // Five more bytes do not fit into the two that are free.
        assert_eq!(batcher.write(b"ccccc").unwrap(), 5);
        assert_eq!(
            sink.0.borrow().accepted,
            [10],
            "the buffer is emptied with one write of everything it holds"
        );
        assert_eq!(batcher.buffered(), 5);
        batcher.flush().unwrap();
        assert_eq!(sink.0.borrow().accepted, [10, 5]);
        assert_eq!(sink.0.borrow().text(), "aaaaabbbbbccccc");
        assert_eq!(
            sink.0.borrow().flushes,
            1,
            "flush must reach the inner writer"
        );
        assert_eq!(batcher.buffered(), 0);
    }

    #[test]
    fn c_large_writes_go_straight_through() {
        let sink = Shared::default();
        let mut batcher = Batcher::with_capacity(8, sink.clone());
        batcher.write_all(b"abc").unwrap();
        let big = b"0123456789ABCDEFGHIJ";
        assert_eq!(batcher.write(big).unwrap(), 20);
        assert_eq!(
            sink.0.borrow().accepted,
            [3, 20],
            "first the buffered bytes, then the large piece in one call, uncopied"
        );
        assert_eq!(batcher.buffered(), 0);
        // A piece of exactly the capacity is "large" too.
        assert_eq!(batcher.write(b"12345678").unwrap(), 8);
        assert_eq!(sink.0.borrow().accepted, [3, 20, 8]);
        assert_eq!(sink.0.borrow().text(), "abc0123456789ABCDEFGHIJ12345678");
    }

    #[test]
    fn c_a_large_write_reports_what_the_inner_writer_accepted() {
        let mut sink = Sink {
            limit: Some(6),
            ..Sink::new()
        };
        {
            let mut batcher = Batcher::with_capacity(4, &mut sink);
            assert_eq!(
                batcher.write(b"0123456789").unwrap(),
                6,
                "a large piece is passed on in one call; if the inner writer takes only \
                 six bytes, six is the answer, and the caller offers the rest again"
            );
            batcher.write_all(b"6789").unwrap();
            batcher.flush().unwrap();
        }
        assert_eq!(sink.text(), "0123456789");
    }

    #[test]
    fn c_short_writes_of_the_inner_writer_are_completed() {
        let mut sink = Sink {
            limit: Some(3),
            ..Sink::new()
        };
        {
            let mut batcher = Batcher::with_capacity(16, &mut sink);
            batcher.write_all(b"0123456789").unwrap();
            batcher.flush().unwrap();
        }
        assert_eq!(sink.text(), "0123456789");
        assert_eq!(sink.accepted, [3, 3, 3, 1]);
    }

    #[test]
    fn c_an_interrupted_write_is_tried_again() {
        let mut sink = Sink {
            fail_write: Some((1, io::ErrorKind::Interrupted)),
            ..Sink::new()
        };
        {
            let mut batcher = Batcher::with_capacity(16, &mut sink);
            batcher.write_all(b"hello").unwrap();
            batcher
                .flush()
                .expect("ErrorKind::Interrupted means \"try again\", not \"failed\"");
        }
        assert_eq!(sink.text(), "hello");
    }

    #[test]
    fn c_after_an_error_nothing_is_lost_or_repeated() {
        let sink = Shared::default();
        sink.0.borrow_mut().limit = Some(4);
        sink.0.borrow_mut().fail_write = Some((2, io::ErrorKind::TimedOut));
        let mut batcher = Batcher::with_capacity(16, sink.clone());
        batcher.write_all(b"0123456789").unwrap();
        let error = batcher.flush().expect_err("the second inner write fails");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            sink.0.borrow().text(),
            "0123",
            "the first inner write got through"
        );
        assert_eq!(
            batcher.buffered(),
            6,
            "after the failure the buffer must hold exactly the six bytes that the inner \
             writer has not accepted"
        );
        batcher.flush().expect("the sink works again");
        assert_eq!(
            sink.0.borrow().text(),
            "0123456789",
            "the second flush must continue where the first one stopped"
        );
        assert_eq!(batcher.buffered(), 0);
    }

    #[test]
    fn c_a_failed_drain_ends_the_write() {
        let sink = Shared::default();
        sink.0.borrow_mut().fail_write = Some((1, io::ErrorKind::TimedOut));
        let mut batcher = Batcher::with_capacity(8, sink.clone());
        batcher.write_all(b"aaaaa").unwrap();
        // Five more bytes do not fit, so the buffer is drained first, and
        // the sink fails that write.
        let error = batcher
            .write(b"bbbbb")
            .expect_err("the drain failed, so this write must report the error");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            batcher.buffered(),
            5,
            "after the failed write the buffer must hold the five old bytes and none of \
             the new ones: the caller was told that the new bytes were not taken"
        );
        batcher.write_all(b"bbbbb").expect("the sink works again");
        batcher.flush().unwrap();
        assert_eq!(
            sink.0.borrow().text(),
            "aaaaabbbbb",
            "every byte must arrive once, in order"
        );
    }

    #[test]
    fn c_a_full_device_is_an_error_not_a_loop() {
        let (kind, buffered, text) = crate::within(10, "flushing into a full sink", || {
            let sink = Shared::default();
            sink.0.borrow_mut().room = Some(4);
            // The batcher is dropped at the end of this closure. The drop
            // tries once more, and that attempt must end as well.
            let mut batcher = Batcher::with_capacity(16, sink.clone());
            batcher.write_all(b"0123456789").unwrap();
            let error = batcher
                .flush()
                .expect_err("the sink is full after four bytes");
            (error.kind(), batcher.buffered(), sink.0.borrow().text())
        });
        assert_eq!(
            kind,
            io::ErrorKind::WriteZero,
            "Ok(0) from the inner writer means it will never take more"
        );
        assert_eq!(buffered, 6);
        assert_eq!(text, "0123");
    }

    #[test]
    fn c_into_inner_returns_the_writer_with_everything_written() {
        let mut batcher = Batcher::with_capacity(64, Sink::new());
        batcher.write_all(b"kept in the buffer").unwrap();
        let sink = batcher.into_inner().unwrap();
        assert_eq!(sink.text(), "kept in the buffer");
        assert_eq!(sink.writes, 1);

        let failing = Sink {
            fail_write: Some((1, io::ErrorKind::BrokenPipe)),
            ..Sink::new()
        };
        let mut batcher = Batcher::with_capacity(64, failing);
        batcher.write_all(b"x").unwrap();
        assert!(batcher.into_inner().is_err());
    }

    #[test]
    fn c_dropping_writes_what_is_left_exactly_once() {
        let sink = Shared::default();
        {
            let mut batcher = Batcher::with_capacity(64, sink.clone());
            batcher.write_all(b"not flushed").unwrap();
            assert_eq!(sink.0.borrow().writes, 0);
        }
        assert_eq!(
            sink.0.borrow().text(),
            "not flushed",
            "a Batcher that is dropped must hand over what it still holds"
        );

        let sink = Shared::default();
        let mut batcher = Batcher::with_capacity(64, sink.clone());
        batcher.write_all(b"once").unwrap();
        let inner = batcher.into_inner().unwrap();
        drop(inner);
        assert_eq!(
            sink.0.borrow().text(),
            "once",
            "into_inner wrote the bytes; the drop after it must not write them again"
        );
    }

    #[test]
    fn c_the_buffer_is_the_only_allocation() {
        let (_, usage) = measure(|| {
            let mut batcher = Batcher::with_capacity(256, io::sink());
            for round in 0..5_000u32 {
                batcher.write_all(&round.to_le_bytes()).unwrap();
                if round % 100 == 0 {
                    batcher.write_all(&[0u8; 300]).unwrap();
                }
            }
            batcher.flush().unwrap();
        });
        assert!(
            usage.allocations <= 1,
            "a Batcher made {} allocations; it needs one buffer and must never grow it",
            usage.allocations
        );
        assert!(
            usage.bytes <= 256,
            "a Batcher with capacity 256 asked for {} bytes",
            usage.bytes
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// The questions are about the standard library's own buffers.
    // P4 reads byte by byte without a buffer on purpose; clippy would rightly
    // complain about that in real code.
    #[allow(clippy::unbuffered_bytes)]
    #[test]
    fn p_predictions() {
        // P1. Four writes of five bytes through a buffer of 16 bytes, then a
        // flush. How many `write` calls reach the sink, and how many bytes
        // does the first of them carry?
        let mut sink = Sink::new();
        let mut buffered = BufWriter::with_capacity(16, &mut sink);
        for piece in [b"aaaaa", b"bbbbb", b"ccccc", b"ddddd"] {
            buffered.write_all(piece).unwrap();
        }
        buffered.flush().unwrap();
        drop(buffered);
        let p1: (usize, usize) = (2, 15);
        assert_eq!(
            p1,
            (sink.writes, sink.accepted[0]),
            "P1: (write calls that reach the sink, bytes in the first one) (left: your prediction, right: actual)"
        );

        // P2. One `writeln!` with two values, sent to a writer without a
        // buffer and to one with a buffer (then flushed). How many `write`
        // calls reach each sink?
        // (`black_box` hides the two numbers from the compiler, which would
        // otherwise paste them into the text while it compiles.)
        let (low, high) = (std::hint::black_box(12), std::hint::black_box(34));
        let mut direct = Sink::new();
        writeln!(direct, "{low}-{high}").unwrap();
        let mut sink = Sink::new();
        let mut buffered = BufWriter::new(&mut sink);
        writeln!(buffered, "{low}-{high}").unwrap();
        buffered.flush().unwrap();
        drop(buffered);
        assert_eq!(direct.text(), sink.text());
        let p2: (usize, usize) = (4, 1);
        assert_eq!(
            p2,
            (direct.writes, sink.writes),
            "P2: write calls for one writeln! (without a buffer, with a buffer) (left: your prediction, right: actual)"
        );

        // P3. A `LineWriter` passes complete lines on at once and keeps an
        // unfinished line back. How many bytes have reached the sink after
        // each of the three writes?
        let sink = Shared::default();
        let mut lines = LineWriter::new(sink.clone());
        lines.write_all(b"ab").unwrap();
        let after_first = sink.0.borrow().data.len();
        lines.write_all(b"c\nde").unwrap();
        let after_second = sink.0.borrow().data.len();
        lines.write_all(b"f\n").unwrap();
        let after_third = sink.0.borrow().data.len();
        let p3: (usize, usize, usize) = (0, 4, 8);
        assert_eq!(
            p3,
            (after_first, after_second, after_third),
            "P3: bytes at the sink after each write through a LineWriter (left: your prediction, right: actual)"
        );

        // P4. 20,000 bytes are read to the end, first one byte at a time
        // straight from the source, then one byte at a time through a
        // `BufReader` (buffer size 8,192). How many `read` calls reach the
        // source in each case? Count the last call that reports the end.
        let mut source = Source::new(vec![b'x'; 20_000], usize::MAX);
        let mut count = 0;
        for byte in (&mut source).bytes() {
            byte.unwrap();
            count += 1;
        }
        let unbuffered = source.reads;
        let mut source = Source::new(vec![b'x'; 20_000], usize::MAX);
        for byte in BufReader::new(&mut source).bytes() {
            byte.unwrap();
            count += 1;
        }
        assert_eq!(count, 40_000);
        let p4: (usize, usize) = (20001, 4);
        assert_eq!(
            p4,
            (unbuffered, source.reads),
            "P4: read calls that reach the source (without a buffer, with a BufReader) (left: your prediction, right: actual)"
        );
    }
}
