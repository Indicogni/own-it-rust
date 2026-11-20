//@ expect: run
//@ why: C-METHOD: a function with an obvious receiver is a method, so that it is found on the type and can be chained

use tagset::TagSet;

fn main() {
    let a: TagSet = "web, cli".parse().expect("valid tags");
    let b: TagSet = "cli, async".parse().expect("valid tags");
    let both = a.union(&b);
    assert_eq!(both.to_string(), "async, cli, web");
    // The two operands are still there.
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 2);
    assert_eq!(a.union(&TagSet::new()), a);
    assert_eq!(a.union(&b).union(&a).len(), 3);
}
