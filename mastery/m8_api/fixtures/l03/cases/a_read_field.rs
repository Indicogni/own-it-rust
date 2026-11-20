//@ expect: E0616 or E0609
//@ why: (A) if users can read `id.0`, the field's type is part of the public interface; they have `get()` for this

fn main() {
    let id = gauge::SensorId::new(4).expect("4 is a valid sensor number");
    // A private field of a tuple struct gives E0616. A struct with a named
    // field has no field `0` (E0609); the next case checks that one.
    let raw: u32 = id.0;
    println!("{raw}");
}
