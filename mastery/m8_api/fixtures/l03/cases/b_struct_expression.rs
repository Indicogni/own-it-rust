//@ expect: E0451 or E0639
//@ why: (B) a user who builds `Reading { .. }` from all its fields stops compiling when a field is added; users have `Reading::new`

fn main() {
    let sensor = gauge::SensorId::new(4).expect("4 is a valid sensor number");
    // Private fields give E0451. (E0639 is the answer of a struct that is
    // also `#[non_exhaustive]`.)
    let reading = gauge::Reading {
        sensor,
        celsius: 21.5,
    };
    println!("{reading:?}");
}
