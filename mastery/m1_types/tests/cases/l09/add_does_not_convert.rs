//@ expect: E0308 | E0277
//@ why: metres plus kilometres must not compile either; an `Add` impl that converts the right-hand side silently hides the mistakes this library exists to catch. The caller writes `.to::<Metres>()`.
use m1_types::l09_capstone::{Kilometres, Metres, Quantity};

fn main() {
    let mixed = Quantity::<Metres>::new(500.0) + Quantity::<Kilometres>::new(1.5);
    println!("{mixed}");
}
