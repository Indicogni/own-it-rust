//! Lesson 08, own passing case (model answer): a builder is passed to a
//! function and back, and is then finished in two different ways. The
//! supplied cases never name the builder type of a struct without generic
//! parameters, and never hand a builder to other code. Both work because the
//! builder is an ordinary struct and its setters take and return it by value.
//! This program must compile and run.

use m5_proc_macros::Builder;

#[derive(Builder, Debug, PartialEq)]
struct Request {
    #[builder(into)]
    host: String,
    #[builder(default = 443)]
    port: u16,
    path: Option<String>,
}

/// A function that takes and returns the builder: setters consume `self`,
/// so the builder moves through the function.
fn with_defaults(builder: RequestBuilder) -> RequestBuilder {
    builder.host("example.org")
}

fn main() {
    let secure = with_defaults(Request::builder()).build();
    let plain = with_defaults(Request::builder())
        .port(80)
        .path("/index".to_owned())
        .build();
    assert_eq!(
        secure,
        Ok(Request {
            host: "example.org".into(),
            port: 443,
            path: None
        })
    );
    assert_eq!(
        plain,
        Ok(Request {
            host: "example.org".into(),
            port: 80,
            path: Some("/index".into())
        })
    );
}
