//! The manifest says `unused_mut = "forbid"`.

#[allow(unused_mut)]
pub fn e() -> i32 {
    let mut never_changed = 1;
    never_changed
}
