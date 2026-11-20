//! LESSON 01 — Benchmark design: what a harness does, and what the optimizer undoes
//! Read first: CONCEPTS.md, "Lesson 01". Start after: chapters 14 and 15;
//! ecosystem E08 (the timing task) helps.
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l01_
//! Bench: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box
//! Lab: LABS.md, "Lab 1" runs that benchmark and shows what `black_box` is for.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a benchmark reports 0.3 ns for a
//! function that loops over 1,000 elements. Give two different reasons why the
//! number can be that small, and say how you would tell them apart.
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! You build the core of a benchmark harness. The tests drive it with a fake
//! clock whose time moves only when a test moves it, so they can check exactly
//! when your code reads the clock and what happens between two readings.
//! EXPLAIN afterward: Why does a harness time a whole batch of calls with two
//! clock readings instead of timing every call? What does `black_box` promise,
//! what does it not promise, and where does a benchmark need it? Why is the
//! median a better summary of timings than the mean?
//! JUDGMENT: For real measurements use criterion (or divan): it does the warm-up,
//! the batch sizing, the statistics and the comparison with the last run for
//! you. Write a harness yourself only to learn what one does, as here, or when
//! you need a number that is not time, such as the counts in the other lessons
//! of this module. A benchmark answers "which of these two is faster on this
//! input and this machine". It does not tell you where a program spends its
//! time; that is the job of a profiler (lesson 02).

use std::hint::black_box;
use std::time::Instant;

/// A source of time.
///
/// A real harness reads a monotonic clock. The tests use a fake one, so the
/// harness functions below take the clock as a parameter.
pub trait Clock {
    /// The current time in ticks. The value never goes backwards.
    fn now(&mut self) -> u64;
}

/// The real clock: one tick is one nanosecond since the clock was made.
pub struct WallClock {
    start: Instant,
}

impl WallClock {
    pub fn new() -> WallClock {
        WallClock {
            start: Instant::now(),
        }
    }
}

impl Default for WallClock {
    fn default() -> WallClock {
        WallClock::new()
    }
}

impl Clock for WallClock {
    fn now(&mut self) -> u64 {
        // More than 580 years of nanoseconds do not fit; stop there.
        u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// The function under test in `benches/l01_black_box.rs`: the sum of the
/// decimal digits of `n`. Supplied; you do not edit it.
///
/// `#[inline]` allows the compiler to copy the body into another crate, and
/// the benchmark is another crate. Only then can the compiler see the loop
/// next to the benchmark's input, which is what LABS.md asks you to observe.
/// Lesson 06 comes back to this attribute.
#[inline]
pub fn digit_sum(mut n: u64) -> u32 {
    let mut sum = 0;
    while n > 0 {
        sum += (n % 10) as u32;
        n /= 10;
    }
    sum
}

/// The second function under test in `benches/l01_black_box.rs`: the sum of
/// the squares of `values`, wrapping around. Supplied; you do not edit it.
#[inline]
pub fn sum_of_squares(values: &[u64]) -> u64 {
    values
        .iter()
        .fold(0, |sum, &value| sum.wrapping_add(value.wrapping_mul(value)))
}

/// A. Time one batch: call `routine` exactly `iters` times and return the
/// ticks that passed.
///
/// - Read the clock exactly twice: once before the first call and once after
///   the last one.
/// - Pass every result of `routine` through `std::hint::black_box` before it
///   is dropped, so that the optimizer has to assume somebody reads it.
pub fn time_batch<C: Clock, O>(clock: &mut C, iters: u64, mut routine: impl FnMut() -> O) -> u64 {
    let start = clock.now();
    for _ in 0..iters {
        black_box(routine());
    }
    clock.now() - start
}

/// B. Time a routine that consumes its input, without timing the work around
/// it. Return the ticks between the two clock readings.
///
/// - Call `setup` exactly `iters` times, all of them before the first reading.
/// - Reserve the room for the outputs before the first reading as well: the
///   timed region must not allocate.
/// - Between the two readings, call `routine` once per input, in the order in
///   which `setup` made the inputs. Pass inputs and outputs through `black_box`.
/// - Drop the outputs only after the second reading. Dropping can be slow (a
///   large `Vec`, a file), and it is not what the caller asked you to time.
pub fn time_batched<C: Clock, I, O>(
    clock: &mut C,
    iters: usize,
    mut setup: impl FnMut() -> I,
    mut routine: impl FnMut(I) -> O,
) -> u64 {
    let inputs: Vec<I> = (0..iters).map(|_| setup()).collect();
    let mut outputs: Vec<O> = Vec::with_capacity(iters);
    // The iterator owns the buffer of `inputs`. Keeping it alive until the
    // end also moves the freeing of that buffer out of the timed region.
    let mut inputs = inputs.into_iter();
    let start = clock.now();
    for input in inputs.by_ref() {
        outputs.push(black_box(routine(black_box(input))));
    }
    let elapsed = clock.now() - start;
    drop(outputs);
    elapsed
}

/// The largest batch that [`calibrate`] tries.
pub const MAX_BATCH: u64 = 1 << 20;

/// C. Find a batch size that is long enough to time. Try batches of
/// 1, 2, 4, 8, ... calls with [`time_batch`] and return the first size whose
/// batch takes at least `min_ticks`.
///
/// If a batch of [`MAX_BATCH`] calls is still too short, return `MAX_BATCH`.
/// A routine can take no measurable time at all (for example because the
/// optimizer removed it), and a harness must not search forever.
pub fn calibrate<C: Clock>(clock: &mut C, min_ticks: u64, mut routine: impl FnMut()) -> u64 {
    let mut iters = 1;
    while iters < MAX_BATCH && time_batch(clock, iters, &mut routine) < min_ticks {
        iters *= 2;
    }
    iters
}

/// A robust summary of timing samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// The smallest sample.
    pub min: u64,
    /// The middle sample.
    pub median: u64,
    /// The median absolute deviation: the median of the distances
    /// `|sample - median|`. It measures the spread and ignores outliers.
    pub mad: u64,
}

/// D. Summarize samples. `None` for no samples.
///
/// The median of an odd number of values is the middle one after sorting. For
/// an even number it is the mean of the two middle values, rounded down; your
/// code must not overflow when both are close to `u64::MAX`. Do not change
/// the caller's slice.
pub fn summarize(samples: &[u64]) -> Option<Summary> {
    /// The median of a sorted, non-empty slice.
    fn median(sorted: &[u64]) -> u64 {
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 1 {
            sorted[mid]
        } else {
            // `low + (high - low) / 2` cannot overflow; `(low + high) / 2` can.
            sorted[mid - 1] + (sorted[mid] - sorted[mid - 1]) / 2
        }
    }

    let mut values = samples.to_vec();
    values.sort_unstable();
    let min = *values.first()?;
    let middle = median(&values);
    // Reuse the buffer for the distances; one allocation serves both steps.
    for value in &mut values {
        *value = value.abs_diff(middle);
    }
    values.sort_unstable();
    Some(Summary {
        min,
        median: middle,
        mad: median(&values),
    })
}

/// The result of comparing two summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The candidate is faster than the base by more than the noise.
    Faster,
    /// The candidate is slower than the base by more than the noise.
    Slower,
    /// The difference is too small to tell from noise.
    WithinNoise,
}

/// How many deviations two medians must be apart before [`compare`] calls
/// them different.
pub const NOISE_FACTOR: u64 = 3;

/// E. Decide whether `candidate` differs from `base`.
///
/// The noise is `NOISE_FACTOR` times the larger of the two `mad` values. The
/// candidate is `Faster` if its median plus the noise is still below the base
/// median, `Slower` in the mirrored case, and `WithinNoise` otherwise (also
/// when the gap is exactly equal to the noise). Use saturating arithmetic.
pub fn compare(base: &Summary, candidate: &Summary) -> Verdict {
    let noise = NOISE_FACTOR.saturating_mul(base.mad.max(candidate.mad));
    if candidate.median.saturating_add(noise) < base.median {
        Verdict::Faster
    } else if base.median.saturating_add(noise) < candidate.median {
        Verdict::Slower
    } else {
        Verdict::WithinNoise
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::{measure, so_far};
    use crate::{guess, within};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Setup(usize),
        Read,
        Run(usize),
        Dropped(usize),
    }

    /// A clock for tests. Its time moves only when a test moves it. It also
    /// notes every reading, and how many allocations had been made by then.
    #[derive(Clone)]
    struct FakeClock {
        time: Rc<Cell<u64>>,
        events: Rc<RefCell<Vec<Event>>>,
        allocations_at_read: Rc<RefCell<Vec<usize>>>,
    }

    impl FakeClock {
        fn new() -> FakeClock {
            FakeClock {
                time: Rc::new(Cell::new(0)),
                // Room for every test below, so that noting an event never
                // allocates in the middle of a measurement.
                events: Rc::new(RefCell::new(Vec::with_capacity(256))),
                allocations_at_read: Rc::new(RefCell::new(Vec::with_capacity(256))),
            }
        }

        fn advance(&self, ticks: u64) {
            self.time.set(self.time.get() + ticks);
        }

        fn note(&self, event: Event) {
            self.events.borrow_mut().push(event);
        }

        fn reads(&self) -> usize {
            self.events
                .borrow()
                .iter()
                .filter(|event| **event == Event::Read)
                .count()
        }
    }

    impl Clock for FakeClock {
        fn now(&mut self) -> u64 {
            self.note(Event::Read);
            self.allocations_at_read
                .borrow_mut()
                .push(so_far().allocations);
            self.time.get()
        }
    }

    /// An output that notes when it is dropped.
    struct Output {
        id: usize,
        clock: FakeClock,
    }

    impl Drop for Output {
        fn drop(&mut self) {
            self.clock.note(Event::Dropped(self.id));
        }
    }

    #[test]
    fn a_batch_is_timed_with_two_readings() {
        let mut clock = FakeClock::new();
        let probe = clock.clone();
        let mut calls = 0;
        let elapsed = time_batch(&mut clock, 10, || {
            calls += 1;
            probe.advance(7);
        });
        assert_eq!(calls, 10, "the routine must run exactly `iters` times");
        assert_eq!(elapsed, 70, "10 calls of 7 ticks each");
        assert_eq!(
            probe.reads(),
            2,
            "read the clock once before the batch and once after it, not around every call"
        );
    }

    #[test]
    fn a_empty_batch_takes_no_time() {
        let mut clock = FakeClock::new();
        let probe = clock.clone();
        probe.advance(1_000);
        assert_eq!(time_batch(&mut clock, 0, || probe.advance(7)), 0);
        assert_eq!(probe.reads(), 2);
    }

    #[test]
    fn b_setup_and_drops_stay_outside_the_timed_region() {
        let mut clock = FakeClock::new();
        let probe = clock.clone();
        let mut made = 0;
        let elapsed = time_batched(
            &mut clock,
            3,
            || {
                probe.advance(1_000); // expensive setup
                probe.note(Event::Setup(made));
                made += 1;
                made - 1
            },
            |id| {
                probe.advance(5);
                probe.note(Event::Run(id));
                Output {
                    id,
                    clock: probe.clone(),
                }
            },
        );
        assert_eq!(elapsed, 15, "only the three calls of 5 ticks are timed");

        let events = probe.events.borrow().clone();
        let reads: Vec<usize> = (0..events.len())
            .filter(|&at| events[at] == Event::Read)
            .collect();
        assert_eq!(reads.len(), 2, "exactly two clock readings: {events:?}");
        let (first, second) = (reads[0], reads[1]);
        assert_eq!(
            events[..first],
            [Event::Setup(0), Event::Setup(1), Event::Setup(2)],
            "every setup call must come before the first clock reading: {events:?}"
        );
        assert_eq!(
            events[first + 1..second],
            [Event::Run(0), Event::Run(1), Event::Run(2)],
            "between the readings: one routine call per input, in setup order, and no drop: {events:?}"
        );
        let mut dropped = events[second + 1..].to_vec();
        dropped.sort_by_key(|event| format!("{event:?}"));
        assert_eq!(
            dropped,
            [Event::Dropped(0), Event::Dropped(1), Event::Dropped(2)],
            "every output must be dropped after the second clock reading: {events:?}"
        );
    }

    #[test]
    fn b_timed_region_does_not_allocate() {
        let mut clock = FakeClock::new();
        let probe = clock.clone();
        let (_, usage) = measure(|| {
            time_batched(
                &mut clock,
                100,
                || black_box(vec![0u8; 32]),
                |input| {
                    probe.advance(1);
                    input.len()
                },
            )
        });
        let marks = probe.allocations_at_read.borrow().clone();
        assert_eq!(marks.len(), 2, "exactly two clock readings");
        assert!(
            marks[0] >= 100,
            "the 100 inputs must exist before the first clock reading; only {} allocation(s) \
             had happened by then",
            marks[0]
        );
        assert_eq!(
            marks[1] - marks[0],
            0,
            "{} allocation(s) happened between the two clock readings; reserve the room for \
             the outputs before the first reading",
            marks[1] - marks[0]
        );
        assert!(
            usage.allocations <= 102,
            "one buffer for inputs, one for outputs"
        );
    }

    #[test]
    fn c_calibration_doubles_until_the_batch_is_long_enough() {
        let (iters, reads) = within(10, "calibrate", || {
            let mut clock = FakeClock::new();
            let probe = clock.clone();
            let iters = calibrate(&mut clock, 100, || probe.advance(3));
            (iters, probe.reads())
        });
        // Batches of 1, 2, 4, 8, 16 and 32 calls take 3 to 96 ticks.
        assert_eq!(iters, 64, "the first power of two with 3 * iters >= 100");
        assert_eq!(
            reads, 14,
            "seven batches were needed, and each batch reads the clock twice"
        );
    }

    #[test]
    fn c_calibration_accepts_a_long_first_call() {
        let iters = within(10, "calibrate", || {
            let mut clock = FakeClock::new();
            let probe = clock.clone();
            calibrate(&mut clock, 100, || probe.advance(100))
        });
        assert_eq!(iters, 1, "one call already takes `min_ticks`");
    }

    #[test]
    fn c_calibration_gives_up_on_a_routine_that_takes_no_time() {
        let iters = within(60, "calibrate with a routine that takes no time", || {
            let mut clock = FakeClock::new();
            calibrate(&mut clock, 100, || ())
        });
        assert_eq!(
            iters, MAX_BATCH,
            "a routine that takes no time must end the search at MAX_BATCH"
        );
    }

    #[test]
    fn d_summary_of_odd_and_even_sample_counts() {
        assert_eq!(summarize(&[]), None);
        assert_eq!(
            summarize(&[9]),
            Some(Summary {
                min: 9,
                median: 9,
                mad: 0
            })
        );
        // Sorted: 10 20 30 40 50. Distances from 30: 20 10 0 10 20.
        assert_eq!(
            summarize(&[30, 10, 50, 20, 40]),
            Some(Summary {
                min: 10,
                median: 30,
                mad: 10
            })
        );
        // Sorted: 1 2 4 9. Median 3. Distances: 2 1 1 6, sorted 1 1 2 6.
        assert_eq!(
            summarize(&[9, 1, 4, 2]),
            Some(Summary {
                min: 1,
                median: 3,
                mad: 1
            })
        );
    }

    #[test]
    fn d_one_outlier_does_not_move_the_summary() {
        let quiet = summarize(&[100, 101, 99, 100, 102, 100, 98]).unwrap();
        let noisy = summarize(&[100, 101, 99, 100, 102, 100, 9_000_000]).unwrap();
        assert_eq!(quiet.median, 100);
        assert_eq!(
            noisy.median, 100,
            "one slow sample must not move the median"
        );
        assert_eq!(noisy.mad, 1, "and it must not blow up the spread either");
    }

    #[test]
    fn d_median_of_huge_values_does_not_overflow() {
        let summary = summarize(&[u64::MAX, u64::MAX - 2]).unwrap();
        assert_eq!(summary.median, u64::MAX - 1);
        assert_eq!(summary.mad, 1);
    }

    #[test]
    fn e_verdicts_respect_the_noise() {
        let at = |median, mad| Summary {
            min: 0,
            median,
            mad,
        };
        assert_eq!(compare(&at(1_000, 10), &at(900, 5)), Verdict::Faster);
        assert_eq!(compare(&at(900, 5), &at(1_000, 10)), Verdict::Slower);
        assert_eq!(
            compare(&at(1_000, 10), &at(980, 2)),
            Verdict::WithinNoise,
            "a gap of 20 is inside the noise of 3 * 10"
        );
        assert_eq!(
            compare(&at(1_000, 10), &at(970, 10)),
            Verdict::WithinNoise,
            "a gap exactly equal to the noise is not a difference"
        );
        assert_eq!(compare(&at(1_000, 10), &at(969, 10)), Verdict::Faster);
        assert_eq!(compare(&at(500, 0), &at(500, 0)), Verdict::WithinNoise);
        assert_eq!(compare(&at(500, 0), &at(499, 0)), Verdict::Faster);
        assert_eq!(
            compare(&at(u64::MAX, u64::MAX), &at(0, 0)),
            Verdict::WithinNoise,
            "noise that large hides every difference; the arithmetic must saturate, not wrap"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Reading this clock is not free: `now` returns the time and then
        // costs 25 ticks. One call of the routine costs 5 ticks.
        struct Costly {
            time: u64,
        }
        impl Costly {
            fn now(&mut self) -> u64 {
                let seen = self.time;
                self.time += 25;
                seen
            }
            fn routine(&mut self) {
                self.time += 5;
            }
        }
        // Harness X reads the clock around every call and adds up the parts.
        let mut x = Costly { time: 0 };
        let mut total_x = 0;
        for _ in 0..100 {
            let start = x.now();
            x.routine();
            total_x += x.now() - start;
        }
        // Harness Y reads the clock once before and once after all the calls.
        let mut y = Costly { time: 0 };
        let start = y.now();
        for _ in 0..100 {
            y.routine();
        }
        let total_y = y.now() - start;
        // Ticks per call that X and Y report (integer division by 100).
        let p1: (u64, u64) = (30, 5);
        assert_eq!(
            p1,
            (total_x / 100, total_y / 100),
            "P1: ticks per call reported by (harness X, harness Y) (left: your prediction, right: actual)"
        );

        // P2. A coarse clock: it only shows multiples of 100 ticks. One call
        // of the routine takes 30 ticks.
        struct Coarse {
            time: u64,
        }
        impl Coarse {
            fn now(&self) -> u64 {
                self.time - self.time % 100
            }
        }
        // Ten calls, each one timed on its own.
        let mut coarse = Coarse { time: 0 };
        let mut samples = [0u64; 10];
        for sample in &mut samples {
            let start = coarse.now();
            coarse.time += 30;
            *sample = coarse.now() - start;
        }
        let zeros = samples.iter().filter(|&&sample| sample == 0).count();
        samples.sort_unstable();
        let median_single = (samples[4] + samples[5]) / 2;
        // Ten calls in one batch, timed together, divided by ten.
        let mut coarse = Coarse { time: 0 };
        let start = coarse.now();
        coarse.time += 10 * 30;
        let per_call_in_batch = (coarse.now() - start) / 10;
        // (samples that are 0, median of the ten samples, ticks per call from the batch)
        let p2: (usize, u64, u64) = (7, 0, 30);
        assert_eq!(
            p2,
            (zeros, median_single, per_call_in_batch),
            "P2: (single-call samples that are 0, their median, ticks per call from one batch) (left: your prediction, right: actual)"
        );

        // P3. Seven timings of the same routine; the scheduler interrupted one.
        let samples = [10u64, 11, 10, 12, 500, 11, 10];
        let mean = samples.iter().sum::<u64>() / samples.len() as u64;
        let mut sorted = samples;
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        let p3: (u64, u64) = (80, 11);
        assert_eq!(
            p3,
            (mean, median),
            "P3: (mean rounded down, median) of the samples (left: your prediction, right: actual)"
        );

        // P4. One call takes 7 ticks. A harness doubles the batch size,
        // starting at 1, until one batch takes at least 1,000 ticks.
        let mut batch = 1u64;
        while batch * 7 < 1_000 {
            batch *= 2;
        }
        let p4: u64 = 256;
        assert_eq!(
            p4, batch,
            "P4: the batch size the harness settles on (left: your prediction, right: actual)"
        );
    }
}
