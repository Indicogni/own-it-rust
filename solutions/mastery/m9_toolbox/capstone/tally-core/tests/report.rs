//! Snapshot tests of the report layout. Review changes with `cargo insta review`.

use tally_core::{parse, report};

#[test]
fn report_of_the_sample_ledger() {
    let ledger = include_str!("../../ledger.txt");
    let entries = parse(ledger).expect("the sample ledger is valid");
    insta::assert_snapshot!(report(&entries));
}

#[test]
fn report_of_an_empty_ledger() {
    insta::assert_snapshot!(report(&[]));
}
