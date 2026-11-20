//@ expect: E0277
//@ why: Unit must be sealed with a private supertrait, so that another crate cannot add a unit.
use m1_types::l09_capstone::{Length, Unit};

enum Furlongs {}

impl Unit for Furlongs {
    type Dim = Length;
    const IN_BASE: f64 = 201.168;
    fn symbol() -> String {
        "fur".to_string()
    }
}

fn main() {
    println!("{}", Furlongs::symbol());
}
