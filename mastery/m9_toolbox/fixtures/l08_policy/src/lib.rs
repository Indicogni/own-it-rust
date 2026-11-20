//! A small checksum over 4-byte words.

/// Adds up the input as little-endian 32-bit words, wrapping on overflow. A last
/// group of fewer than four bytes is padded with zeros.
pub fn checksum(bytes: &[u8]) -> u32 {
    let (words, rest) = bytes.as_chunks::<4>();
    let mut sum = 0_u32;
    for word in words {
        sum = sum.wrapping_add(u32::from_le_bytes(*word));
    }
    if !rest.is_empty() {
        let mut last = [0_u8; 4];
        last[..rest.len()].copy_from_slice(rest);
        sum = sum.wrapping_add(u32::from_le_bytes(last));
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_words() {
        assert_eq!(checksum(&[]), 0);
        assert_eq!(checksum(&[1, 0, 0, 0, 2, 0, 0, 0]), 3);
        assert_eq!(checksum(&[0xFF; 4]), u32::MAX);
        assert_eq!(checksum(&[0xFF, 0xFF, 0xFF, 0xFF, 1, 0, 0, 0]), 0);
        assert_eq!(checksum(&[1, 0, 0, 0, 5]), 6);
        assert_eq!(checksum(&[0, 1]), 256);
    }
}
