//! DRILL 02-07 · One coin too few
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many variants does `Coin` have, and how many arms does the
//! `match` have?

enum Coin {
    Penny,
    Nickel,
    Dime,
}

fn cents(coin: Coin) -> u32 {
    match coin {
        Coin::Penny => 1,
        Coin::Nickel => 5,
    }
}

fn main() {
    let total = cents(Coin::Penny) + cents(Coin::Nickel) + cents(Coin::Dime);
    assert_eq!(total, 16); // KEEP
}
