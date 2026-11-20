//@ expect: run
//@ why: BUG: the documentation says "in lower case" and "without duplicates", so `Rust` and `rust` are one tag in whatever order they arrive

use tagset::TagSet;

fn main() {
    let mut set = TagSet::new();
    assert_eq!(set.insert("rust"), Ok(true));
    assert_eq!(
        set.insert("Rust"),
        Ok(false),
        "the same tag in another spelling"
    );
    assert_eq!(set.insert("RUST"), Ok(false));
    assert_eq!(set.len(), 1, "one tag, not three");
    assert!(set.contains("rust") && set.contains("Rust"));

    let mut set = TagSet::new();
    assert_eq!(set.insert("Web"), Ok(true));
    assert_eq!(set.insert("Web"), Ok(false), "the same spelling twice");
    assert_eq!(set.len(), 1);
}
