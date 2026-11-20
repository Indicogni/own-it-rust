//! CRC-32, the checksum of SPEC 5.3.
//!
//! This is the CRC that zip, gzip and PNG use: polynomial `0xEDB88320` in
//! its reflected form, start value and final XOR `0xFFFFFFFF`. The table is
//! built at compile time, so there is nothing to initialise and nothing to
//! share between threads.

/// `TABLE[b]` is the effect of pushing the byte `b` through eight steps of
/// the bitwise algorithm.
const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut crc = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[byte] = crc;
        byte += 1;
    }
    table
};

/// A checksum that can be fed in pieces: a record's checksum covers part of
/// its header and its body, which recovery reads into two buffers.
#[derive(Clone, Copy, Debug)]
pub struct Crc32(u32);

impl Crc32 {
    pub const fn new() -> Crc32 {
        Crc32(0xFFFF_FFFF)
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            // The low byte of the state selects the table entry.
            let index = (self.0 ^ u32::from(byte)) & 0xFF;
            self.0 = (self.0 >> 8) ^ TABLE[index as usize];
        }
    }

    pub const fn finish(self) -> u32 {
        !self.0
    }
}

impl Default for Crc32 {
    fn default() -> Crc32 {
        Crc32::new()
    }
}

/// The checksum of `parts`, as if they were one slice.
pub fn checksum(parts: &[&[u8]]) -> u32 {
    let mut crc = Crc32::new();
    for part in parts {
        crc.update(part);
    }
    crc.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bitwise definition from SPEC 5.3, to check the table against.
    fn bitwise(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn check_values_from_the_spec() {
        assert_eq!(checksum(&[b"123456789"]), 0xCBF4_3926);
        assert_eq!(checksum(&[b""]), 0);
        assert_eq!(checksum(&[b"a"]), 0xE8B7_BE43);
    }

    #[test]
    fn table_agrees_with_the_bitwise_definition() {
        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        for len in [0, 1, 2, 7, 8, 255, 256, 1000] {
            assert_eq!(
                checksum(&[&data[..len]]),
                bitwise(&data[..len]),
                "length {len}"
            );
        }
    }

    #[test]
    fn pieces_give_the_same_result_as_the_whole() {
        let data = b"the quick brown fox";
        for cut in 0..=data.len() {
            assert_eq!(checksum(&[&data[..cut], &data[cut..]]), checksum(&[data]));
        }
    }

    #[test]
    fn any_two_bit_flips_are_detected() {
        let data = b"SET key value".to_vec();
        let good = checksum(&[&data]);
        let bits = data.len() * 8;
        for first in 0..bits {
            for second in first + 1..bits {
                let mut bad = data.clone();
                bad[first / 8] ^= 1 << (first % 8);
                bad[second / 8] ^= 1 << (second % 8);
                assert_ne!(checksum(&[&bad]), good, "bits {first} and {second}");
            }
        }
    }

    #[test]
    fn damage_within_32_bits_is_detected() {
        // Every way to change four neighbouring bytes of a short record
        // would be 2^32 cases per position; try a fixed sample of them.
        let data = b"SET key value".to_vec();
        let good = checksum(&[&data]);
        let mut pattern = 0x9E37_79B9u32;
        for at in 0..data.len() - 3 {
            for _ in 0..2000 {
                pattern = pattern.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                if pattern == 0 {
                    continue;
                }
                let mut bad = data.clone();
                for (byte, flip) in bad[at..at + 4].iter_mut().zip(pattern.to_le_bytes()) {
                    *byte ^= flip;
                }
                assert_ne!(checksum(&[&bad]), good, "pattern {pattern:#x} at byte {at}");
            }
        }
    }

    #[test]
    fn any_single_bit_flip_is_detected() {
        let data = b"SET key value".to_vec();
        let good = checksum(&[&data]);
        for bit in 0..data.len() * 8 {
            let mut bad = data.clone();
            bad[bit / 8] ^= 1 << (bit % 8);
            assert_ne!(checksum(&[&bad]), good, "flipping bit {bit} went unnoticed");
        }
    }
}
