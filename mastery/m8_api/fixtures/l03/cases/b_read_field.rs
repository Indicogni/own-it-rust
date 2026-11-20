//@ expect: E0616
//@ why: (B) a field that users can read can never be renamed, retyped or computed; `#[non_exhaustive]` alone does not hide it, only a private field does, and users have the methods `sensor()` and `celsius()`

fn main() {
    let sensor = gauge::SensorId::new(4).expect("4 is a valid sensor number");
    let reading = gauge::Reading::new(sensor, 21.5);
    // Without the parentheses these are the fields, not the methods.
    let celsius: f64 = reading.celsius;
    let sensor: gauge::SensorId = reading.sensor;
    println!("{sensor:?} {celsius}");
}
