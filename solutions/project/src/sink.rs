//! Milestone m11 (Chapter 11): where matching records go.
//!
//! The pipeline (m15) hands each matching record to a [`Sink`]. A sink only
//! *borrows* the record for the duration of the call: the record borrows a
//! line buffer that the reader reuses for the next line. A sink that wants to
//! keep something copies it (`Vec<String>` keeps the formatted line; the m16
//! `Summary` keeps counters and group names).

use crate::record::Record;
use std::io::{self, Write};

/// A consumer of records.
///
/// The trait is dyn-compatible: `Box<dyn Sink>` and `&mut dyn Sink` work.
pub trait Sink {
    /// Consume one record.
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()>;

    /// Called once after the last record, for example to flush output.
    /// The default does nothing.
    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// m11 E. Collects each record's canonical line (its `Display` form).
impl Sink for Vec<String> {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        self.push(record.to_string());
        Ok(())
    }
}

/// Counts the records it accepts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Count(pub u64);

/// m11 F. Adds one per record.
impl Sink for Count {
    fn accept(&mut self, _record: &Record<'_>) -> io::Result<()> {
        self.0 += 1;
        Ok(())
    }
}

/// Writes each record's canonical line, followed by `\n`, to a writer.
#[derive(Debug)]
pub struct LineWriter<W: Write> {
    out: W,
}

impl<W: Write> LineWriter<W> {
    /// Write to `out`. Wrap files and stdout in a `BufWriter` first.
    pub fn new(out: W) -> Self {
        LineWriter { out }
    }

    /// The writer.
    pub fn get_ref(&self) -> &W {
        &self.out
    }

    /// Give the writer back.
    pub fn into_inner(self) -> W {
        self.out
    }
}

/// m11 G. `accept` formats the record directly into the writer with
/// `writeln!` (no `to_string`, so no allocation per record); `finish`
/// flushes the writer.
impl<W: Write> Sink for LineWriter<W> {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        writeln!(self.out, "{record}")
    }

    fn finish(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Sends every record to two sinks, first `.0` then `.1`.
#[derive(Debug, Default)]
pub struct Tee<A, B>(pub A, pub B);

/// m11 H. `accept` stops at the first error. `finish` calls `finish` on
/// *both* sinks even if the first fails, then returns the first error.
impl<A: Sink, B: Sink> Sink for Tee<A, B> {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        self.0.accept(record)?;
        self.1.accept(record)
    }

    fn finish(&mut self) -> io::Result<()> {
        let first = self.0.finish();
        let second = self.1.finish();
        first.and(second)
    }
}

/// m11 I. A mutable reference to a sink is a sink, so callers can lend one.
impl<S: Sink + ?Sized> Sink for &mut S {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        (**self).accept(record)
    }

    fn finish(&mut self) -> io::Result<()> {
        (**self).finish()
    }
}

/// m11 I. A boxed sink, including `Box<dyn Sink>`, is a sink.
impl<S: Sink + ?Sized> Sink for Box<S> {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        (**self).accept(record)
    }

    fn finish(&mut self) -> io::Result<()> {
        (**self).finish()
    }
}
