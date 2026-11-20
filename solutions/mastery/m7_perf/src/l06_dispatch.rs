//! LESSON 06 — Static and dynamic dispatch, inlining, and code size
//! Read first: CONCEPTS.md, "Lesson 06". Start after: lesson 05; chapters 11
//! (traits and generics) and 14 (closures). Mastery M1, lesson 05 (what a
//! trait object is made of) helps.
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l06_
//! Lab: LABS.md, "Lab 6" counts the copies that generic code produces.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a function `fn log<T: Display>(x: T)`
//! of 40 lines is called with 30 different types. How many copies of its
//! machine code does the program contain? How many if it took `&dyn Display`?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! From this lesson on, a task is a contract in prose. The file holds only the
//! names that the tests use, with placeholder bodies; fields, helper types and
//! private functions are your design.
//! EXPLAIN afterward: What does a call through `dyn Trait` cost that a generic
//! call does not, and what does a generic function cost that a `dyn` call does
//! not? Why does one virtual call per batch remove most of the first cost?
//! Why is `Each<F>` with a closure that captures nothing zero bytes large?
//! JUDGMENT: Generics give the optimizer everything: it can inline the call
//! and specialize the loop. The price is one copy of the code per type, which
//! costs compile time and binary size. `dyn` gives one copy and a call that
//! cannot be inlined. Use generics for small, hot code; use `dyn` (or a
//! non-generic inner function) for large, cold code and for collections of
//! mixed types; use an enum when the set of types is closed and you own it.
//! Do not sprinkle `#[inline]`: within a crate the compiler decides well by
//! itself. Add it to small functions that other crates call in hot loops, and
//! add `#[inline(never)]` or `#[cold]` when a profile or the generated code
//! shows that a rare path bloats a hot one.

/// One step of a pipeline over `u64` values.
pub trait Stage {
    /// Transforms one value.
    fn apply(&self, value: u64) -> u64;

    /// A. Transforms every value of `values` in place.
    ///
    /// Write the default body: it replaces each value by `self.apply(value)`.
    /// A type that implements `Stage` gets this method for free and may
    /// override it. Inside the body, `Self` is one concrete type, so the call
    /// of `apply` is a direct call that the compiler can inline, also when the
    /// caller reached `apply_all` through a `dyn Stage`.
    fn apply_all(&self, values: &mut [u64]) {
        for value in values {
            *value = self.apply(*value);
        }
    }
}

/// A. Send all `values` through all `stages`, in order: afterwards each value
/// is `stageN(...stage1(stage0(value)))`.
///
/// Budget: one virtual call per stage, however many values there are.
pub fn run_pipeline(stages: &[Box<dyn Stage>], values: &mut [u64]) {
    // Stage by stage, not value by value: the dynamic dispatch happens once
    // per stage, and the loop over the values runs inside the concrete type.
    for stage in stages {
        stage.apply_all(values);
    }
}

/// B. The 32-bit FNV-1a checksum of some bytes: start with `0x811c9dc5`; for
/// every byte, xor the byte into the value and then multiply the value by
/// `0x01000193`, wrapping around.
///
/// `checksum` is the convenient entrance: callers pass a `&str`, a `String`,
/// a `Vec<u8>`, an array, and so on, and the compiler makes one copy of this
/// function for every such type. Keep that copy as small as possible: convert
/// the input to a byte slice exactly once and hand it to `checksum_bytes`,
/// which is not generic and therefore exists once.
pub fn checksum<B: AsRef<[u8]>>(input: B) -> u32 {
    checksum_bytes(input.as_ref())
}

/// B. The checksum of `bytes`; see [`checksum`]. All the work happens here.
pub fn checksum_bytes(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// C. One step of a calculation, from a closed set of four kinds.
///
/// Contract:
/// - `Op::parse(code, operand)` makes a step, or returns `None` for an
///   unknown code:
///   - `'+'`: add `operand`, wrapping around;
///   - `'*'`: multiply by `operand`, wrapping around;
///   - `'^'`: xor with `operand`;
///   - `'<'`: rotate left by `operand % 64` bits.
/// - `Op` implements [`Stage`]; `apply` performs the step.
/// - `Op` is `Copy` and takes at most 16 bytes. Making an `Op` never
///   allocates: there is no `Box` and no trait object behind it.
///
/// The representation is yours. The derives are part of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add(u64),
    Mul(u64),
    Xor(u64),
    /// The number of bits, already reduced to `0..64`.
    RotateLeft(u32),
}

impl Op {
    pub fn parse(code: char, operand: u64) -> Option<Op> {
        match code {
            '+' => Some(Op::Add(operand)),
            '*' => Some(Op::Mul(operand)),
            '^' => Some(Op::Xor(operand)),
            // `operand % 64` is below 64, so it fits a `u32` exactly.
            '<' => Some(Op::RotateLeft((operand % 64) as u32)),
            _ => None,
        }
    }
}

impl Stage for Op {
    fn apply(&self, value: u64) -> u64 {
        match *self {
            Op::Add(operand) => value.wrapping_add(operand),
            Op::Mul(operand) => value.wrapping_mul(operand),
            Op::Xor(operand) => value ^ operand,
            Op::RotateLeft(bits) => value.rotate_left(bits),
        }
    }
}

/// C. Send all `values` through all `ops`, in order. No trait object is
/// involved: every call is resolved at compile time.
pub fn run_ops(ops: &[Op], values: &mut [u64]) {
    // `Op::apply_all` is the default method, compiled for `Op`: the `match`
    // sits in a plain loop over the values, where the optimizer can move the
    // decision out of the loop.
    for op in ops {
        op.apply_all(values);
    }
}

/// D. An adapter that turns a function `u64 -> u64` into a [`Stage`].
///
/// A teammate wrote it with a boxed closure: every `Each` allocates, takes 16
/// bytes, and calls its function through a pointer. It works; the behavior
/// tests pass.
///
/// Redesign it so that `Each<F>` stores the function itself:
/// - `Each<F>` is exactly as large as `F`. A closure that captures nothing
///   has size zero, and then so does the `Each`.
/// - `Each::new` does not allocate.
/// - `Each<F>` implements `Stage` whenever `F: Fn(u64) -> u64`.
///
/// You may change the fields, the bounds and the bodies; keep the names.
pub struct Each<F> {
    function: F,
}

impl<F: Fn(u64) -> u64> Each<F> {
    pub fn new(function: F) -> Each<F> {
        Each { function }
    }
}

impl<F: Fn(u64) -> u64> Stage for Each<F> {
    fn apply(&self, value: u64) -> u64 {
        // `F` is a concrete type here, so this is a direct call.
        (self.function)(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;
    use std::cell::Cell;
    use std::hint::black_box;
    use std::mem::{size_of, size_of_val};
    use std::rc::Rc;

    /// A stage that counts how it is called.
    struct Probe {
        add: u64,
        single_calls: Rc<Cell<u64>>,
        batch_calls: Rc<Cell<u64>>,
    }

    impl Stage for Probe {
        fn apply(&self, value: u64) -> u64 {
            self.single_calls.set(self.single_calls.get() + 1);
            value + self.add
        }
        fn apply_all(&self, values: &mut [u64]) {
            self.batch_calls.set(self.batch_calls.get() + 1);
            for value in values {
                *value += self.add;
            }
        }
    }

    /// A stage that implements only `apply`.
    struct Triple;

    impl Stage for Triple {
        fn apply(&self, value: u64) -> u64 {
            value * 3
        }
    }

    /// What a pipeline must compute, written in the most direct way.
    fn reference(stages: &[&dyn Stage], values: &[u64]) -> Vec<u64> {
        values
            .iter()
            .map(|&value| stages.iter().fold(value, |value, stage| stage.apply(value)))
            .collect()
    }

    #[test]
    fn a_default_apply_all_uses_apply() {
        let mut values = [1, 2, 3, 0];
        Triple.apply_all(&mut values);
        assert_eq!(values, [3, 6, 9, 0]);
        Triple.apply_all(&mut []);
    }

    #[test]
    fn a_pipeline_applies_the_stages_in_order() {
        let stages: Vec<Box<dyn Stage>> = vec![
            Box::new(Triple),
            Box::new(Each::new(|value| value + 1)),
            Box::new(Triple),
        ];
        let mut values = [0, 1, 10];
        run_pipeline(&stages, &mut values);
        assert_eq!(values, [3, 12, 93], "(v * 3 + 1) * 3");
        run_pipeline(&[], &mut values);
        assert_eq!(values, [3, 12, 93], "no stages change nothing");
    }

    #[test]
    fn a_one_virtual_call_per_stage() {
        let single_calls = Rc::new(Cell::new(0));
        let batch_calls = Rc::new(Cell::new(0));
        let stages: Vec<Box<dyn Stage>> = (1..=3)
            .map(|add| {
                Box::new(Probe {
                    add,
                    single_calls: Rc::clone(&single_calls),
                    batch_calls: Rc::clone(&batch_calls),
                }) as Box<dyn Stage>
            })
            .collect();
        let mut values: Vec<u64> = (0..1_000).collect();
        run_pipeline(&stages, &mut values);
        assert!(values.iter().copied().eq(6..1_006));
        assert_eq!(
            single_calls.get(),
            0,
            "run_pipeline called `apply` through a trait object {} times for 1,000 values \
             and 3 stages; hand each stage the whole slice instead",
            single_calls.get()
        );
        assert_eq!(batch_calls.get(), 3, "one `apply_all` call per stage");
    }

    #[test]
    fn b_checksums_match_the_published_values() {
        assert_eq!(checksum_bytes(b""), 0x811c_9dc5);
        assert_eq!(checksum_bytes(b"a"), 0xe40c_292c);
        assert_eq!(checksum_bytes(b"foobar"), 0xbf9c_f968);
        assert_eq!(checksum("foobar"), 0xbf9c_f968);
        assert_eq!(checksum(String::from("foobar")), 0xbf9c_f968);
        assert_eq!(checksum(Vec::from(*b"foobar")), 0xbf9c_f968);
        assert_eq!(checksum(*b"a"), 0xe40c_292c);
        assert_eq!(checksum(&b"a"[..]), 0xe40c_292c);
    }

    #[test]
    fn b_the_generic_shell_converts_once() {
        /// Counts how often it is asked for its bytes.
        struct Counted<'a> {
            bytes: &'a [u8],
            conversions: &'a Cell<u32>,
        }
        impl AsRef<[u8]> for Counted<'_> {
            fn as_ref(&self) -> &[u8] {
                self.conversions.set(self.conversions.get() + 1);
                self.bytes
            }
        }
        let conversions = Cell::new(0);
        let input = Counted {
            bytes: b"foobar",
            conversions: &conversions,
        };
        assert_eq!(checksum(input), 0xbf9c_f968);
        assert_eq!(
            conversions.get(),
            1,
            "checksum called as_ref {} times; convert once, then pass the slice to the \
             non-generic function",
            conversions.get()
        );
        // The core exists once: it coerces to a plain function pointer.
        let core: fn(&[u8]) -> u32 = checksum_bytes;
        assert_eq!(core(b"a"), 0xe40c_292c);
    }

    #[test]
    fn c_ops_compute_the_four_steps() {
        let op = |code, operand| Op::parse(code, operand).expect("a known code");
        assert_eq!(op('+', 5).apply(10), 15);
        assert_eq!(op('+', 2).apply(u64::MAX), 1, "addition wraps");
        assert_eq!(op('*', 3).apply(7), 21);
        assert_eq!(op('*', 2).apply(1 << 63), 0, "multiplication wraps");
        assert_eq!(op('^', 0b1100).apply(0b1010), 0b0110);
        assert_eq!(
            op('<', 4).apply(0xF000_0000_0000_000F),
            0x0000_0000_0000_00FF
        );
        assert_eq!(op('<', 64).apply(12345), 12345, "64 bits is a full turn");
        assert_eq!(op('<', 68), op('<', 4), "the bit count is taken modulo 64");
        assert_eq!(Op::parse('?', 1), None);
        assert_eq!(Op::parse('-', 1), None);
    }

    #[test]
    fn c_ops_run_like_boxed_stages() {
        let spec = [('+', 7u64), ('*', 31), ('^', 0xFFFF), ('<', 13), ('+', 1)];
        let ops: Vec<Op> = spec
            .iter()
            .map(|&(code, operand)| Op::parse(code, operand).unwrap())
            .collect();
        let dynamic: Vec<&dyn Stage> = ops.iter().map(|op| op as &dyn Stage).collect();
        let input: Vec<u64> = (0..200).map(|i| i * 1_000_003).collect();
        let mut values = input.clone();
        run_ops(&ops, &mut values);
        assert_eq!(values, reference(&dynamic, &input));
        run_ops(&[], &mut values);
        assert_eq!(values, reference(&dynamic, &input));
    }

    #[test]
    fn c_ops_are_small_plain_values() {
        fn is_copy<T: Copy>() {}
        is_copy::<Op>();
        assert!(
            size_of::<Op>() <= 16,
            "an Op takes {} bytes; the budget is 16",
            size_of::<Op>()
        );
        let codes = ['+', '*', '^', '<'];
        let (ops, allocations) = count(|| {
            let ops: Vec<Op> = (0..1_000usize)
                .map(|i| Op::parse(codes[i % 4], i as u64).unwrap())
                .collect();
            black_box(ops)
        });
        assert_eq!(ops.len(), 1_000);
        assert!(
            allocations <= 1,
            "building 1,000 ops made {allocations} allocations; only the Vec may allocate, \
             an Op itself owns no heap memory"
        );
    }

    #[test]
    fn d_each_is_a_stage() {
        let offset = 10;
        let add = Each::new(move |value| value + offset);
        assert_eq!(add.apply(5), 15);
        let mut values = [1, 2, 3];
        add.apply_all(&mut values);
        assert_eq!(values, [11, 12, 13]);
        let as_object: &dyn Stage = &add;
        assert_eq!(as_object.apply(0), 10);
    }

    #[test]
    fn d_each_stores_the_function_inline() {
        let plain = Each::new(|value: u64| value + 1);
        assert_eq!(
            size_of_val(&plain),
            0,
            "a closure that captures nothing has size 0, and an Each that stores it inline \
             has size 0 too; this one takes {} bytes",
            size_of_val(&plain)
        );
        let factor = black_box(3u64);
        let capturing = Each::new(move |value: u64| value * factor);
        assert_eq!(
            size_of_val(&capturing),
            8,
            "a closure that captures one u64 takes 8 bytes; so must the Each around it"
        );
        let (each, allocations) = count(|| black_box(Each::new(move |value: u64| value ^ factor)));
        assert_eq!(
            allocations, 0,
            "Each::new allocated {allocations} time(s); store the function in a field of \
             type F instead of a Box"
        );
        assert_eq!(each.apply(0), 3);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// All sizes are for a 64-bit target.
    #[test]
    fn p_predictions() {
        // P1. Sizes of three kinds of pointer.
        let p1: (usize, usize, usize) = (8, 16, 16);
        assert_eq!(
            p1,
            (
                size_of::<&u64>(),
                size_of::<&dyn Stage>(),
                size_of::<Box<dyn Stage>>()
            ),
            "P1: sizes of &u64, &dyn Stage, Box<dyn Stage> (left: your prediction, right: actual)"
        );

        // P2. Sizes of three callable values.
        let step = black_box(5u64);
        let captures_nothing = |value: u64| value + 1;
        let captures_step = move |value: u64| value + step;
        let pointer: fn(u64) -> u64 = captures_nothing;
        let p2: (usize, usize, usize) = (0, 8, 8);
        assert_eq!(
            p2,
            (
                size_of_val(&captures_nothing),
                size_of_val(&captures_step),
                size_of_val(&pointer)
            ),
            "P2: sizes of (a closure without captures, a closure that captures a u64, a fn pointer) (left: your prediction, right: actual)"
        );
        let _ = (captures_step(1), pointer(1));

        // P3. 1,000 values go through 3 stages. Every call below goes
        // through a trait object. How many such calls does the loop "value by
        // value" make, and how many does the loop "stage by stage" make?
        struct Counting<'a> {
            apply_calls: &'a Cell<u64>,
            apply_all_calls: &'a Cell<u64>,
        }
        impl Stage for Counting<'_> {
            fn apply(&self, value: u64) -> u64 {
                self.apply_calls.set(self.apply_calls.get() + 1);
                value
            }
            fn apply_all(&self, _values: &mut [u64]) {
                self.apply_all_calls.set(self.apply_all_calls.get() + 1);
            }
        }
        let (apply_calls, apply_all_calls) = (Cell::new(0), Cell::new(0));
        let stages: Vec<Box<dyn Stage + '_>> = (0..3)
            .map(|_| {
                Box::new(Counting {
                    apply_calls: &apply_calls,
                    apply_all_calls: &apply_all_calls,
                }) as Box<dyn Stage + '_>
            })
            .collect();
        let mut values = vec![0u64; 1_000];
        for value in &mut values {
            for stage in &stages {
                *value = stage.apply(*value);
            }
        }
        for stage in &stages {
            stage.apply_all(&mut values);
        }
        let (by_value, by_stage) = (apply_calls.get(), apply_all_calls.get());
        let p3: (u64, u64) = (3000, 3);
        assert_eq!(
            p3,
            (by_value, by_stage),
            "P3: virtual calls for (value by value, stage by stage) (left: your prediction, right: actual)"
        );

        // P4. Does `Option` add bytes to these two?
        let p4: (usize, usize) = (16, 8);
        assert_eq!(
            p4,
            (
                size_of::<Option<Box<dyn Stage>>>(),
                size_of::<Option<fn(u64) -> u64>>()
            ),
            "P4: sizes of Option<Box<dyn Stage>> and Option<fn(u64) -> u64> (left: your prediction, right: actual)"
        );
    }
}
