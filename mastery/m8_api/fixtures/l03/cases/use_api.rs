//@ expect: run
//@ why: this is a user of gauge 1.0.0 who does only what the library supports; it must keep working in every later 1.x release

use gauge::{Celsius, Fahrenheit, Fault, Log, Reading, Readings, SensorId, Unit};

/// A user's own function over any unit: the sealed trait can be named and
/// used as a bound.
fn label<U: Unit>(reading: &Reading) -> String {
    format!(
        "sensor {}: {}",
        reading.sensor().get(),
        gauge::show::<U>(reading)
    )
}

/// A user's `match` on a fault, with the wildcard arm that the library asks for.
fn advice(fault: &Fault) -> &'static str {
    match fault {
        Fault::Timeout => "check the cable",
        Fault::OutOfRange { .. } => "check the sensor",
        _ => "read the manual",
    }
}

/// The iterator type can be named, for example in a signature.
fn warmest(readings: Readings<'_>) -> Option<f64> {
    readings.map(|reading| reading.celsius()).reduce(f64::max)
}

fn main() {
    let id = SensorId::new(4).expect("4 is a valid sensor number");
    assert!(SensorId::new(0).is_none());

    let reading = Reading::new(id, 21.5);
    assert_eq!(label::<Celsius>(&reading), "sensor 4: 21.5 °C");
    assert_eq!(label::<Fahrenheit>(&reading), "sensor 4: 70.7 °F");
    assert_eq!(Fahrenheit::SYMBOL, "°F");

    assert_eq!(advice(&Fault::Timeout), "check the cable");
    let fault = gauge::out_of_range(85.0);
    assert_eq!(advice(&fault), "check the sensor");
    assert_eq!(gauge::describe(&fault), "beyond 85 °C");
    if let Fault::OutOfRange { limit, .. } = fault {
        assert_eq!(limit, 85.0);
    }

    let mut log = Log::new();
    log.push(reading);
    log.push(Reading::new(id, 19.0));
    assert_eq!(warmest(log.readings()), Some(21.5));
    let copy = log.readings().clone();
    assert_eq!(copy.count(), 2);
}
