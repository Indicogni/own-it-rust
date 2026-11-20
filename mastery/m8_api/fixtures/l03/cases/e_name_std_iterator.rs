//@ expect: E0308
//@ why: (E) if `readings()` returns `std::slice::Iter`, the log can never store its readings in anything but a slice; a type of the library hides that choice

fn main() {
    let log = gauge::Log::new();
    let readings: std::slice::Iter<'_, gauge::Reading> = log.readings();
    println!("{}", readings.len());
}
