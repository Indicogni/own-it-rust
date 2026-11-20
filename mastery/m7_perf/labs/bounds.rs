//! Lab 5: where the bounds checks are. See LABS.md.
//!
//! Build: rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -o bounds.ll bounds.rs
//!
//! Every function adds two slices element by element and stores the sums.
//! `#[inline(never)]` keeps each function in the output as a function of its
//! own, so that you can find it by name.

/// An index in the loop. The lengths of `b` and `out` are unknown, and the
/// loop stores a result before it reaches the next index.
#[inline(never)]
pub fn add_indexed(a: &[u64], b: &[u64], out: &mut [u64]) {
    for i in 0..a.len() {
        out[i] = a[i].wrapping_add(b[i]);
    }
}

/// The checks moved in front of the loop: after the two sub-slices, all
/// three slices are known to have `n` elements.
#[inline(never)]
pub fn add_resliced(a: &[u64], b: &[u64], out: &mut [u64]) {
    let n = a.len();
    let (b, out) = (&b[..n], &mut out[..n]);
    for i in 0..n {
        out[i] = a[i].wrapping_add(b[i]);
    }
}

/// No index at all. The loop ends with the shortest slice, so a slice that
/// is too short gives fewer results instead of a panic.
#[inline(never)]
pub fn add_zipped(a: &[u64], b: &[u64], out: &mut [u64]) {
    for ((sum, x), y) in out.iter_mut().zip(a).zip(b) {
        *sum = x.wrapping_add(*y);
    }
}
