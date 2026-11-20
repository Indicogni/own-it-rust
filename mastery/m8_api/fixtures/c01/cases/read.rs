//@ expect: run
//@ why: C-ITER, C-CONV, C-COMMON-TRAITS: the iterator method of a collection is `iter`, and `&collection` works in a `for` loop; a conversion that consumes `self` is `into_`; text for people comes from `Display`, so `to_string()` and `{}` work

use tagset::TagSet;

fn main() {
    let mut set = TagSet::new();
    for tag in ["web", "cli", "async"] {
        set.insert(tag).expect("a valid tag");
    }

    // `contains` takes `&str`.
    assert!(set.contains("cli"));
    assert!(!set.contains("gui"));

    // `iter` yields the tags in alphabetical order.
    let tags: Vec<String> = set.iter().map(|tag| tag.to_string()).collect();
    assert_eq!(tags, ["async", "cli", "web"]);

    // `&set` is iterable, like `&Vec<T>`.
    let mut letters = String::new();
    for tag in &set {
        letters.push_str(&tag[..1]);
    }
    assert_eq!(letters, "acw");

    // `Display`.
    assert_eq!(format!("tags: {set}"), "tags: async, cli, web");

    // `into_vec` consumes the set and gives the vector without copying the tags.
    let owned: Vec<String> = set.into_vec();
    assert_eq!(owned, ["async", "cli", "web"]);
}
