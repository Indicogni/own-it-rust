# Chapter 16 optional lab — Random elements

[Chapter guide](../README.md) · [Concepts](../CONCEPTS.md)

The standard library has no random number generator. Programs that need one use a crate, and the most common one is [`rand`](https://docs.rs/rand/0.10.3/rand/). This small package is separate from the chapter's other exercises so that those need no download. It pins `rand` 0.10.3. Do it when you can fetch the crate, or when it is already in your Cargo cache.

**What you practise:** choosing one element, shuffling, and sampling without repeats; a seeded generator; and how to test code whose output is random.

## The idea in short

A random number generator is a value with internal state. Each request changes the state, so every operation takes the generator as `&mut rng`.

- `StdRng::seed_from_u64(seed)` creates a generator whose whole future is fixed by `seed`. The same seed gives the same sequence, which makes a test repeatable. (`rand::rng()` gives a generator that is seeded differently in every run.)
- The trait [`IndexedRandom`](https://docs.rs/rand/0.10.3/rand/seq/trait.IndexedRandom.html) adds `choose` and `sample` to slices. `choose` returns `Option<&T>`: `None` for an empty slice. `sample(&mut rng, amount)` picks `amount` different positions, or all of them if there are fewer.
- The trait [`SliceRandom`](https://docs.rs/rand/0.10.3/rand/seq/trait.SliceRandom.html) adds `shuffle`, which reorders a mutable slice in place.
- [`StdRng`](https://docs.rs/rand/0.10.3/rand/rngs/struct.StdRng.html) is the crate's general-purpose generator. Its sequence for a given seed may change between versions of the crate and is not a data format.

Three operations are easy to confuse. *Sampling with replacement* means calling `choose` several times: one element can come up twice. *Sampling without replacement* is `sample`: each position at most once. A *shuffle* keeps every element and changes only the order.

## Predict, then implement

Before coding, write down your answers: Which of the three operations borrow the data and which change it? What does choosing from an empty slice return? Must a shuffled list differ from the original? Can two different seeds choose the same element?

Implement the three functions in `src/lib.rs` and run from the repository root:

```sh
cargo test --manifest-path exercises/chapter16/random-lab/Cargo.toml
cargo test --release --manifest-path exercises/chapter16/random-lab/Cargo.toml
```

Add `--offline` if the dependencies are already cached. The starter compiles, and its tests fail at `todo!()` until you implement the functions.

1. **Choose:** seed a `StdRng` and return a copy of one element through `IndexedRandom`.
2. **Shuffle:** reorder the vector through `SliceRandom`. Every value must still be there, the same number of times.
3. **Sample:** use `IndexedRandom::sample` to select at most `amount` different positions. The input values in this task are unique, so the returned values are unique too.

## How the tests avoid being fragile

No test expects one particular element or one particular order. The tests check properties that hold for every correct implementation: the chosen value is in the input, a shuffle keeps the same values, a sample has the requested size and no repeats, and the same seed gives the same result twice. Three more tests check that the generator is really used: over 64 different seeds, the results must not all be the same. A function that always returns the first element passes the property tests and fails these.

## Optional experiments (no test checks them)

4. Replace `StdRng::seed_from_u64(seed)` by `rand::rng()` in a scratch copy and run it several times. Explain why a test with one fixed expected output cannot work for it, and why a statistical expectation ("each element about a quarter of the time") makes a test that fails now and then.
5. Explain in your notes why `values[seed as usize % values.len()]` is not a random choice, even though it returns different elements for different seeds.

<details><summary>Hint 1</summary>

Import `SeedableRng` and the extension traits from `rand::seq`; the starter already does. Random operations take `&mut rng` because they advance the generator's state.

</details>
<details><summary>Hint 2</summary>

Create `StdRng::seed_from_u64(seed)`, then use `choose(&mut rng).copied()`, `shuffle(&mut rng)`, and `sample(&mut rng, amount).copied().collect()`. No index arithmetic is needed.

</details>

## Solution

The solved file is [solutions/exercises/chapter16/random-lab/src/lib.rs](../../../solutions/exercises/chapter16/random-lab/src/lib.rs). Use it after an honest attempt.
