//@ expect: run
//@ why: sum_by_ref must add references, for every type whose references can be added; it must not ask for Copy or Clone, or for addition by value.
use m1_types::l02_bounds_hrtb::{Cents, sum_by_ref};
use std::ops::Add;

/// A caller's own type: no Copy, no Clone, no addition by value.
#[derive(Debug, Default, PartialEq)]
struct Tally {
    marks: Vec<char>,
}

impl Add<&Tally> for &Tally {
    type Output = Tally;

    fn add(self, rhs: &Tally) -> Tally {
        Tally {
            marks: self.marks.iter().chain(&rhs.marks).copied().collect(),
        }
    }
}

fn main() {
    let prices = [Cents(250), Cents(199), Cents(1)];
    assert_eq!(sum_by_ref(&prices), Cents(450));
    assert_eq!(sum_by_ref::<Cents>(&[]), Cents(0));
    // The items were only borrowed.
    assert_eq!(prices[1], Cents(199));

    let tallies = vec![
        Tally { marks: vec!['a'] },
        Tally { marks: vec![] },
        Tally {
            marks: vec!['b', 'c'],
        },
    ];
    assert_eq!(sum_by_ref(&tallies).marks, ['a', 'b', 'c']);
    assert_eq!(tallies.len(), 3);

    assert_eq!(sum_by_ref(&[1.5_f64, 2.5]), 4.0);
    assert_eq!(sum_by_ref(&[u64::MAX - 1, 1]), u64::MAX);
}
