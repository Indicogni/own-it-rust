//@ expect: run
//@ why: a caller must be able to branch on what went wrong and to point at the line, without reading the error's text

use quota::ErrorKind;
use std::collections::HashSet;

fn fails(text: &str) -> (ErrorKind, usize) {
    let error = quota::parse(text).expect_err("the text has a mistake");
    (error.kind(), error.line())
}

fn main() {
    assert_eq!(fails("uploads"), (ErrorKind::MissingLimit, 1));
    assert_eq!(fails("uploads   "), (ErrorKind::MissingLimit, 1));
    assert_eq!(fails("a 1\nuploads many"), (ErrorKind::InvalidLimit, 2));
    assert_eq!(fails("uploads -1"), (ErrorKind::InvalidLimit, 1));
    assert_eq!(fails("uploads 4294967296"), (ErrorKind::InvalidLimit, 1));
    assert_eq!(fails("uploads 1 2"), (ErrorKind::InvalidLimit, 1));
    assert_eq!(fails("a 1\nb 2\na 3"), (ErrorKind::Duplicate, 3));

    // Skipped lines count.
    assert_eq!(
        fails("# plan\n\nuploads 20\n\nuploads 30"),
        (ErrorKind::Duplicate, 5)
    );
    // The first wrong line wins.
    assert_eq!(fails("a 1\nb\na 2"), (ErrorKind::MissingLimit, 2));
    // On one line, the limit is checked before the name.
    assert_eq!(fails("a 1\na x"), (ErrorKind::InvalidLimit, 2));

    // A kind is a small value: it can be copied, compared and used as a key.
    let kind = ErrorKind::Duplicate;
    let copy = kind;
    assert_eq!(kind, copy);
    let seen: HashSet<ErrorKind> = [kind, copy, ErrorKind::MissingLimit].into_iter().collect();
    assert_eq!(seen.len(), 2);
    assert!(!format!("{kind:?}").is_empty());
}
