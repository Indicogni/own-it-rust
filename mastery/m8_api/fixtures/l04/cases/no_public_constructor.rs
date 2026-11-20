//@ expect: E0451
//@ why: `ParseError` is a struct whose fields are all private, so users cannot build one or depend on what it stores; this struct expression is allowed only when every field is public. (If you get E0639, the struct is also `#[non_exhaustive]`. Remove that attribute: it is for structs whose fields stay public, private fields already do its work, and with it this check cannot see whether your fields are private.)

/// Rebuilds an error from all the fields of another one.
fn rebuild(error: quota::ParseError) -> quota::ParseError {
    quota::ParseError { ..error }
}

fn main() {
    let error = quota::parse("uploads").expect_err("the text has a mistake");
    println!("{}", rebuild(error));
}
