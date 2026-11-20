//! Lesson 05: `#[logged]` on functions of many shapes.
//! This program must compile and run.

use m5_proc_macros::calls;
use m5_proc_macros::logged;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

#[logged]
fn plain() -> u32 {
    7
}

/// Three ways to leave a function early. The exit line must be written in
/// every case, and after the body has run.
#[logged(label = "parse")]
fn parse(text: &str) -> Result<u32, std::num::ParseIntError> {
    if text.is_empty() {
        return Ok(0);
    }
    let number: u32 = text.parse()?;
    Ok(number * 2)
}

#[logged]
fn outer() -> u32 {
    // `let _ =` does not keep the guard: both of its lines are written here.
    let _ = calls::enter("by hand");
    inner() + 1
}

#[logged]
#[inline]
fn inner() -> u32 {
    1
}

/// Generic parameters, a where clause and a pattern in the parameter list.
#[logged]
pub(crate) fn largest<T>((first, second): (T, T)) -> T
where
    T: PartialOrd,
{
    if first > second { first } else { second }
}

/// Parameters with names that a macro might choose for its own variable. The
/// macro's variable must not hide them.
#[logged]
fn shadowing(_guard: u32, guard: u32, _g: u32) -> u32 {
    _guard + guard + _g
}

#[logged]
fn r#match() {}

#[logged]
fn panics() {
    panic!("on purpose");
}

struct Counter {
    count: u32,
}

impl Counter {
    #[logged(label = "Counter::bump")]
    fn bump(&mut self, by: u32) -> u32 {
        self.count += by;
        self.count
    }

    #[logged]
    fn limit() -> u32 {
        100
    }
}

#[logged]
async fn fetch(key: u32) -> u32 {
    let _ = calls::enter("inside the future");
    key + 1
}

/// Runs a future that never waits, without an async runtime.
fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("this future has nothing to wait for"),
    }
}

fn main() {
    assert_eq!(plain(), 7);
    assert_eq!(calls::take(), ["> plain", "< plain"]);

    assert_eq!(parse(""), Ok(0));
    assert_eq!(parse("21"), Ok(42));
    assert!(parse("x").is_err());
    assert_eq!(
        calls::take(),
        [
            "> parse", "< parse", "> parse", "< parse", "> parse", "< parse"
        ],
        "the exit line must be written after `return`, after `?` and at the normal end"
    );

    assert_eq!(outer(), 2);
    assert_eq!(
        calls::take(),
        [
            "> outer",
            "> by hand",
            "< by hand",
            "> inner",
            "< inner",
            "< outer"
        ],
        "the guard of `outer` must live until `outer` ends: `< outer` comes last"
    );

    assert_eq!(largest((3, 9)), 9);
    assert_eq!(largest(("b", "a")), "b");
    assert_eq!(
        shadowing(1, 2, 4),
        7,
        "the macro's guard must not hide a parameter"
    );
    r#match();
    assert_eq!(
        calls::take(),
        [
            "> largest",
            "< largest",
            "> largest",
            "< largest",
            "> shadowing",
            "< shadowing",
            "> match",
            "< match"
        ]
    );

    let caught = std::panic::catch_unwind(panics);
    assert!(caught.is_err());
    assert_eq!(
        calls::take(),
        ["> panics", "< panics"],
        "a guard is dropped during a panic too"
    );

    let mut counter = Counter { count: 1 };
    assert_eq!(counter.bump(2), 3);
    assert_eq!(Counter::limit(), 100);
    assert_eq!(
        calls::take(),
        ["> Counter::bump", "< Counter::bump", "> limit", "< limit"]
    );

    let future = fetch(1);
    assert!(
        calls::take().is_empty(),
        "an async function does nothing until its future is polled"
    );
    assert_eq!(run(future), 2);
    assert_eq!(
        calls::take(),
        [
            "> fetch",
            "> inside the future",
            "< inside the future",
            "< fetch"
        ]
    );
}
