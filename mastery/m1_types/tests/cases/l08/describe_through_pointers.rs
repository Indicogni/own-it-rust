//@ expect: run
//@ why: &T and Box<T> must be Describe for every describable T, sized or not; only your crate can write these impls, a user's crate cannot.
use m1_types::l08_coherence::{Describe, describe_all};

fn log<D: Describe>(value: D) -> String {
    value.describe()
}

fn main() {
    // `&str` and `&[i64]`: references to unsized types.
    assert_eq!(log("text"), "\"text\"");
    let numbers = [1_i64, 2];
    assert_eq!(log(&numbers[..]), "[1, 2]");
    assert_eq!(log(&&true), "yes");

    // Slices of references.
    assert_eq!(describe_all(&["a", "b"]), "\"a\"; \"b\"");
    assert_eq!(vec!["x"].describe(), "[\"x\"]");

    // Trait objects in boxes, in one list.
    let mixed: Vec<Box<dyn Describe>> = vec![
        Box::new(7_i64),
        Box::new(String::from("seven")),
        Box::new(vec![false]),
    ];
    assert_eq!(mixed.describe(), "[7, \"seven\", [no]]");
    assert_eq!(log(Box::new(1_i64)), "1");
    let boxed_text: Box<str> = "boxed".into();
    assert_eq!(log(boxed_text), "\"boxed\"");
}
