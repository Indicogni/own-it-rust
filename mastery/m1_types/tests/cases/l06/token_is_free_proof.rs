//@ expect: run
//@ why: an AdminToken exists only after a successful log_in, takes no space, and cannot be duplicated.
use m1_types::implements;
use m1_types::l06_zst_never::{AdminToken, log_in, wipe};

fn main() {
    assert!(log_in("guess").is_none());
    assert!(log_in("").is_none());
    assert!(log_in("open sesame ").is_none());

    let token = log_in("open sesame").expect("the right password gives a token");
    let mut data = vec![1, 2, 3];
    wipe(&token, &mut data);
    assert!(data.is_empty());
    // The same proof can be shown again.
    let mut more = vec![9];
    wipe(&token, &mut more);
    assert!(more.is_empty());

    assert_eq!(size_of::<AdminToken>(), 0, "the proof must cost no memory");
    assert!(
        !implements!(AdminToken: Clone),
        "a token must not be Clone: a copy would be a proof without a check"
    );
    assert!(
        !implements!(AdminToken: Default),
        "Default would forge a token"
    );
}
