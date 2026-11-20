//@ expect: run
//@ why: the text of an error is read by people and often ends up in the middle of a longer message, so it is short, lower case, and has no full stop

fn message(text: &str) -> String {
    quota::parse(text)
        .expect_err("the text has a mistake")
        .to_string()
}

fn main() {
    assert_eq!(message("a 1\nuploads"), "line 2: `uploads` has no limit");
    assert_eq!(
        message("a 1\nuploads many"),
        "line 2: invalid limit for `uploads`"
    );
    assert_eq!(
        message("uploads 1\nuploads 2"),
        "line 2: `uploads` is defined twice"
    );
    assert_eq!(message("\n\n\nx y z"), "line 4: invalid limit for `x`");

    // `Debug` is for programmers; it must exist and must not be empty.
    let error = quota::parse("uploads").expect_err("the text has a mistake");
    assert!(!format!("{error:?}").is_empty());
}
