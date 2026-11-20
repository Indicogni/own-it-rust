//! Lesson 08: `default = EXPR` runs late, runs only when needed, and is the
//! user's code.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Builder;
use std::sync::atomic::{AtomicU32, Ordering};

static CALLS: AtomicU32 = AtomicU32::new(0);

/// Counts how often it runs.
fn next_port() -> u16 {
    CALLS.fetch_add(1, Ordering::SeqCst);
    8080
}

// Functions and a constant with names that a macro author would choose for
// the local variables of `build`. EXPR must reach these items.
fn missing() -> u32 {
    1
}
fn value() -> u32 {
    10
}
fn result() -> u32 {
    100
}
fn builder() -> u32 {
    1000
}
const FALLBACK: &str = "none";

// The same with two underscores in front: the names that a careful macro
// author would choose. Even these must stay the user's inside EXPR, which
// only hygiene (`Span::mixed_site`) can guarantee.
fn __missing() -> u8 {
    1
}
fn __value() -> u8 {
    2
}
fn __fallback() -> u8 {
    4
}
fn __default() -> u8 {
    8
}
fn __required() -> u8 {
    16
}
fn __required_0() -> u8 {
    32
}
fn __builder() -> u8 {
    64
}

#[derive(Builder)]
struct Server {
    host: String,
    #[builder(default = next_port())]
    port: u16,
    // EXPR may be any expression, with a block, a macro call and `?`-free
    // control flow of its own.
    #[builder(default = {
        let parts = vec![missing(), value(), result(), builder()];
        parts.iter().sum()
    })]
    weight: u32,
    #[builder(default = if cfg!(debug_assertions) { FALLBACK.len() } else { FALLBACK.len() })]
    width: usize,
    #[builder(default = Some(FALLBACK.to_owned()))]
    motto: Option<String>,
    #[builder(
        default = __missing() + __value() + __fallback() + __default() + __required()
            + __required_0() + __builder()
    )]
    mask: u8,
}

fn main() {
    let builder = Server::builder().host("a".to_owned());
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        0,
        "EXPR must not run when the builder is made or a setter is called"
    );

    let server = match builder.build() {
        Ok(server) => server,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        1,
        "EXPR runs once, inside `build`"
    );
    assert_eq!(server.port, 8080);
    assert_eq!(server.weight, 1111);
    assert_eq!(server.width, 4);
    assert_eq!(server.motto.as_deref(), Some("none"));
    assert_eq!(server.mask, 127);

    let server = match Server::builder()
        .host("b".to_owned())
        .port(1)
        .motto("set".to_owned())
        .build()
    {
        Ok(server) => server,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        1,
        "EXPR must not run when the field was set"
    );
    assert_eq!(server.port, 1);
    assert_eq!(server.motto.as_deref(), Some("set"));

    assert!(Server::builder().build().is_err(), "`host` is missing");
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        1,
        "EXPR must not run when `build` fails"
    );
}
