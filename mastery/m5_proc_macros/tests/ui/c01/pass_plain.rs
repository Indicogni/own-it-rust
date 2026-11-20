//! Code review lesson: the one struct that the teammate tried.
//! This program must compile and run.

use m5_proc_macros::Reset;

#[derive(Reset, Debug, PartialEq)]
struct Counter {
    count: u32,
    #[reset(keep)]
    label: String,
    history: Vec<u32>,
}

fn main() {
    let mut counter = Counter {
        count: 3,
        label: "clicks".into(),
        history: vec![1, 2, 3],
    };
    counter.reset();
    assert_eq!(
        counter,
        Counter {
            count: 0,
            label: "clicks".into(),
            history: vec![]
        }
    );
}
