//@ expect: run
//@ why: Box<L> must be a Label for every label type L, sized or not, so that a list of boxed trait objects can be passed to generic code.
use m1_types::l05_trait_objects::{Caption, Label, join_labels};

fn main() {
    let name = String::from("delta");
    let mixed: Vec<Box<dyn Label + '_>> =
        vec![Box::new(String::from("alpha")), Box::new(Caption(&name))];
    assert_eq!(join_labels(&mixed), "alpha, [delta]");

    let same: Vec<Box<Caption>> = vec![Box::new(Caption("a")), Box::new(Caption("b"))];
    assert_eq!(join_labels(&same), "[a], [b]");

    let nested: Box<Box<dyn Label>> = Box::new(Box::new(String::from("deep")));
    assert_eq!(nested.label(), "deep");
}
