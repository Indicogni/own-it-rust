// A playground for your own experiments: `cargo run --features playground`.
fn main() {
    let x = 250u8;
    let y = 8;
    println!("{}", add(x, y));
}

/// Adds two bytes, giving 0 instead of overflowing.
fn add(a: u8, b: u8) -> u8 {
    a.checked_add(b).unwrap_or(0)
}
