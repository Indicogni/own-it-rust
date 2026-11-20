//! Lesson 03, tasks B and C: generic parameters, where clauses and bounds.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Redacted;
use std::fmt::Debug;

/// A type without `Debug`.
struct Opaque;

/// Lifetime, type and const parameters, a bound, defaults and a where clause.
#[derive(Redacted)]
struct View<'a, T: Clone = u8, const N: usize = 2>
where
    T: Send,
{
    title: &'a str,
    #[redacted(mask)]
    secret: T,
    rows: [u8; N],
    #[redacted(skip)]
    cache: Vec<T>,
}

/// `T` is only ever masked, so `Wrapper<Opaque>` must implement `Redacted`
/// although `Opaque` has no `Debug`. A bound `T: Debug` on the impl would be
/// too strict.
#[derive(Redacted)]
struct Wrapper<T> {
    id: u32,
    #[redacted(mask)]
    inner: T,
}

/// Here the bound that matters is on `T::Item`, not on `T`.
#[derive(Redacted)]
struct First<T: Iterator> {
    item: Option<T::Item>,
    #[redacted(skip)]
    rest: T,
}

/// A type that refers to itself.
#[derive(Debug, Redacted)]
struct Chain<T> {
    value: T,
    next: Option<Box<Chain<T>>>,
}

fn show<R: Redacted>(value: &R) -> String {
    value.redacted()
}

fn show_debug_free<T>(wrapper: &Wrapper<T>) -> String {
    // No `T: Debug` here: the impl for `Wrapper<T>` must not ask for it.
    wrapper.redacted()
}

fn show_first<T: Iterator>(first: &First<T>) -> String
where
    T::Item: Debug,
{
    first.redacted()
}

fn main() {
    let view: View<'_, String, 3> = View {
        title: "report",
        secret: "s3cret".to_owned(),
        rows: [1, 2, 3],
        cache: Vec::new(),
    };
    assert_eq!(
        show(&view),
        r#"View { title: "report", secret: ***, rows: [1, 2, 3] }"#
    );

    let wrapper = Wrapper {
        id: 7,
        inner: Opaque,
    };
    assert_eq!(show_debug_free(&wrapper), "Wrapper { id: 7, inner: *** }");

    let mut letters = "ab".chars();
    let first = First {
        item: letters.next(),
        rest: letters,
    };
    assert_eq!(show_first(&first), "First { item: Some('a') }");

    let chain = Chain {
        value: 1,
        next: Some(Box::new(Chain {
            value: 2,
            next: None,
        })),
    };
    assert_eq!(
        chain.redacted(),
        "Chain { value: 1, next: Some(Chain { value: 2, next: None }) }"
    );
}
