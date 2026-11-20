//@ expect: run
//@ why: Describe must be implemented for the listed standard types, with containers built from their items.
use m1_types::l08_coherence::{Describe, describe_all};

fn main() {
    assert_eq!((-3_i64).describe(), "-3");
    assert_eq!(
        (true.describe(), false.describe()),
        ("yes".to_string(), "no".to_string())
    );
    assert_eq!(String::from("abc").describe(), "\"abc\"");
    assert_eq!("abc".describe(), "\"abc\"");

    assert_eq!(Some(4_i64).describe(), "4");
    assert_eq!(None::<bool>.describe(), "none");

    assert_eq!(vec![1_i64, 2, 3].describe(), "[1, 2, 3]");
    assert_eq!(Vec::<i64>::new().describe(), "[]");
    assert_eq!([true, false].describe(), "[yes, no]");
    assert_eq!(
        vec![Some(String::from("a")), None].describe(),
        "[\"a\", none]"
    );
    assert_eq!(vec![vec![1_i64], vec![]].describe(), "[[1], []]");

    assert_eq!(describe_all(&[1_i64, 2]), "1; 2");
    assert_eq!(describe_all::<bool>(&[]), "");
}
