//@ expect: run
//@ why: quantities must add within one unit, convert within one dimension, scale by numbers, and combine into derived units.
use m1_types::l09_capstone::{Hours, Kilometres, Metres, Per, Quantity, Seconds, Unit};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn main() {
    let walk = Quantity::<Kilometres>::new(1.5).to::<Metres>() + Quantity::<Metres>::new(500.0);
    assert_eq!(walk.value(), 2000.0);
    assert_eq!(walk.to_string(), "2000 m");
    assert_eq!(Quantity::<Kilometres>::new(1.5).to_string(), "1.5 km");
    assert_eq!((walk - Quantity::new(0.5)).value(), 1999.5);

    assert_eq!((walk * 2.0).value(), 4000.0);
    assert_eq!((0.5 * walk).value(), 1000.0);

    // A derived unit: metres divided by seconds.
    let speed: Quantity<Per<Metres, Seconds>> = walk / Quantity::<Seconds>::new(100.0);
    assert_eq!(speed.value(), 20.0);
    assert_eq!(speed.to_string(), "20 m/s");
    let per_hour = speed.to::<Per<Kilometres, Hours>>();
    assert!(
        close(per_hour.value(), 72.0),
        "20 m/s is 72 km/h, got {per_hour}"
    );
    assert_eq!(<Per<Kilometres, Hours> as Unit>::symbol(), "km/h");

    // Speed times time is distance again, in the unit of the numerator.
    let covered: Quantity<Metres> = speed * Quantity::<Seconds>::new(10.0);
    assert_eq!(covered.value(), 200.0);
    let trip: Quantity<Kilometres> = per_hour * Quantity::<Hours>::new(0.5);
    assert!(close(trip.value(), 36.0));

    assert!(close(
        Quantity::<Hours>::new(2.0).to::<Seconds>().value(),
        7200.0
    ));
    assert_eq!(
        (Metres::IN_BASE, Kilometres::IN_BASE, Hours::IN_BASE),
        (1.0, 1000.0, 3600.0)
    );

    let legs = [Quantity::<Metres>::new(1.0), Quantity::new(2.5)];
    let by_reference: Quantity<Metres> = legs.iter().sum();
    let by_value: Quantity<Metres> = legs.into_iter().sum();
    assert_eq!(by_reference.value(), 3.5);
    assert_eq!(by_value, by_reference);
    assert!(Quantity::<Metres>::new(1.0) < Quantity::new(2.0));
    assert!(!format!("{walk:?}").is_empty());
}
