//! Effects: the particle pool and the screen-shake timer (SPEC section 12).
//!
//! Effects are driven by the simulation but never feed back into it. They
//! have their own random stream, so adding a particle can never change where
//! an enemy goes, and `Game::state_hash` does not cover them.

use crate::rng::Rng;

/// The pool never holds more than this many live particles.
pub(crate) const MAX_PARTICLES: usize = 64;

/// What a particle looks like; the renderer picks colours from this.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Look {
    Dust,
    Ember,
    Splash,
    Glint,
}

#[derive(Clone, Copy)]
pub(crate) struct Particle {
    pub(crate) x: i32,
    pub(crate) y: i32,
    vx: i32,
    vy: i32,
    /// Steps left to live; a live particle has `life >= 1`.
    pub(crate) life: u16,
    /// Life at birth, so the renderer can tell young from old.
    pub(crate) span: u16,
    pub(crate) look: Look,
}

const BLANK: Particle = Particle {
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    life: 0,
    span: 1,
    look: Look::Dust,
};

/// A fixed array plus a count: slots `0..live` are in use. Spawning writes
/// to slot `live`; a dead particle is overwritten by the last live one.
/// Nothing is ever allocated or freed after the game is built.
#[derive(Clone)]
pub(crate) struct Fx {
    rng: Rng,
    shake: u32,
    live: usize,
    pool: [Particle; MAX_PARTICLES],
}

impl Fx {
    pub(crate) fn new(seed: u64) -> Fx {
        Fx {
            // Any fixed function of the seed will do; it only has to differ
            // from the simulation stream.
            rng: Rng::new(seed ^ 0x00C0_FFEE_F1A5_C0DE),
            shake: 0,
            live: 0,
            pool: [BLANK; MAX_PARTICLES],
        }
    }

    /// Ages everything by one step. Runs first in a step, before the
    /// simulation gets the chance to spawn anything new.
    pub(crate) fn age(&mut self) {
        self.shake = self.shake.saturating_sub(1);
        let mut i = 0;
        while i < self.live {
            let p = &mut self.pool[i];
            p.life -= 1;
            if p.life == 0 {
                // Swap-remove: order does not matter for particles, and the
                // slot is filled without moving anything else.
                self.live -= 1;
                self.pool[i] = self.pool[self.live];
                continue;
            }
            p.x += p.vx;
            p.y += p.vy;
            p.vy += match p.look {
                Look::Dust => 6,
                Look::Glint => -4,
                Look::Ember | Look::Splash => 24,
            };
            i += 1;
        }
    }

    /// Spawns `n` particles around the subpixel point `(x, y)`. Particles
    /// that do not fit in the pool are dropped.
    pub(crate) fn burst(&mut self, look: Look, n: usize, x: i32, y: i32) {
        let (spread, lift) = match look {
            Look::Dust => (160, 96),
            Look::Ember => (420, 520),
            Look::Splash => (360, 640),
            Look::Glint => (260, 260),
        };
        for _ in 0..n {
            if self.live == MAX_PARTICLES {
                return;
            }
            let span = 10 + self.rng.below(31) as u16;
            let vx = self.rng.below(2 * spread + 1) as i32 - spread as i32;
            let vy = -(self.rng.below(lift + 1) as i32);
            self.pool[self.live] = Particle {
                x,
                y,
                vx,
                vy,
                life: span,
                span,
                look,
            };
            self.live += 1;
        }
    }

    /// Starts a shake of `ticks` steps unless a longer one is running.
    pub(crate) fn shake(&mut self, ticks: u32) {
        self.shake = self.shake.max(ticks);
    }

    /// The shake offset in pixels for the current step.
    pub(crate) fn shake_offset(&self) -> (i32, i32) {
        let t = self.shake as i32;
        if t == 0 {
            return (0, 0);
        }
        let amp = (t + 2) / 3;
        let dx = if t % 2 == 1 { amp } else { -amp };
        let dy = if (t / 2) % 2 == 1 { amp } else { -amp };
        (dx, dy)
    }

    pub(crate) fn particles(&self) -> &[Particle] {
        &self.pool[..self.live]
    }
}
