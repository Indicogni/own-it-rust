//@ expect: run
//@ why: the cause of an error belongs in `source()`, where a reporter finds it, and not in the text, where it would be printed twice

use std::error::Error;
use std::num::ParseIntError;

/// What a careful application prints: the error and all its causes.
fn report(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(error) = cause {
        text.push_str(": ");
        text.push_str(&error.to_string());
        cause = error.source();
    }
    text
}

fn main() {
    let error = quota::parse("uploads many").expect_err("the text has a mistake");
    let cause = error
        .source()
        .expect("an invalid limit has a cause: the error of the number parser");
    let cause = cause
        .downcast_ref::<ParseIntError>()
        .expect("the cause is the ParseIntError of the standard library");
    let expected = "many".parse::<u32>().expect_err("not a number");
    assert_eq!(cause, &expected);

    // The text of the error itself does not contain the text of its cause.
    assert!(
        !error.to_string().contains(&expected.to_string()),
        "the Display text repeats its source: {error}"
    );
    assert_eq!(
        report(&error),
        format!("line 1: invalid limit for `uploads`: {expected}")
    );

    // The other kinds have no cause.
    assert!(
        quota::parse("uploads")
            .expect_err("a mistake")
            .source()
            .is_none()
    );
    assert!(
        quota::parse("a 1\na 2")
            .expect_err("a mistake")
            .source()
            .is_none()
    );
}
