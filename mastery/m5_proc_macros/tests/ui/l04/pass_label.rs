//! Lesson 04, task C: `#[derive(Label)]` on valid enums.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Label;
use std::marker::PhantomData;

#[derive(Label, Debug, PartialEq)]
enum Level {
    Low,
    #[label("mid")]
    Middle,
    /// Other attributes and an explicit discriminant change nothing.
    #[label("HIGH")]
    #[allow(dead_code)]
    High = 10,
}

/// An enum without variants is a valid type; it only has no values.
#[derive(Label)]
enum Never {}

#[derive(Label, Debug, PartialEq)]
enum Token {
    r#Match,
    Other,
}

/// Generic parameters and a where clause are carried over. This enum cannot
/// use `T` in a variant (variants must not have fields), which is fine for a
/// const parameter.
#[derive(Label, Debug, PartialEq)]
enum Sized<const N: usize>
where
    [u8; N]: Default,
{
    Small,
    Large,
}

mod elsewhere {
    // Local items named `Option`, `Some` and `None`, and no import of the trait.
    pub struct Option;
    pub struct Some;
    pub struct None;

    #[derive(m5_proc_macros::Label, Debug, PartialEq)]
    pub enum Side {
        Left,
        Right,
    }
}

fn never_label(value: Never) -> &'static str {
    value.label()
}

fn main() {
    assert_eq!(Level::Low.label(), "Low");
    assert_eq!(Level::Middle.label(), "mid");
    assert_eq!(Level::High.label(), "HIGH");
    assert_eq!(Level::from_label("mid"), Some(Level::Middle));
    assert_eq!(Level::from_label("HIGH"), Some(Level::High));
    assert_eq!(
        Level::from_label("High"),
        None,
        "labels are compared exactly"
    );
    assert_eq!(Level::from_label(""), None);

    assert!(Never::from_label("anything").is_none());
    let _ = never_label;

    assert_eq!(
        Token::r#Match.label(),
        "Match",
        "a default label drops `r#`"
    );
    assert_eq!(Token::from_label("Match"), Some(Token::r#Match));

    assert_eq!(Sized::<4>::Large.label(), "Large");
    assert_eq!(Sized::<4>::from_label("Small"), Some(Sized::Small));

    use elsewhere::Side;
    assert_eq!(
        <Side as Label>::from_label("Right"),
        std::option::Option::Some(Side::Right)
    );
    let _marker: PhantomData<u8> = PhantomData;
}
