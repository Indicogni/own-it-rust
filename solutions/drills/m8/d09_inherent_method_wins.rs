//! DRILL M8-09 · A new inherent method with a familiar name
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a type has an inherent method and a trait method with one name. Which one does `stock.count()` call?

/// Stands for version 1.1.0 of a library crate.
mod inventory {
    pub struct Stock {
        pub boxes: u32,
        pub per_box: u32,
    }

    impl Stock {
        /// New in 1.1.0: the number of boxes.
        pub fn count(&self) -> u32 { // KEEP
            self.boxes // KEEP
        } // KEEP
    }
}

use inventory::Stock;

/// A user's trait, written for 1.0.0: the number of single pieces.
trait Pieces { // KEEP
    fn count(&self) -> u32; // KEEP
} // KEEP

impl Pieces for Stock { // KEEP
    fn count(&self) -> u32 { // KEEP
        self.boxes * self.per_box // KEEP
    } // KEEP
} // KEEP

fn main() {
    let stock = Stock { boxes: 3, per_box: 12 }; // KEEP
    // The inherent method would win; name the trait whose method is meant.
    let pieces = Pieces::count(&stock);
    assert_eq!(pieces, 36); // KEEP
}
