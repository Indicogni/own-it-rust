//@ expect: E0004
//@ why: more kinds will come; `#[non_exhaustive]` makes every user write a wildcard arm today, so a new kind is a minor change

use quota::ErrorKind;

fn main() {
    let error = quota::parse("uploads").expect_err("the text has a mistake");
    let advice = match error.kind() {
        ErrorKind::MissingLimit => "add a number",
        ErrorKind::InvalidLimit => "write a number from 0 to 4294967295",
        ErrorKind::Duplicate => "remove one of the two lines",
    };
    println!("{advice}");
}
