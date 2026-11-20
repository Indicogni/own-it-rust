//@ expect: run
//@ why: C-GENERIC, C-CUSTOM-TYPE, C-GOOD-ERR, C-VALIDATE: take `&str`, not `&String`; no `bool` parameter that switches validation off; "refused" and "already there" are different answers, and a refusal is an error type that implements `std::error::Error`

use std::error::Error;
use tagset::{TagError, TagSet};

/// `?` works with the library's error in an ordinary application function.
fn build(tags: &[&str]) -> Result<TagSet, Box<dyn Error + Send + Sync>> {
    let mut set = TagSet::new();
    for tag in tags {
        set.insert(tag)?;
    }
    Ok(set)
}

fn main() {
    let mut set = TagSet::new();
    // A string literal is enough; no `String` has to be built first.
    assert_eq!(set.insert("web"), Ok(true), "a new tag");
    assert_eq!(
        set.insert("web"),
        Ok(false),
        "already there: not an error, and not new"
    );
    let owned = String::from("cli");
    assert_eq!(
        set.insert(&owned),
        Ok(true),
        "a `&String` is accepted where `&str` is asked"
    );
    assert_eq!(set.len(), 2);

    // Validation is not optional, and the caller learns the reason.
    let error: TagError = set.insert("two words").expect_err("white space is refused");
    assert_ne!(
        Ok(false),
        set.insert("tab\there").map_err(|_| ()),
        "a tab is white space too"
    );
    let empty: TagError = set.insert("").expect_err("an empty tag is refused");
    assert_ne!(error, empty, "the two reasons are different values");
    assert_eq!(set.len(), 2, "a refused tag changes nothing");

    // The error is a good citizen: readable, lower case, no full stop.
    for error in [error, empty] {
        let text = error.to_string();
        assert!(
            !text.is_empty() && !text.ends_with('.'),
            "bad message: {text:?}"
        );
        assert!(
            text.starts_with(char::is_lowercase),
            "bad message: {text:?}"
        );
        assert!(!format!("{error:?}").is_empty());
    }

    assert_eq!(build(&["a", "b"]).expect("both are valid").len(), 2);
    let boxed = build(&["a", "b c"]).expect_err("the second tag is refused");
    assert!(boxed.downcast_ref::<TagError>().is_some());
}
