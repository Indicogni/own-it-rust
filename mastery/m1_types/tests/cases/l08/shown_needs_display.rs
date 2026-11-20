//@ expect: E0599 | E0277
//@ why: Shown<T> describes a value through Display, so a T without Display must be rejected at compile time.
use m1_types::l08_coherence::{Describe, Shown};

fn main() {
    let numbers = Shown(vec![1_u8, 2]);
    println!("{}", numbers.describe());
}
