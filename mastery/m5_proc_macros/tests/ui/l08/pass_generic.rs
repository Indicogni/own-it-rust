//! Lesson 08: generic parameters, where clauses and visibility.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Builder;
use std::fmt::Display;

/// A type without `Clone`, `Default` and `Debug`.
struct Opaque;

#[derive(Builder)]
struct Page<'a, T: Display = u8, const N: usize = 2>
where
    T: Send,
{
    title: &'a str,
    body: T,
    cells: [u8; N],
    extra: Option<&'a [T]>,
    #[builder(default)]
    tags: Vec<T>,
}

mod shop {
    use m5_proc_macros::Builder;

    /// `pub(crate)`: the builder, `builder()`, the setters and `build()` are
    /// `pub(crate)` too, also for the private field `secret`.
    #[derive(Builder)]
    pub(crate) struct Order {
        pub(crate) id: u32,
        secret: u64,
    }

    impl Order {
        pub(crate) fn secret(&self) -> u64 {
            self.secret
        }
    }
}

/// The builder type can be named and stored like any other type.
fn half_built<'a>() -> PageBuilder<'a, Opaque2, 1> {
    Page::builder().title("draft")
}

struct Opaque2;

impl Display for Opaque2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("opaque")
    }
}

fn main() {
    let rest = [String::from("x")];
    let page: Page<'_, String, 3> = match Page::builder()
        .title("home")
        .body("welcome".to_owned())
        .cells([1, 2, 3])
        .extra(&rest)
        .build()
    {
        Ok(page) => page,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(page.title, "home");
    assert_eq!(page.body, "welcome");
    assert_eq!(page.cells, [1, 2, 3]);
    assert_eq!(page.extra.map(<[String]>::len), Some(1));
    assert!(page.tags.is_empty());

    // `Opaque2` has no Clone, Default or Debug. Only `tags: Vec<T>` needs a
    // default, and `Vec<T>` has one for every `T`.
    let page = match half_built().body(Opaque2).cells([0]).build() {
        Ok(page) => page,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(page.body.to_string(), "opaque");

    let missing = match half_built().build() {
        Ok(_) => panic!("`body` and `cells` are missing"),
        Err(error) => error,
    };
    assert_eq!(missing.type_name(), "Page");
    assert_eq!(missing.missing(), ["body", "cells"]);

    let order = match shop::Order::builder().id(1).secret(42).build() {
        Ok(order) => order,
        Err(error) => panic!("{error}"),
    };
    assert_eq!((order.id, order.secret()), (1, 42));
    let _ = Opaque;
}
