//@ expect: E0451
//@ why: C-STRUCT-PRIVATE: a struct expression is allowed only when every field is public, whatever the fields are called; then a user can put unsorted tags into a set, and you can never add a field

fn main() {
    let set = tagset::TagSet::new();
    // "A new `TagSet`, with every field taken from `set`." No field is named,
    // so this check does not depend on how you store the tags.
    let copy = tagset::TagSet { ..set };
    println!("{copy:?}");
}
