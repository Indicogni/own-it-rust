//@ expect: E0639
//@ why: (C) a user who builds `Fault::OutOfRange { limit }` stops compiling when the variant gets another field; `#[non_exhaustive]` on the variant reserves that right

fn main() {
    let fault = gauge::Fault::OutOfRange { limit: 85.0 };
    println!("{fault:?}");
}
