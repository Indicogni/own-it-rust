//@ expect: run
//@ why: C-NO-OUT, C-GOOD-ERR: a function returns its results and does not write them into a parameter; reading a value from text is the standard trait `FromStr`, so `"..".parse()` works, and a failure is the library's error type, not a `String`

use tagset::{TagError, TagSet};

fn main() {
    let set: TagSet = "web, CLI ,async,,"
        .parse()
        .expect("all parts are valid tags");
    assert_eq!(set.to_string(), "async, cli, web");

    let empty: TagSet = "  ".parse().expect("no tags is a valid set");
    assert!(empty.is_empty());

    // The first refused tag is the error; no half-filled set leaks out.
    let result: Result<TagSet, TagError> = "web, two words, cli".parse();
    let error = result.expect_err("`two words` is not a tag");
    assert!(!error.to_string().is_empty());

    // Parsing what `Display` printed gives the same set back.
    let again: TagSet = set
        .to_string()
        .parse()
        .expect("the set's own text is valid");
    assert_eq!(again, set);
}
