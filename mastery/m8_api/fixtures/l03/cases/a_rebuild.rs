//@ expect: E0451
//@ why: (A) a struct expression is allowed only when every field is public, whatever the fields are called; a user who can write one can also write `SensorId(0)` or `SensorId { raw: 0 }`

fn main() {
    let id = gauge::SensorId::new(4).expect("4 is a valid sensor number");
    // "A new `SensorId`, with every field taken from `id`." No field is
    // named, so this check works for a tuple struct and for a named field.
    let copy = gauge::SensorId { ..id };
    println!("{copy:?}");
}
