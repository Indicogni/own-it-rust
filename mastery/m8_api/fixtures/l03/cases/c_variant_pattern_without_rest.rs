//@ expect: E0638
//@ why: (C) a pattern that names all fields of `OutOfRange` stops compiling when a field is added; with `#[non_exhaustive]` on the variant, users must write `..`

use gauge::Fault;

fn main() {
    let fault = gauge::out_of_range(85.0);
    if let Fault::OutOfRange { limit } = fault {
        println!("{limit}");
    }
}
