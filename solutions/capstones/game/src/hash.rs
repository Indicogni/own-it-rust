//! FNV-1a, 64 bit: the one checksum used for pixels and for game state.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// A running FNV-1a hash. Feed it bytes; read the result with `finish`.
#[derive(Clone, Copy)]
pub(crate) struct Fnv(u64);

impl Fnv {
    pub(crate) fn new() -> Fnv {
        Fnv(OFFSET)
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(PRIME);
        }
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    pub(crate) fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Fnv;

    #[test]
    fn matches_the_published_test_vectors() {
        assert_eq!(Fnv::new().finish(), 0xcbf2_9ce4_8422_2325);
        let mut h = Fnv::new();
        h.bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        let mut h = Fnv::new();
        h.bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_f739_67e8);
    }
}
