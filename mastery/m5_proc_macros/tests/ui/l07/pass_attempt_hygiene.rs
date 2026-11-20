//! Lesson 07, task A: `attempt!` among names that try to confuse it.
//! This program must compile and run. If it does not compile, read which name
//! the compiler complains about: the expansion and the user's code are using
//! one name for two things.

#![allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    unused_variables
)]

use m5_proc_macros::attempt;

/// Part 1: the user's variables have the names that a macro author would
/// choose for the macro's own variables. All of them are `&str` here, and the
/// user's expression reads all of them.
fn user_variables_stay_visible() {
    let max = "m";
    let tries = "t";
    let attempt = "a";
    let attempts = "s";
    let count = "c";
    let counter = "o";
    let i = "i";
    let n = "n";
    let left = "l";
    let remaining = "r";
    let limit = "L";
    let result = "R";
    let value = "v";
    let error = "e";
    let err = "E";
    let last = "x";
    let outcome = "u";
    // The very names of the contract. Only hygiene can keep these apart.
    let __attempt_max = "M";
    let __attempt_tries = "T";
    let __attempt_value = "V";
    let __attempt_error = "X";
    let joined: Result<String, ()> = attempt!(
        2,
        Ok([
            max,
            tries,
            attempt,
            attempts,
            count,
            counter,
            i,
            n,
            left,
            remaining,
            limit,
            result,
            value,
            error,
            err,
            last,
            outcome,
            __attempt_max,
            __attempt_tries,
            __attempt_value,
            __attempt_error
        ]
        .concat())
    );
    assert_eq!(
        joined,
        Ok("mtascoinlrLRveExuMTVX".to_owned()),
        "the expression must see the user's variables, not the macro's"
    );
    // After the macro, the user's variables are what they were.
    assert_eq!((__attempt_max, __attempt_tries), ("M", "T"));
    // MAX is the user's code as well.
    let tries: u32 = 2;
    let mut calls = 0;
    let never: Result<(), u32> = attempt!(tries + 1, {
        calls += 1;
        Err(calls)
    });
    assert_eq!(never, Err(3));
}

/// Part 2: the user's module has unit structs and constants with the names
/// that a macro author would choose for variables. Hygiene does not help
/// here: it separates variables, and these are items. When the expansion
/// writes `let tries = ..` or the pattern `Ok(value)`, the compiler takes
/// `tries` and `value` for these items and makes no variable. Only a name that
/// no item has, such as `__attempt_tries`, is safe.
mod items_with_plain_names {
    macro_rules! unit_structs {
        ($($name:ident)*) => {
            $( #[derive(PartialEq, Debug)] pub struct $name; )*
            pub fn check() {
                let seen: Result<($($name,)*), ()> = m5_proc_macros::attempt!(1, Ok(($($name,)*)));
                assert_eq!(seen, Ok(($($name,)*)));
            }
        };
    }
    // Twelve names: the standard library compares tuples of up to twelve parts.
    unit_structs!(max tries attempt count i n left limit result value error last);

    pub mod constants {
        pub const max: u32 = 2;
        pub const tries: u32 = 7;
        pub const value: &str = "v";
        pub const error: &str = "e";

        pub fn check() {
            let outcome: Result<&str, &str> = m5_proc_macros::attempt!(max, Err(error));
            assert_eq!(outcome, Err("e"));
            let outcome: Result<&str, &str> = m5_proc_macros::attempt!(tries, Ok(value));
            assert_eq!(outcome, Ok("v"));
        }
    }
}

/// Part 3: the module has its own `Result`, `Ok`, `Err`, `Option`, `Some`,
/// `None` and `u32`. The expansion must still mean the standard ones.
mod shadowed_prelude {
    pub struct Result;
    pub struct Option;
    pub struct Ok;
    pub struct Err;
    pub struct Some;
    pub struct None;
    pub struct u32;

    pub fn check() -> ::core::primitive::u32 {
        let mut calls = 0;
        let outcome: ::core::result::Result<::core::primitive::u32, ()> =
            m5_proc_macros::attempt!(4, {
                calls += 1;
                if calls < 3 {
                    ::core::result::Result::Err(())
                } else {
                    ::core::result::Result::Ok(calls)
                }
            });
        match outcome {
            ::core::result::Result::Ok(calls) => calls,
            ::core::result::Result::Err(()) => 0,
        }
    }
}

fn main() {
    user_variables_stay_visible();
    items_with_plain_names::check();
    items_with_plain_names::constants::check();
    assert_eq!(shadowed_prelude::check(), 3);
}
