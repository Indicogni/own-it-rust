//! Lesson 06, task E: an example program for `slugger`.
//!
//! Run it from `fixtures/l06/slugger/`:
//!
//!     cargo run --example titles -- "Rust 2024" "Why  SemVer?"

fn main() {
    let titles: Vec<String> = std::env::args().skip(1).collect();
    if titles.is_empty() {
        // Without arguments, show one result, so that a first run prints something.
        println!("{}", slugger::slug("Hello, World!"));
    }
    for title in &titles {
        println!("{}", slugger::slug(title));
    }
}
