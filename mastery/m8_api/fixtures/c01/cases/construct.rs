//@ expect: run
//@ why: C-CTOR, C-COMMON-TRAITS, C-GETTER, C-DEBUG, C-SEND-SYNC: a constructor is called `new`; a type implements the common traits early (adding them later is easy, a user who needs them today cannot add them); a getter has no `get_` prefix; a type with `len` has `is_empty`

use tagset::TagSet;

/// Compiles only for types that can be moved to and shared with other threads.
fn thread_safe<T: Send + Sync>() {}

fn main() {
    let empty = TagSet::new();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty, TagSet::default());

    // `Debug` is never empty, also for an empty set.
    assert!(!format!("{empty:?}").is_empty());

    let mut set = TagSet::new();
    set.insert("rust").expect("`rust` is a valid tag");
    let copy = set.clone();
    assert_eq!(copy, set);
    assert_ne!(copy, empty);
    assert!(format!("{set:?}").contains("rust"));
    assert_eq!(set.len(), 1);
    assert!(!set.is_empty());

    thread_safe::<TagSet>();
    thread_safe::<tagset::TagError>();
}
