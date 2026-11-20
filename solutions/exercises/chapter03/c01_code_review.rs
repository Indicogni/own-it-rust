//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter03 c01_
//! Then: cargo clippy --test chapter03
//! Read first: nothing new. This lesson reuses lessons 03, 09, 10 and 11.
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every overflow or Unicode hazard, and every unnecessary
//!    allocation or copy.
//! 2. Run the tests in debug AND `--release`. Compare what they found with
//!    your review. Does any bug behave differently in the two profiles?
//! 3. Fix it. You MAY change parameter types where the doc comment says so;
//!    the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

/// Mean of the readings, rounded down. None for an empty list. Any number
/// of readings, each up to u16::MAX, must work. You may change the parameter type.
fn mean(readings: &[u16]) -> Option<u16> {
    if readings.is_empty() {
        return None;
    }
    // A u64 total cannot overflow for any slice that fits in memory, and it
    // is the same width in debug and release builds.
    let mut total: u64 = 0;
    for &reading in readings {
        total += u64::from(reading);
    }
    // No `as u16` on the length: 65_536 readings would become a divisor of 0.
    let mean = total / readings.len() as u64;
    // The mean of u16 values is never above u16::MAX, so this is always Some.
    u16::try_from(mean).ok()
}

/// The uppercase first letter of each whitespace-separated word:
/// "ada lovelace" becomes "AL". Keep this signature.
fn initials(name: &str) -> String {
    let mut out = String::new();
    for word in name.split_whitespace() {
        // `chars().next()` reads one whole scalar, however many bytes it has.
        if let Some(first) = word.chars().next() {
            // `char::to_uppercase` yields one or more chars ('ß' gives "SS").
            // Appending them to `out` needs no String per word.
            out.extend(first.to_uppercase());
        }
    }
    out
}

/// Checksum: the sum of all bytes, wrapping modulo 256.
/// You may change the parameter type.
fn checksum(bytes: &[u8]) -> u8 {
    let mut sum: u8 = 0;
    for &byte in bytes {
        sum = sum.wrapping_add(byte);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;

    fn owned<T: Clone>(items: &[T]) -> Vec<T> {
        items.to_vec()
    }

    #[test]
    fn a_mean_of_small_readings() {
        assert_eq!(mean(&owned(&[1, 2, 4])), Some(2));
        assert_eq!(mean(&owned(&[7])), Some(7));
        assert_eq!(mean(&owned(&[])), None);
    }

    #[test]
    fn a_mean_does_not_overflow() {
        assert_eq!(mean(&owned(&[60_000, 60_000])), Some(60_000));
        assert_eq!(mean(&owned(&[u16::MAX; 300])), Some(u16::MAX));
        assert_eq!(mean(&owned(&[1; 70_000])), Some(1));
        // 70_000 maximal readings sum to about 4.59 billion: too big for u32.
        assert_eq!(
            mean(&owned(&[u16::MAX; 70_000])),
            Some(u16::MAX),
            "the total of 70_000 maximal readings does not fit in u32; use a wider total"
        );
    }

    #[test]
    fn b_initials_of_ascii_names() {
        assert_eq!(initials("ada lovelace"), "AL");
        assert_eq!(initials("  grace   hopper "), "GH");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn b_initials_respect_unicode() {
        assert_eq!(initials("émile zola"), "ÉZ");
        assert_eq!(initials("李 小龙"), "李小");
        // Some letters uppercase to more than one character.
        assert_eq!(initials("ßtraße x"), "SSX");
    }

    #[test]
    fn b_initials_build_one_string() {
        let name = "a b c d e f g h i j k l m n o p";
        let (out, allocations) = count(|| initials(name));
        assert_eq!(out, "ABCDEFGHIJKLMNOP");
        assert!(
            allocations <= 6,
            "16 words cost {allocations} allocations; append chars to one String"
        );
    }

    #[test]
    fn c_checksum_wraps() {
        assert_eq!(checksum(&owned(&[200, 100])), 44);
        assert_eq!(checksum(&owned(&[255; 256])), 0);
        assert_eq!(checksum(&owned(&[])), 0);
    }

    #[test]
    fn c_checksum_reads_without_copying() {
        let bytes = owned(&[7_u8; 1_000]);
        let (sum, allocations) = count(|| checksum(&bytes));
        assert_eq!(sum, (7_000 % 256) as u8);
        assert_eq!(
            allocations, 0,
            "a checksum only reads the bytes; borrow them instead of cloning the vector"
        );
    }
}
