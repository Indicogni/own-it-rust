//@ expect: run
//@ why: record must borrow the log only for the call, so that it can be called again and the log can be read afterwards.
use m1_types::l01_variance::record;

fn main() {
    let first = String::from("first");
    let second = String::from("second");
    let mut log = Vec::new();
    assert_eq!(record(&mut log, &first), 1);
    assert_eq!(record(&mut log, &second), 2);
    assert_eq!(log, ["first", "second"]);
}
