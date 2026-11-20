//! LESSON 06, predictions: a program that panics three calls deep. Supplied; do
//! not edit. `tests/l06_process.rs` runs it with different environment variables.
//!
//! - `panicker`: prints one line to stdout, then panics.
//! - `panicker --dbg`: evaluates `dbg!(2 + 3)` first.
//! - `panicker --hook`: installs a panic hook that prints one line of its own.

#[expect(
    clippy::dbg_macro,
    reason = "this program exists to show what dbg! prints"
)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--hook") {
        std::panic::set_hook(Box::new(|info| {
            let location = info.location().map(|l| l.line()).unwrap_or(0);
            eprintln!("custom hook: panic on line {location}");
        }));
    }
    println!("starting");
    if args.iter().any(|arg| arg == "--dbg") {
        let five = dbg!(2 + 3);
        println!("five = {five}");
    }
    outer(3);
}

fn outer(depth: u32) {
    middle(depth);
}

fn middle(depth: u32) {
    inner(depth);
}

fn inner(depth: u32) {
    let limit: u32 = 2;
    assert!(depth <= limit, "depth {depth} is over the limit {limit}");
}
