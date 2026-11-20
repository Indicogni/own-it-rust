//@ expect: run
//@ why: this program was written for clockwork 1.3.0; a minor release must not stop it from building or change what it does

use clockwork::{Timer, fmt_secs};

fn main() {
    let timer = Timer::start(10_000);
    let elapsed = timer.get_elapsed_ms(135_000);
    assert_eq!(elapsed, 125_000);
    assert_eq!(fmt_secs(elapsed / 1000), "2m 05s");

    // A function value of the old name still has the old type.
    let format: fn(u64) -> String = fmt_secs;
    assert_eq!(format(3600), "1h 00m 00s");
    assert_eq!(clockwork::total_elapsed_ms(&[timer, timer], 10_500), 1_000);
}
