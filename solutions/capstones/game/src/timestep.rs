//! The fixed-timestep accumulator from SPEC section 6.

/// Turns irregular frame times into a whole number of equal simulation
/// steps, and remembers the left-over time for interpolation.
#[derive(Clone, Copy, Debug)]
pub struct FixedStep {
    step: u64,
    max_steps: u32,
    acc: u64,
}

impl FixedStep {
    /// `step_micros` is the length of one simulation step. `max_steps` caps
    /// how many steps one call to `advance` may ask for.
    ///
    /// # Panics
    /// Panics if `step_micros` is zero.
    pub fn new(step_micros: u64, max_steps: u32) -> FixedStep {
        assert!(step_micros > 0, "a simulation step must take some time");
        FixedStep {
            step: step_micros,
            max_steps,
            acc: 0,
        }
    }

    /// Adds `elapsed_micros` of real time and returns how many simulation
    /// steps to run now.
    pub fn advance(&mut self, elapsed_micros: u64) -> u32 {
        self.acc = self.acc.saturating_add(elapsed_micros);
        let due = self.acc / self.step;
        if due > u64::from(self.max_steps) {
            // The machine cannot keep up. Run the cap and forget the rest,
            // otherwise each slow frame would make the next one slower.
            self.acc = 0;
            self.max_steps
        } else {
            self.acc -= due * self.step;
            due as u32
        }
    }

    /// How far the left-over time reaches into the next step, in 256ths.
    /// Always in `0..=255`.
    pub fn alpha(&self) -> u32 {
        // acc < step here, so the product needs more than 64 bits only for
        // absurd step lengths; u128 makes that impossible instead of rare.
        (u128::from(self.acc) * 256 / u128::from(self.step)) as u32
    }
}
