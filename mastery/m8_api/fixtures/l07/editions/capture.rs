//! Lesson 07, prediction P2. You do not need to edit this file.
//!
//! The same text is compiled twice: as edition 2021 and as edition 2024.

/// The even numbers below `limit`. The iterator owns everything it needs:
/// the body copies the number out of the reference.
fn evens(limit: &u32) -> impl Iterator<Item = u32> {
    (0..*limit).filter(|n| n % 2 == 0)
}

fn main() {
    let numbers = {
        let limit = 10;
        evens(&limit)
    };
    // `limit` is gone here. Is `numbers` still allowed to exist?
    assert_eq!(numbers.count(), 5);
}
