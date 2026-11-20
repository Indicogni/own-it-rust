//@ expect: E0308 | E0277
//@ why: metres plus seconds must not compile: `+` works between two quantities of the same unit only.
use m1_types::l09_capstone::{Metres, Quantity, Seconds};

fn main() {
    let nonsense = Quantity::<Metres>::new(1.0) + Quantity::<Seconds>::new(1.0);
    println!("{nonsense}");
}
