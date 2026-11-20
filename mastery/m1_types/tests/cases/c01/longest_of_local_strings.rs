//@ expect: run
//@ why: longest must work on data that is not 'static, and its result must borrow from the items for exactly as long as they live.
use m1_types::c01_code_review::longest;

fn main() {
    let words = vec![
        String::from("fig"),
        String::from("banana"),
        String::from("cherry"),
    ];
    assert_eq!(longest(&words), Some("banana"));

    let line = String::from("one three five");
    let parts: Vec<&str> = line.split(' ').collect();
    assert_eq!(longest(&parts), Some("three"));

    let nothing: Vec<String> = Vec::new();
    assert_eq!(longest(&nothing), None);
}
