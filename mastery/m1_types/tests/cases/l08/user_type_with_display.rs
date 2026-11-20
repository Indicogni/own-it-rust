//@ expect: run
//@ why: a user's type that implements Display must still be free to implement Describe in its own way; a blanket impl for every T: Display would forbid that.
use m1_types::l08_coherence::{Describe, Shown, describe_all};
use std::fmt;

struct Temperature(f64);

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} C", self.0)
    }
}

// The user's own choice: not the Display text.
impl Describe for Temperature {
    fn describe(&self) -> String {
        format!("temperature({})", self.0)
    }
}

fn main() {
    let reading = Temperature(21.5);
    assert_eq!(reading.describe(), "temperature(21.5)");
    assert_eq!(vec![Temperature(1.0)].describe(), "[temperature(1)]");

    // Opting in to the Display text, one value at a time.
    assert_eq!(Shown(&reading).describe(), "21.5 C");
    assert_eq!(Shown(3.25_f64).describe(), "3.25");
    assert_eq!(Shown("no quotes").describe(), "no quotes");
    assert_eq!(describe_all(&[Shown('a'), Shown('b')]), "a; b");
}
