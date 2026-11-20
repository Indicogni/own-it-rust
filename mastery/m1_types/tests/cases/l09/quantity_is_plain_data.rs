//@ expect: run
//@ why: the unit must live in the type only: a quantity is 8 bytes, Copy, Send and Sync for every unit, and unit types have no values.
use m1_types::implements;
use m1_types::l09_capstone::{Hours, Kilometres, Metres, Per, Quantity, Seconds};

fn main() {
    assert_eq!(size_of::<Quantity<Metres>>(), 8);
    assert_eq!(size_of::<Quantity<Per<Kilometres, Hours>>>(), 8);

    assert!(
        implements!(Quantity<Metres>: Copy + Send + Sync),
        "a quantity holds one f64, so it must be Copy, Send and Sync"
    );
    assert!(
        implements!(Quantity<Per<Metres, Seconds>>: Copy + Send + Sync),
        "a quantity of a derived unit must be Copy, Send and Sync too; \
         check the bounds of your impls and your marker"
    );

    assert_eq!(
        (size_of::<Option<Metres>>(), size_of::<Option<Seconds>>()),
        (0, 0),
        "a unit type must have no values (an Option of it can only be None)"
    );
    assert_eq!(
        size_of::<Option<Per<Metres, Seconds>>>(),
        0,
        "Per<A, B> must have no values either"
    );

    let original = Quantity::<Metres>::new(3.0);
    let copy = original;
    assert_eq!(original, copy);
}
