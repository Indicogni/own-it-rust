mod support;

#[test]
fn doubles_the_sample() {
    assert_eq!(layout::double(support::sample()), 42);
}

#[test]
fn doubles_negative_numbers() {
    assert_eq!(layout::double(-3), -6);
}

#[test]
fn doubles_zero() {
    assert_eq!(layout::double(0), 0);
}
