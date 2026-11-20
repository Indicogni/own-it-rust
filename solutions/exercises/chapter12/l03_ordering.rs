//! LESSON 03 — Partial and total comparisons
//! Run: cargo test --test chapter12 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: If cmp returns Equal, what must equality
//! report? What does partial_cmp return for NaN?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why is lexicographic tie-breaking part of the public contract? Why
//! must PartialOrd delegate to Ord for a totally ordered type?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch12_scratch.rs):
//! Create a float wrapper using f64::total_cmp and bitwise equality in scratch. Test -0.0,
//! +0.0, and two NaN bit patterns; explain how that order differs from native float
//! equality.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::cmp::Ordering;
#[derive(Debug, PartialEq, Eq)]
struct Job {
    priority: u8,
    name: String,
}
impl Ord for Job {
    /// A. Ascending priority, then ascending name; equality uses both fields.
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.name.cmp(&other.name))
    }
}
impl PartialOrd for Job {
    /// B. This total order is always comparable and agrees with Ord.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // One source of truth: the total order lives in `cmp`.
        Some(self.cmp(other))
    }
}
/// C. Compare floats without inventing an ordering for NaN.
fn compare_readings(a: f64, b: f64) -> Option<Ordering> {
    a.partial_cmp(&b)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::run_op;

    fn job(priority: u8, name: &str) -> Job {
        Job {
            priority,
            name: name.into(),
        }
    }

    // Comparisons of `Job` run through `run_op` (see mod.rs). It fails with a
    // message if `cmp` and `partial_cmp` call each other forever, which happens
    // when each one is written in terms of the other.
    #[test]
    fn a_total_order_respects_primary_and_secondary_keys() {
        let sorted = run_op("sorting Jobs", || {
            let mut jobs = [job(2, "a"), job(1, "z"), job(1, "a")];
            jobs.sort();
            jobs
        });
        assert_eq!(
            sorted
                .iter()
                .map(|j| (j.priority, j.name.as_str()))
                .collect::<Vec<_>>(),
            [(1, "a"), (1, "z"), (2, "a")],
            "sort by priority first; equal priorities are ordered by name"
        );
    }
    #[test]
    fn b_partial_order_agrees_with_equality_and_total_order() {
        let (partial, total, with_itself) = run_op("comparing Jobs", || {
            let (a, b) = (job(1, "a"), job(2, "a"));
            (a.partial_cmp(&b), a.cmp(&b), a.partial_cmp(&a))
        });
        assert_eq!(partial, Some(total), "partial_cmp must agree with cmp");
        assert_eq!(
            with_itself,
            Some(Ordering::Equal),
            "a job is Equal to itself, because `==` says so"
        );
    }
    #[test]
    fn b_every_pair_gets_the_answer_cmp_gives() {
        let disagreements = run_op("comparing Jobs", || {
            let jobs = [job(1, "a"), job(1, "b"), job(2, "a"), job(2, "b")];
            let mut disagreements = Vec::new();
            for x in &jobs {
                for y in &jobs {
                    let (partial, total) = (x.partial_cmp(y), x.cmp(y));
                    if partial != Some(total) || (total == Ordering::Equal) != (x == y) {
                        disagreements.push(format!("{x:?} vs {y:?}: {partial:?} and {total:?}"));
                    }
                }
            }
            disagreements
        });
        assert!(
            disagreements.is_empty(),
            "partial_cmp, cmp and == must tell one story: {disagreements:#?}"
        );
    }
    #[test]
    fn c_float_order_is_partial() {
        assert_eq!(compare_readings(f64::NAN, 1.0), None);
        assert_eq!(compare_readings(-0.0, 0.0), Some(Ordering::Equal));
        assert_eq!(compare_readings(-1.0, 2.0), Some(Ordering::Less));
    }

    #[test]
    fn a_priority_wins_over_name() {
        let (low_vs_high, high_vs_low, less_than) = run_op("comparing Jobs", || {
            let (low, high) = (job(1, "zzz"), job(9, "a"));
            (low.cmp(&high), high.cmp(&low), low < high)
        });
        assert_eq!(low_vs_high, Ordering::Less);
        assert_eq!(high_vs_low, Ordering::Greater);
        assert!(less_than, "the < operator goes through PartialOrd");
    }

    #[test]
    fn c_float_order_handles_nan_on_either_side() {
        assert_eq!(compare_readings(1.0, f64::NAN), None);
        assert_eq!(compare_readings(f64::NAN, f64::NAN), None);
        assert_eq!(compare_readings(2.0, -1.0), Some(Ordering::Greater));
        assert_eq!(
            compare_readings(f64::NEG_INFINITY, f64::MIN),
            Some(Ordering::Less)
        );
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    enum Level {
        High,
        Low,
        Medium,
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Partial vs total comparison of the two zeros.
        let p1: (Option<Ordering>, Ordering) = (Some(Ordering::Equal), Ordering::Less);
        assert_eq!(
            p1,
            ((-0.0_f64).partial_cmp(&0.0), (-0.0_f64).total_cmp(&0.0)),
            "P1: partial_cmp vs total_cmp"
        );

        // P2. Derived Ord on an enum, after sorting.
        let mut levels = [Level::Low, Level::High, Level::Medium];
        levels.sort();
        let p2: [Level; 3] = [Level::High, Level::Low, Level::Medium];
        assert_eq!(p2, levels, "P2: derived enum order");

        // P3. str comparison.
        let p3: (bool, bool, bool) = (true, true, false);
        assert_eq!(
            p3,
            ("Zebra" < "apple", "10" < "9", "apple" < "Apple"),
            "P3: comparing strs with <"
        );

        // P4. Ties: which element do max_by_key and min_by_key return?
        let pairs = [(1, 'a'), (1, 'b')];
        let p4: (char, char) = ('b', 'a');
        assert_eq!(
            p4,
            (
                pairs.iter().max_by_key(|p| p.0).unwrap().1,
                pairs.iter().min_by_key(|p| p.0).unwrap().1
            ),
            "P4: tie-breaking in max and min"
        );
    }
}
