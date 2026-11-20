//! The replay text format (SPEC section 13).

use std::fmt::Write as _;

use crate::{Input, Replay, ReplayError};

const HEADER: &str = "underkiln-replay 1";
/// The longest replay the format allows: a little over 46 hours.
const MAX_TICKS: u64 = 10_000_000;

impl Replay {
    /// The canonical text: header, seed, tick count, then one `count mask`
    /// line per run of equal inputs.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        // Writing to a `String` cannot fail, so the results are ignored.
        let _ = writeln!(out, "{HEADER}");
        let _ = writeln!(out, "seed {}", self.seed);
        let _ = writeln!(out, "ticks {}", self.inputs.len());
        for run in self.inputs.chunk_by(|a, b| a == b) {
            let _ = writeln!(out, "{} {}", run.len(), run[0].0);
        }
        out
    }

    /// Parses replay text. Accepts runs that are not merged, blank lines,
    /// `#` comments and Windows line endings.
    pub fn from_text(text: &str) -> Result<Replay, ReplayError> {
        let mut lines = text
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .enumerate()
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
            .map(|(i, line)| (i + 1, line));

        if lines.next().map(|(_, line)| line) != Some(HEADER) {
            return Err(ReplayError::BadHeader);
        }
        let seed = lines
            .next()
            .and_then(|(_, line)| line.strip_prefix("seed "))
            .and_then(decimal)
            .ok_or(ReplayError::BadSeed)?;
        let ticks = lines
            .next()
            .and_then(|(_, line)| line.strip_prefix("ticks "))
            .and_then(decimal)
            .filter(|&ticks| ticks <= MAX_TICKS)
            .ok_or(ReplayError::BadTicks)?;

        // Check every run and add the counts up before building anything, so
        // a file cannot make this function allocate more than MAX_TICKS.
        let mut runs = Vec::new();
        let mut found = 0u64;
        for (number, line) in lines {
            let (count, mask) = line
                .split_once(' ')
                .and_then(|(count, mask)| Some((decimal(count)?, decimal(mask)?)))
                .filter(|&(count, mask)| count > 0 && mask <= u64::from(Input::ALL))
                .ok_or(ReplayError::BadRun { line: number })?;
            found = found.saturating_add(count);
            runs.push((count, Input(mask as u8)));
        }
        if found != ticks {
            return Err(ReplayError::LengthMismatch {
                expected: ticks,
                found,
            });
        }
        let mut inputs = Vec::with_capacity(ticks as usize);
        for (count, input) in runs {
            inputs.extend(std::iter::repeat_n(input, count as usize));
        }
        Ok(Replay { seed, inputs })
    }
}

/// Plain decimal digits only. `str::parse` would also accept a leading `+`,
/// which the format does not allow.
fn decimal(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}
