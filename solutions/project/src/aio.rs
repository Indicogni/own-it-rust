//! Optional milestone m20 (Chapters 20–21): a std-only async reader.
//!
//! No runtime crate: you write the executor ([`block_on`]) and the future
//! that reads one line ([`NextLine`]). The source is [`Trickle`], a
//! simulated non-blocking input that is "not ready" every other time it is
//! polled, the way a socket often is. It is provided; read its `poll_read`
//! to see what a well-behaved source does before returning `Pending`.

use crate::filter::Filter;
use crate::pipeline::{Tally, handle_line};
use crate::sink::Sink;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

/// A simulated non-blocking source. Provided.
#[derive(Debug, Clone)]
pub struct Trickle {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    ready: bool,
    pendings: usize,
}

impl Trickle {
    /// A source that delivers `data` at most `chunk` bytes at a time
    /// (`chunk` is raised to at least 1).
    pub fn new(data: impl Into<Vec<u8>>, chunk: usize) -> Self {
        Trickle {
            data: data.into(),
            pos: 0,
            chunk: chunk.max(1),
            ready: false,
            pendings: 0,
        }
    }

    /// Every other call returns `Poll::Pending`, after asking the waker to
    /// poll again (as a real source would once data arrives). The calls in
    /// between append up to `chunk` bytes to `buf` and return how many;
    /// `Ready(0)` means end of input.
    pub fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut Vec<u8>) -> Poll<usize> {
        if !self.ready {
            self.ready = true;
            self.pendings += 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.ready = false;
        let end = self.data.len().min(self.pos + self.chunk);
        buf.extend_from_slice(&self.data[self.pos..end]);
        let n = end - self.pos;
        self.pos = end;
        Poll::Ready(n)
    }

    /// How many times `poll_read` returned `Pending`.
    pub fn pendings(&self) -> usize {
        self.pendings
    }
}

/// Lines from a [`Trickle`], read asynchronously.
#[derive(Debug)]
pub struct AsyncLines {
    src: Trickle,
    buf: Vec<u8>,
    eof: bool,
}

impl AsyncLines {
    /// Read lines from `src`.
    pub fn new(src: Trickle) -> Self {
        AsyncLines {
            src,
            buf: Vec::new(),
            eof: false,
        }
    }

    /// A future for the next line: `lines.next_line().await`.
    pub fn next_line(&mut self) -> NextLine<'_> {
        NextLine { lines: self }
    }

    /// The source, for inspecting [`Trickle::pendings`].
    pub fn source(&self) -> &Trickle {
        &self.src
    }
}

/// The future returned by [`AsyncLines::next_line`].
#[derive(Debug)]
pub struct NextLine<'a> {
    lines: &'a mut AsyncLines,
}

/// m20 A. Resolve to the next line's bytes without `\n` (and without a `\r`
/// before it), or `None` after the last line.
///
/// Loop: if `buf` holds a `\n`, remove that line from the front of `buf` and
/// return it. If the source has ended, return what is left in `buf` as a
/// final line (unless it is empty), then `None`. Otherwise poll the source:
/// on `Pending` return `Pending` (the source already arranged a wake-up); on
/// `Ready(0)` remember the end; on `Ready(n)` loop again.
impl Future for NextLine<'_> {
    type Output = Option<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let lines = &mut *self.get_mut().lines;
        loop {
            if let Some(i) = lines.buf.iter().position(|&b| b == b'\n') {
                let mut line: Vec<u8> = lines.buf.drain(..=i).collect();
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Poll::Ready(Some(line));
            }
            if lines.eof {
                if lines.buf.is_empty() {
                    return Poll::Ready(None);
                }
                return Poll::Ready(Some(std::mem::take(&mut lines.buf)));
            }
            match lines.src.poll_read(cx, &mut lines.buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(0) => lines.eof = true,
                Poll::Ready(_) => {}
            }
        }
    }
}

/// m20 B. Run a future to completion on the current thread.
///
/// Pin the future (`std::pin::pin!`), build a `Waker` from an
/// `Arc<impl std::task::Wake>` that unparks this thread, and loop: poll;
/// on `Ready` return; on `Pending` call `std::thread::park()`. (A spurious
/// wake-up only causes an extra poll, which futures must tolerate.)
pub fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

/// m20 C. The async twin of [`crate::pipeline::process_reader`]: the same
/// line rules, tally, and sink calls. Malformed lines are only counted.
///
/// Reuse the per-line helper you extracted in m18; only the way lines
/// arrive differs.
pub async fn process_async<S: Sink + ?Sized>(
    lines: &mut AsyncLines,
    filter: &Filter,
    sink: &mut S,
) -> io::Result<Tally> {
    let mut tally = Tally::default();
    let mut line_no = 0;
    while let Some(line) = lines.next_line().await {
        line_no += 1;
        handle_line(&line, line_no, filter, sink, &mut |_| {}, &mut tally)?;
    }
    Ok(tally)
}
