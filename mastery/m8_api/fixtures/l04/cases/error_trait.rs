//@ expect: run
//@ why: an error type must work with the tools every Rust program has: `?`, `Box<dyn Error>`, and other threads

use std::error::Error;

/// Compiles only when `T` can be used as a boxed, thread-safe error.
fn usable_everywhere<T: Error + Send + Sync + 'static>() {}

/// An application function: `?` turns the library's error into a boxed one.
fn first_limit(text: &str) -> Result<u32, Box<dyn Error + Send + Sync>> {
    let quotas = quota::parse(text)?;
    let first = quotas.first().ok_or("the text has no quota")?;
    Ok(first.limit())
}

/// The same with the box that is not thread-safe.
fn count(text: &str) -> Result<usize, Box<dyn Error>> {
    Ok(quota::parse(text)?.len())
}

fn main() {
    usable_everywhere::<quota::ParseError>();

    assert_eq!(first_limit("uploads 20").expect("the text is fine"), 20);
    let boxed = first_limit("uploads").expect_err("the text has a mistake");
    assert_eq!(boxed.to_string(), "line 1: `uploads` has no limit");
    // The box still knows which error it holds.
    let original = boxed
        .downcast_ref::<quota::ParseError>()
        .expect("the box holds a ParseError");
    assert_eq!(original.line(), 1);
    assert_eq!(count("a 1\nb 2").expect("the text is fine"), 2);

    // An error can be made on one thread and handled on another.
    let error = std::thread::spawn(|| quota::parse("a x").expect_err("the text has a mistake"))
        .join()
        .expect("the worker does not panic");
    assert_eq!(error.kind(), quota::ErrorKind::InvalidLimit);
}
