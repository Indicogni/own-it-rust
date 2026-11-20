//@ expect: run
//@ why: BUG: "separated by" means between the tags; no separator after the last tag, and none for an empty set

use tagset::TagSet;

fn main() {
    let mut set = TagSet::new();
    assert_eq!(set.to_string(), "");
    set.insert("web").expect("a valid tag");
    assert_eq!(set.to_string(), "web");
    set.insert("cli").expect("a valid tag");
    assert_eq!(set.to_string(), "cli, web");
}
