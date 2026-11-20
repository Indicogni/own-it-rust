//@ expect: run
//@ why: a program that uses the new names must build without any warning, also in a project that turns warnings into errors

#![deny(warnings)]

use clockwork::{Timer, format_duration};

fn main() {
    let timer = Timer::start(10_000);
    let elapsed = timer.elapsed_ms(135_000);
    assert_eq!(elapsed, 125_000);
    assert_eq!(format_duration(elapsed / 1000), "2m 05s");
    assert_eq!(format_duration(59), "59s");
    assert_eq!(Timer::start(500).elapsed_ms(100), 0);
}
