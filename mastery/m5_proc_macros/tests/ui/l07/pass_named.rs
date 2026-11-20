//! Lesson 07, task B: `#[derive(Named)]` and its `crate` argument.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Named;

#[derive(Named)]
struct Plain;

#[derive(Named)]
enum Pair<'a, T: Clone = u8, const N: usize = 2>
where
    T: Send,
{
    Left(&'a T),
    Right([T; N]),
}

#[derive(Named)]
union Bits {
    int: u32,
    float: f32,
}

#[derive(Named)]
struct r#Type;

/// A library that re-exports this crate under another name, as a user sees it
/// who depends on `toolkit` and not on `m5_proc_macros`.
mod toolkit {
    pub mod deps {
        pub use m5_proc_macros as kit;
    }
}

mod user {
    // The derive is told where the trait is. `Named` itself is not imported.
    #[derive(m5_proc_macros::Named)]
    #[named(crate = crate::toolkit::deps::kit)]
    pub struct Order<T>(pub T);

    #[derive(m5_proc_macros::Named)]
    #[named(crate = super::toolkit::deps::kit)]
    #[derive(Debug)]
    pub enum Status {
        Open,
    }
}

fn name_of<T: Named>() -> &'static str {
    T::NAME
}

fn main() {
    assert_eq!(Plain::NAME, "Plain");
    assert_eq!(name_of::<Pair<'static, String, 3>>(), "Pair");
    assert_eq!(Bits::NAME, "Bits");
    assert_eq!(r#Type::NAME, "Type");
    assert_eq!(name_of::<user::Order<u8>>(), "Order");
    assert_eq!(name_of::<user::Status>(), "Status");
}
