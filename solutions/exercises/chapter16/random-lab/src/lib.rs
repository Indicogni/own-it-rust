//! Chapter 16 optional lab: random choice, shuffling and sampling with `rand`.
//! Guide: README.md in this package. Concepts: "Lesson 03" in ../CONCEPTS.md.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use rand::{
    SeedableRng,
    rngs::StdRng,
    seq::{IndexedRandom, SliceRandom},
};
/// A. Choose one value of the slice with a fresh `StdRng` seeded with `seed`.
/// An empty slice gives None.
fn choose_one(values: &[i32], seed: u64) -> Option<i32> {
    let mut rng = StdRng::seed_from_u64(seed);
    values.choose(&mut rng).copied()
}
/// B. Shuffle the owned vector in place with a fresh `StdRng` seeded with
/// `seed`, and return it.
fn shuffled(mut values: Vec<i32>, seed: u64) -> Vec<i32> {
    let mut rng = StdRng::seed_from_u64(seed);
    values.shuffle(&mut rng);
    values
}
/// C. Sample min(amount, len) distinct positions with a fresh `StdRng` seeded
/// with `seed`, and return the values at those positions. The input values
/// are unique, so the result has no repeats.
fn sampled(values: &[i32], amount: usize, seed: u64) -> Vec<i32> {
    let mut rng = StdRng::seed_from_u64(seed);
    // `sample` picks distinct positions and stops at the slice's length, so
    // no `min(amount, len)` is needed here.
    values.sample(&mut rng, amount).copied().collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn a_choice_is_in_input_repeatable_and_empty_safe() {
        assert_eq!(choose_one(&[], 7), None);
        assert_eq!(choose_one(&[9], 7), Some(9));
        let values = [2, 5, 8, 11];
        let first = choose_one(&values, 42).unwrap();
        assert!(
            values.contains(&first),
            "the choice must be one of the values"
        );
        assert_eq!(
            choose_one(&values, 42),
            Some(first),
            "the same seed must give the same choice"
        );
    }
    #[test]
    fn a_choice_depends_on_the_seed() {
        // No test here expects one particular element. This one only checks
        // that the generator is really used: 64 seeds cannot all agree.
        let values = [2, 5, 8, 11];
        let chosen: HashSet<i32> = (0..64)
            .filter_map(|seed| choose_one(&values, seed))
            .collect();
        assert!(
            chosen.len() >= 2,
            "64 different seeds all chose the same element; the choice must \
             come from the seeded generator, not from a fixed position"
        );
    }
    #[test]
    fn b_shuffle_preserves_multiplicity_and_seeded_repeatability() {
        let input = vec![1, 2, 2, 3, 4];
        let output = shuffled(input.clone(), 99);
        assert_eq!(
            output,
            shuffled(input.clone(), 99),
            "the same seed must give the same order"
        );
        let mut sorted = output;
        sorted.sort();
        assert_eq!(
            sorted, input,
            "a shuffle moves values; it must not add, drop or change any"
        );
        assert!(shuffled(vec![], 0).is_empty());
    }
    #[test]
    fn b_shuffle_changes_the_order_for_some_seed() {
        let input: Vec<i32> = (0..10).collect();
        assert!(
            (0..64).any(|seed| shuffled(input.clone(), seed) != input),
            "64 different seeds all left ten values in their original order; \
             the vector was not shuffled"
        );
    }
    #[test]
    fn c_sample_is_unique_bounded_and_repeatable() {
        let input = [1, 2, 3, 4, 5];
        let result = sampled(&input, 3, 42);
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|n| input.contains(n)));
        assert_eq!(
            result.iter().collect::<HashSet<_>>().len(),
            3,
            "sampling without replacement never returns one position twice"
        );
        assert_eq!(
            result,
            sampled(&input, 3, 42),
            "the same seed must give the same sample"
        );
        assert_eq!(
            sampled(&input, 99, 7).len(),
            5,
            "asking for more than there is gives everything"
        );
        assert!(sampled(&input, 0, 7).is_empty());
        assert!(sampled(&[], 3, 7).is_empty());
    }
    #[test]
    fn c_sample_depends_on_the_seed() {
        let input = [1, 2, 3, 4, 5];
        let samples: HashSet<Vec<i32>> = (0..64)
            .map(|seed| {
                let mut sample = sampled(&input, 3, seed);
                sample.sort();
                sample
            })
            .collect();
        assert!(
            samples.len() >= 2,
            "64 different seeds all picked the same three values; the sample \
             must come from the seeded generator, not from the front of the slice"
        );
    }
}
