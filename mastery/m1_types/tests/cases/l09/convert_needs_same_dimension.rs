//@ expect: E0271 | E0277
//@ why: a conversion must stay inside one dimension; the bound on `to` must compare the two dimensions (with `Dim = ...` the code is E0271).
use m1_types::l09_capstone::{Kilometres, Quantity, Seconds};

fn main() {
    let confused = Quantity::<Kilometres>::new(1.0).to::<Seconds>();
    println!("{confused}");
}
