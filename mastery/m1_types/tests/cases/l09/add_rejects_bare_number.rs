//@ expect: E0308 | E0277
//@ why: a bare number has no unit, so it must not be added to a quantity.
use m1_types::l09_capstone::{Metres, Quantity};

fn main() {
    let unclear = Quantity::<Metres>::new(1.0) + 1.0;
    println!("{unclear}");
}
