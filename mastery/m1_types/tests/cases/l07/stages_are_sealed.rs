//@ expect: E0277
//@ why: Stage must be sealed with a private supertrait, so that another crate cannot invent a stage.
use m1_types::l07_sealed_typestate::Stage;

struct Forged;

impl Stage for Forged {
    const NAME: &'static str = "forged";
}

fn main() {
    println!("{}", Forged::NAME);
}
