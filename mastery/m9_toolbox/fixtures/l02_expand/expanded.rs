#![feature(prelude_import)]
//! The source that `expanded.rs` was generated from, with
//! `cargo expand --lib > expanded.rs` (cargo-expand 1.0.127).
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use std::rc::Rc;
/// Field order matters to the derived comparison.
pub struct Release {
    pub patch: u32,
    pub major: u32,
}
#[automatically_derived]
impl ::core::fmt::Debug for Release {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "Release",
            "patch",
            &self.patch,
            "major",
            &&self.major,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Release {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Release {
    #[inline]
    fn eq(&self, other: &Release) -> bool {
        self.patch == other.patch && self.major == other.major
    }
}
#[automatically_derived]
impl ::core::cmp::PartialOrd for Release {
    #[inline]
    fn partial_cmp(
        &self,
        other: &Release,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        match ::core::cmp::PartialOrd::partial_cmp(&self.patch, &other.patch) {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                ::core::cmp::PartialOrd::partial_cmp(&self.major, &other.major)
            }
            cmp => cmp,
        }
    }
}
/// A handle that shares its payload.
pub struct Shared<T> {
    pub inner: Rc<T>,
}
#[automatically_derived]
impl<T: ::core::clone::Clone> ::core::clone::Clone for Shared<T> {
    #[inline]
    fn clone(&self) -> Shared<T> {
        Shared {
            inner: ::core::clone::Clone::clone(&self.inner),
        }
    }
}
pub fn bonus_demo() -> i32 {
    let x = 1;
    {
        let x = 100;
        x + x
    }
}
