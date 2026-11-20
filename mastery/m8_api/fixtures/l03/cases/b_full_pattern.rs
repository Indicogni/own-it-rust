//@ expect: E0451 or E0638
//@ why: (B) a user who takes a `Reading` apart by naming its fields depends on those fields; users have the methods `sensor()` and `celsius()`

fn main() {
    let sensor = gauge::SensorId::new(4).expect("4 is a valid sensor number");
    let reading = gauge::Reading::new(sensor, 21.5);
    // Private fields give E0451. (E0638 is the answer of a struct that is
    // also `#[non_exhaustive]`.)
    let gauge::Reading { sensor, celsius } = reading;
    println!("{sensor:?} {celsius}");
}
