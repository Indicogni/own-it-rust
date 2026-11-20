//! Lint levels set in three places: the manifest, an attribute, and the default.
//! `unused_variables` and `unused_mut` are both warn-by-default lints of rustc.

pub fn a() {
    let unused_a = 1;
}

#[allow(unused_variables)]
pub fn b() {
    let unused_b = 1;
}

#[expect(unused_variables, reason = "kept for the next release")]
pub fn c() {
    let unused_c = 1;
}

#[expect(unused_variables, reason = "kept for the next release")]
pub fn d() {
    let used = 1;
    assert_eq!(used, 1);
}

// The manifest says `unused_mut = "deny"`.
#[warn(unused_mut)]
pub fn e() -> i32 {
    let mut never_changed = 1;
    never_changed
}
