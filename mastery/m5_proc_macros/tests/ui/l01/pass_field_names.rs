//! Lesson 01, tasks D and E: `#[derive(FieldNames)]`.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::FieldNames;
use std::collections::HashMap;

#[derive(FieldNames)]
struct Job {
    id: u32,
    name: String,
}

#[derive(FieldNames)]
struct Empty {}

/// Every field here has a type or an attribute with a comma in it.
#[derive(FieldNames)]
#[rustfmt::skip] // keeps the last field without a comma after it
pub struct Tricky {
    /// Commas between `<` and `>` belong to the type.
    pub index: HashMap<String, Vec<(u8, u8)>>,
    pub(crate) nested: Vec<Vec<HashMap<u8, u8>>>,
    // The `>` of `->` closes nothing.
    run: fn(u8, u8) -> u8,
    make: Box<dyn Fn(u8) -> Vec<u8>>,
    // Here the `>` of `->` stands between `<` and a comma. If it closed the
    // `<`, the comma after it would end the field too early.
    convert: Result<fn(u8) -> u8, String>,
    #[allow(dead_code, unused)]
    pair: (u8, [u8; 2]),
    #[doc = "a, b, c"]
    pub(in crate) path: std::path::PathBuf,
    r#type: u8,
    last: Option<Result<u8, String>>
}

fn main() {
    assert_eq!(Job::FIELD_NAMES, ["id", "name"]);
    assert!(Empty::FIELD_NAMES.is_empty());
    assert_eq!(
        Tricky::FIELD_NAMES,
        [
            "index", "nested", "run", "make", "convert", "pair", "path", "type", "last"
        ],
        "one name per field: a comma inside a type or an attribute does not end a field"
    );
}
