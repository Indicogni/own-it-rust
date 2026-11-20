//! Code review lesson: the structs that the teammate did not try.
//! This program must compile and run.

#![allow(dead_code)]

use m5_proc_macros::Reset;

#[derive(Reset, Debug, PartialEq)]
struct Pair(u8, #[reset(keep)] String, Vec<u8>);

#[derive(Reset)]
struct Unit;

#[derive(Reset, Debug, PartialEq)]
struct Window<'a, T: Clone = u8, const N: usize = 2>
where
    T: Default,
    [u8; N]: Default,
{
    first: T,
    rest: Vec<T>,
    grid: [u8; N],
    #[reset(keep)]
    title: &'a str,
    r#type: u8,
}

mod elsewhere {
    // This module has its own `Default`, and the trait `Reset` is not imported.
    pub trait Default {
        fn default() -> Self;
    }

    #[derive(m5_proc_macros::Reset)]
    pub struct Meter {
        pub level: u32,
    }
}

fn main() {
    let mut pair = Pair(7, "kept".into(), vec![1]);
    pair.reset();
    assert_eq!(pair, Pair(0, "kept".into(), vec![]));

    Unit.reset();

    let mut window: Window<'_, u16, 2> = Window {
        first: 5,
        rest: vec![6, 7],
        grid: [8, 9],
        title: "main",
        r#type: 9,
    };
    window.reset();
    assert_eq!(
        window,
        Window {
            first: 0,
            rest: vec![],
            grid: [0, 0],
            title: "main",
            r#type: 0
        }
    );

    let mut meter = elsewhere::Meter { level: 4 };
    m5_proc_macros::Reset::reset(&mut meter);
    assert_eq!(meter.level, 0);
}
