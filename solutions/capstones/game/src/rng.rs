//! The seeded generator from SPEC section 7 (SplitMix64).

/// A small, fast, seedable generator. The whole state is one `u64`, which
/// makes it cheap to copy, to hash and to store in a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Starts a stream. The seed becomes the state unchanged.
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// Advances the state and returns the next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        // Wrapping arithmetic is the algorithm, not an accident: say so, or
        // a debug build panics on the first overflow.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The high half of `next_u64`. The high bits are the better mixed ones.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// A value in `0..n` by multiply-and-shift. `below(0)` returns 0 and
    /// still consumes one draw, so the stream position never depends on `n`.
    pub fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }

    /// The current state, as hashed by `Game::state_hash`.
    pub fn state(&self) -> u64 {
        self.state
    }
}
