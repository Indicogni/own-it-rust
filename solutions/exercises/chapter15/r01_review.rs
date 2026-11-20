//! REVIEW 01 — Spaced retrieval: Chapters 10–14 inside Chapter 15
//! Run: cargo test --test chapter15 r01_
//!
//! Do this WITHOUT reopening Chapters 10–14 or their hints. Struggling to recall
//! is what makes the knowledge stick; look things up only after a real attempt,
//! and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (stateful closures, fallible conversions,
//! enum patterns) with this chapter's iterators.

/// A. (Ch14 move closures + Ch15 filter) Lazily yield each value that is greater
/// than every value before it; the first value always qualifies. Keep the
/// running maximum in state captured by a `move` closure passed to `filter`.
/// (The returned iterator borrows `values`. In edition 2024 an `impl Trait`
/// return type may do that without any extra lifetime syntax.)
fn record_highs(values: &[i32]) -> impl Iterator<Item = i32> {
    let mut best: Option<i32> = None;
    values.iter().copied().filter(move |&v| {
        // None means "no value seen yet", so even i32::MIN can be a high.
        let is_high = best.is_none_or(|highest| v > highest);
        if is_high {
            best = Some(v);
        }
        is_high
    })
}

#[derive(Debug, PartialEq)]
struct Percent(u8);

/// B. (Ch13 TryFrom) Accept 0..=100; otherwise Err("<value> is out of range").
impl TryFrom<i32> for Percent {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match u8::try_from(value) {
            Ok(percent) if percent <= 100 => Ok(Percent(percent)),
            _ => Err(format!("{value} is out of range")),
        }
    }
}

/// B. (Ch15 collect into Result) Convert every value with TryFrom. Return all
/// of them, or the first error.
fn parse_percents(values: &[i32]) -> Result<Vec<Percent>, String> {
    values
        .iter()
        .map(|&value| Percent::try_from(value))
        .collect()
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Push(i64),
    Add,
    Mul,
    Neg,
}

/// C. (Ch10 patterns + Ch15 try_fold) Evaluate the ops in order on a stack,
/// using try_fold. Push pushes its value; Add and Mul pop two values and push
/// the result; Neg replaces the top value with its negation. Return None if an
/// op lacks operands, on overflow (use checked arithmetic), or if the stack does
/// not end with exactly one value.
fn eval(ops: &[Op]) -> Option<i64> {
    let stack = ops.iter().try_fold(Vec::new(), |mut stack: Vec<i64>, op| {
        // `?` leaves the closure with None, and try_fold then stops.
        match *op {
            Op::Push(value) => stack.push(value),
            Op::Add => {
                let (right, left) = (stack.pop()?, stack.pop()?);
                stack.push(left.checked_add(right)?);
            }
            Op::Mul => {
                let (right, left) = (stack.pop()?, stack.pop()?);
                stack.push(left.checked_mul(right)?);
            }
            Op::Neg => {
                let top = stack.pop()?;
                stack.push(top.checked_neg()?);
            }
        }
        Some(stack)
    })?;
    match stack[..] {
        [value] => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_maximum_lives_in_the_closure() {
        let v = [3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(record_highs(&v).collect::<Vec<_>>(), [3, 4, 5, 9]);
        assert_eq!(
            record_highs(&v).take(2).collect::<Vec<_>>(),
            [3, 4],
            "every call starts with fresh state"
        );
        assert_eq!(
            record_highs(&[i32::MIN, i32::MIN]).collect::<Vec<_>>(),
            [i32::MIN]
        );
        assert_eq!(
            record_highs(&[2, 2, 2]).count(),
            1,
            "an equal value is not a new high"
        );
        assert_eq!(record_highs(&[]).count(), 0);
    }

    #[test]
    fn b_conversions_collect_or_fail_fast() {
        assert_eq!(
            parse_percents(&[0, 100, 55]),
            Ok(vec![Percent(0), Percent(100), Percent(55)])
        );
        assert_eq!(
            parse_percents(&[5, 101, -1]),
            Err("101 is out of range".to_string())
        );
        assert_eq!(Percent::try_from(-1), Err("-1 is out of range".to_string()));
        assert_eq!(parse_percents(&[]), Ok(vec![]));
    }

    #[test]
    fn c_stack_machine_checks_everything() {
        use Op::*;
        assert_eq!(eval(&[Push(2), Push(3), Add, Push(4), Mul]), Some(20));
        assert_eq!(eval(&[Push(5), Neg]), Some(-5));
        assert_eq!(eval(&[Add]), None);
        assert_eq!(eval(&[Push(1), Add]), None);
        assert_eq!(eval(&[Push(1), Push(2)]), None);
        assert_eq!(eval(&[]), None);
        assert_eq!(eval(&[Push(i64::MAX), Push(1), Add]), None);
        assert_eq!(eval(&[Push(i64::MIN), Neg]), None);
        assert_eq!(eval(&[Push(7), Push(2), Neg, Mul]), Some(-14));
    }

    #[derive(PartialEq, PartialOrd)]
    enum Level {
        Low,
        High,
    }

    /// P. Chapters 12–14 recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1. (Ch14) `add` owns its own `total`. What do the three expressions give?
        let mut total = 0;
        let mut add = move |n: i32| {
            total += n;
            total
        };
        add(5);
        let mut copy = add;
        let p1: (i32, i32, i32) = (6, 6, 0);
        assert_eq!(p1, (add(1), copy(1), total), "P1: (add(1), copy(1), total)");

        // P2. (Ch13) (300i32 into u8 with try_into fails?, 300i32 as u8)
        let checked: Result<u8, _> = 300i32.try_into();
        let p2: (bool, u8) = (true, 44);
        assert_eq!(
            p2,
            (checked.is_err(), 300i32 as u8),
            "P2: (try_into failed, value of the `as` cast)"
        );

        // P3. (Ch12) Derived ordering: (Low < High, (2, "a") < (1, "z"))
        let p3: (bool, bool) = (true, false);
        assert_eq!(
            p3,
            (Level::Low < Level::High, (2, "a") < (1, "z")),
            "P3: (Low < High, (2, \"a\") < (1, \"z\"))"
        );
    }
}
