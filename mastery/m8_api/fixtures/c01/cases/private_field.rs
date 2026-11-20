//@ expect: E0616 or E0609
//@ why: C-STRUCT-PRIVATE: "sorted, lower case, no duplicates" is an invariant; with a public field any user can break it, and the field's type can never change

fn main() {
    let mut set = tagset::TagSet::new();
    // E0616: the field `tags` is private. E0609: there is no field `tags` any
    // more, because you store the tags in another way.
    set.tags.push(String::from("Not Sorted"));
    println!("{set:?}");
}
