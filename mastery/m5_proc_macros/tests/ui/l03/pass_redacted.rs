//! Lesson 03, tasks A and C: `#[derive(Redacted)]` and its helper attribute.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Redacted;

#[derive(Redacted)]
struct Login {
    user: String,
    #[redacted(mask)]
    password: String,
    #[redacted(skip)]
    attempts: u32,
    /// Other attributes on a field are none of this derive's business.
    #[redacted(rename = "ip")]
    #[allow(dead_code)]
    address: [u8; 4],
}

#[derive(Redacted)]
struct Empty {}

#[derive(Redacted)]
struct AllHidden {
    #[redacted(skip)]
    a: u8,
    #[redacted(skip)]
    b: u8,
}

#[derive(Redacted)]
struct Token {
    #[redacted(mask)]
    #[redacted(rename = "secret")]
    value: String,
    r#type: u8,
}

mod elsewhere {
    // A module with its own `String` and `Debug`, and without the trait in
    // scope: the generated code must not notice.
    pub struct String;
    pub trait Debug {}

    #[derive(m5_proc_macros::Redacted)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }
}

fn main() {
    let login = Login {
        user: "ann".into(),
        password: "hunter2".into(),
        attempts: 3,
        address: [10, 0, 0, 1],
    };
    assert_eq!(
        login.redacted(),
        r#"Login { user: "ann", password: ***, ip: [10, 0, 0, 1] }"#
    );
    assert_eq!(Empty {}.redacted(), "Empty {}");
    assert_eq!(AllHidden { a: 1, b: 2 }.redacted(), "AllHidden {}");
    let token = Token {
        value: "abc".into(),
        r#type: 2,
    };
    assert_eq!(token.redacted(), "Token { secret: ***, type: 2 }");
    assert_eq!(
        elsewhere::Point { x: 1, y: -2 }.redacted(),
        "Point { x: 1, y: -2 }"
    );
}
