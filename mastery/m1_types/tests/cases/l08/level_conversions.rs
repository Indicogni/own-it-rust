//@ expect: run
//@ why: Level must convert to and from u8 and compare with u8 in both orders; each of these impls has a foreign trait, and two of them a foreign Self type.
use m1_types::l08_coherence::{Level, UnknownLevel};

fn main() {
    assert_eq!(u8::from(Level::Warn), 2);
    let number: u8 = Level::Debug.into();
    assert_eq!(number, 0);

    assert_eq!(Level::try_from(1_u8), Ok(Level::Info));
    assert_eq!(Level::try_from(3_u8), Err(UnknownLevel(3)));
    let parsed: Result<Level, _> = 255_u8.try_into();
    assert_eq!(parsed, Err(UnknownLevel(255)));

    assert!(Level::Warn == 2_u8);
    assert!(2_u8 == Level::Warn);
    assert!(Level::Info != 2_u8);
    assert!(0_u8 != Level::Info);

    for number in 0..=2_u8 {
        let level = Level::try_from(number).expect("0, 1 and 2 are levels");
        assert_eq!(u8::from(level), number);
    }
}
