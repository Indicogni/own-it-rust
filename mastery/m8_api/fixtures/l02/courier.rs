//! courier 1.0.0: sends text through a transport. You do not need to edit this file.
//!
//! This is the published version 1.0.0 of a library crate. Lesson 02 applies
//! one change at a time to this text.

use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Something that carries bytes. Users of this library implement the trait
/// for their own transports.
pub trait Transport {
    /// Sends `bytes` and returns how many of them were accepted.
    fn send(&mut self, bytes: &[u8]) -> usize;
}

/// A transport that keeps everything in memory.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Memory {
    pub sent: Vec<u8>,
}

impl Transport for Memory {
    fn send(&mut self, bytes: &[u8]) -> usize {
        self.sent.extend_from_slice(bytes);
        bytes.len()
    }
}

mod sealed {
    pub trait Sealed {}
}

/// How text becomes bytes. This trait is sealed: users can call it and name
/// it, but only this library implements it.
pub trait Codec: sealed::Sealed {
    fn encode(&self, text: &str) -> Vec<u8>;
}

/// The text as it is.
#[derive(Debug, Clone, Copy)]
pub struct Plain;

/// The text followed by a line break.
#[derive(Debug, Clone, Copy)]
pub struct Line;

impl sealed::Sealed for Plain {}
impl sealed::Sealed for Line {}

impl Codec for Plain {
    fn encode(&self, text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }
}

impl Codec for Line {
    fn encode(&self, text: &str) -> Vec<u8> {
        format!("{text}\n").into_bytes()
    }
}

/// Counts messages. Clones share one count.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    messages: Arc<AtomicUsize>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats::default()
    }

    pub fn record(&self) {
        self.messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn messages(&self) -> usize {
        self.messages.load(Ordering::Relaxed)
    }
}

/// Encodes `message` with `codec` and sends it through `transport`. Returns
/// the number of bytes that the transport accepted.
pub fn deliver<T: Transport, M: Display>(
    transport: &mut T,
    codec: &dyn Codec,
    message: M,
) -> usize {
    transport.send(&codec.encode(&message.to_string()))
}

/// The words of `text`.
pub fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
}
