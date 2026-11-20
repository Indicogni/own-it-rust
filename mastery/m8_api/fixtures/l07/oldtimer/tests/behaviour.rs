//! What `oldtimer` does. You do not need to edit this file: task B changes
//! how the library is written, not what it does.

use oldtimer::{is_within, laps_in_order, parse_duration, unit_seconds};

#[test]
fn units_are_looked_up_by_letter() {
    assert_eq!(unit_seconds('h'), Some(3600));
    assert_eq!(unit_seconds('m'), Some(60));
    assert_eq!(unit_seconds('s'), Some(1));
    assert_eq!(unit_seconds('d'), None);
}

#[test]
fn durations_are_sums_of_parts() {
    assert_eq!(parse_duration("90s"), Ok(90));
    assert_eq!(parse_duration("1h 30m"), Ok(5400));
    assert_eq!(parse_duration("  2m   5s "), Ok(125));
    assert_eq!(parse_duration(""), Ok(0));
}

#[test]
fn wrong_parts_are_reported_and_counted() {
    let before = oldtimer::rejected();
    assert_eq!(
        parse_duration("1h 5x"),
        Err(String::from("`5x`: unknown unit"))
    );
    assert_eq!(
        parse_duration("h"),
        Err(String::from("`h`: missing number"))
    );
    assert_eq!(
        parse_duration("1.5h"),
        Err(String::from("`1.5h`: missing number"))
    );
    assert!(
        oldtimer::rejected() >= before + 3,
        "every rejected text is counted"
    );
}

#[test]
fn a_missing_limit_allows_everything() {
    assert!(is_within(None, u64::MAX));
    assert!(is_within(Some(60), 60));
    assert!(!is_within(Some(60), 61));
}

#[test]
fn laps_are_checked_for_order() {
    assert!(laps_in_order(&[]));
    assert!(laps_in_order(&[10]));
    assert!(laps_in_order(&[10, 10, 12]));
    assert!(!laps_in_order(&[10, 9]));
}
