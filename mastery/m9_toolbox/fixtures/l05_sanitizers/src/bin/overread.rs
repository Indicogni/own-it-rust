//! Reads one element past the end of a heap buffer.
//!
//! Without a sanitizer this program usually prints a number and exits with 0: the
//! read lands in memory the allocator happens to own. That is what makes this class
//! of bug dangerous.

fn sum_first(values: &[u32], count: usize) -> u32 {
    let mut sum = 0_u32;
    for i in 0..count {
        // SAFETY: wrong on purpose. Nothing checks `count <= values.len()`.
        sum = sum.wrapping_add(unsafe { *values.as_ptr().add(i) });
    }
    sum
}

fn main() {
    let values: Vec<u32> = vec![1, 2, 3, 4];
    // The caller passes the capacity it asked for, not the length it has.
    let total = sum_first(&values, 5);
    println!("total: {total}");
}
