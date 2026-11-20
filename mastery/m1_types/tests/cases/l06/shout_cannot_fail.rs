//@ expect: run
//@ why: the error type of Shout::parse must have no values, so that a caller can take the Ok value with an irrefutable pattern.
use m1_types::l06_zst_never::{Parse, Shout, unwrap_infallible};

fn main() {
    // This `let` has no `else`: it compiles only if `Err` is impossible.
    let Ok(shout) = Shout::parse("no error path");
    assert_eq!(shout.0, "NO ERROR PATH");

    let again = unwrap_infallible(Shout::parse("ok"));
    assert_eq!(again, Shout("OK".to_string()));
}
