//! Milestone 5: `Arena` (SPEC section 6).
//!
//! ```text
//! cargo test --test m5_arena
//! cargo +nightly-2026-11-20 miri test --test m5_arena
//! ```
//!
//! Run this file under Miri early and often. An arena that hands out `&mut`
//! references from `&self` can pass every test here and still break the
//! aliasing rules; Miri is the tool that sees it.

mod common;

use common::{Ledger, Rng, bounded, expect_panic, lies_inside, steps};
use nostd_collections::Arena;

/// SPEC 6.1: `new` is a `const fn`. This function compiles only if it is.
const fn made_in_a_const_fn() -> Arena<64> {
    Arena::new()
}

/// The address of a value, as a number.
///
/// `black_box` hides from the optimiser where the number came from. Without
/// it, an optimised build may reason "this is the address of a reference,
/// and references are always aligned", and remove the very check that a
/// test makes on the alignment.
fn address<T: ?Sized>(reference: &T) -> usize {
    std::hint::black_box(std::ptr::from_ref(reference).cast::<u8>() as usize)
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(align(16))]
struct Aligned16(u8);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(align(64))]
struct Aligned64(u8);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(align(1024))]
struct Aligned1024(u8);

/// Puts an arena one byte behind a 1024-byte boundary (rounded up to the
/// alignment of the arena itself). Offset 0 of its storage is then not at a
/// multiple of 1024, wherever the test happens to run.
#[repr(C, align(1024))]
struct NotAtABoundary {
    skew: u8,
    arena: Arena<4096>,
}

// ---------------------------------------------------------------------------
// SPEC 6.1 and 6.3: construction and `alloc`
// ---------------------------------------------------------------------------

#[test]
fn a_new_arena_is_empty() {
    bounded(|| {
        let arena = made_in_a_const_fn();
        assert_eq!(arena.capacity(), 64, "capacity is the const parameter N");
        assert_eq!(arena.used(), 0, "a new arena has used nothing");
        assert_eq!(arena.remaining(), 64, "remaining is capacity minus used");

        let d: Arena<8> = Arena::default();
        assert_eq!(d.used(), 0, "Default::default() is an empty arena");
        assert!(!format!("{d:?}").is_empty(), "Debug must print something");
    });
}

#[test]
fn alloc_stores_the_value_and_returns_a_reference_to_it() {
    bounded(|| {
        let arena: Arena<64> = Arena::new();
        let number = arena.alloc(41u32).expect("4 bytes fit into 64");
        assert_eq!(*number, 41);
        *number += 1;
        assert_eq!(*number, 42, "the reference is mutable");
        assert!(
            lies_inside(number, &arena),
            "the value must be stored inside the Arena value itself (inline storage)"
        );
    });
}

#[test]
fn earlier_references_stay_valid_while_the_arena_is_used() {
    bounded(|| {
        let arena: Arena<128> = Arena::new();
        // All four references are alive at the same time, and each is used
        // after later allocations were made.
        let a = arena.alloc(1u64).unwrap();
        let b = arena.alloc([2u8; 5]).unwrap();
        let c = arena.alloc((3u16, 4u32)).unwrap();
        *a += 10;
        let d = arena.alloc(5.5f64).unwrap();
        b[4] = 9;
        c.1 += 1;
        *d *= 2.0;
        *a += 100;
        assert_eq!(*a, 111, "an allocation must not disturb earlier values");
        assert_eq!(*b, [2, 2, 2, 2, 9]);
        assert_eq!(*c, (3, 5));
        assert_eq!(*d, 11.0);
    });
}

#[test]
fn the_arena_can_hold_a_linked_structure() {
    bounded(|| {
        struct Node<'a> {
            value: u32,
            next: Option<&'a Node<'a>>,
        }
        let arena: Arena<256> = Arena::new();
        let mut head: Option<&Node<'_>> = None;
        for value in 1..=5 {
            // Each node refers to the node made before it. This compiles
            // only if the returned reference lives as long as the borrow of
            // the arena, and not shorter.
            let node = arena
                .alloc(Node { value, next: head })
                .expect("5 nodes fit into 256 bytes");
            head = Some(node);
        }
        let mut seen = Vec::new();
        while let Some(node) = head {
            seen.push(node.value);
            head = node.next;
        }
        assert_eq!(seen, [5, 4, 3, 2, 1]);
    });
}

// ---------------------------------------------------------------------------
// SPEC 6.2: placement
// ---------------------------------------------------------------------------

#[test]
fn placement_follows_the_rule_of_the_specification() {
    bounded(|| {
        // The example of SPEC 6.2, step by step.
        let arena: Arena<32> = Arena::new();
        let base = address(&*arena.alloc(1u8).unwrap());
        assert_eq!(arena.used(), 1, "a u8 at offset 0 takes one byte");

        let second = address(&*arena.alloc(2u32).unwrap());
        assert_eq!(
            second - base,
            4,
            "a u32 needs alignment 4: three bytes of padding"
        );
        assert_eq!(arena.used(), 8, "used counts the padding too");

        let third = address(&*arena.alloc(3u16).unwrap());
        assert_eq!(third - base, 8, "offset 8 is already aligned for a u16");
        assert_eq!(arena.used(), 10);

        // 16 on every platform where a u64 is aligned to 8 bytes. (On 32-bit
        // x86 its alignment is 4, and the rule gives 12.)
        let wide = 10usize.next_multiple_of(align_of::<u64>());
        let fourth = address(&*arena.alloc(4u64).unwrap());
        assert_eq!(
            fourth - base,
            wide,
            "a u64 starts at the next multiple of its alignment"
        );
        assert_eq!(arena.used(), wide + 8);
        assert_eq!(arena.remaining(), 32 - (wide + 8));

        let rest = arena.remaining();
        let fifth = arena
            .alloc_slice_copy(&[5u8; 32][..rest])
            .expect("exactly the rest fits");
        assert_eq!(address(fifth) - base, wide + 8);
        assert_eq!(arena.used(), 32, "the arena is exactly full");
        assert_eq!(arena.remaining(), 0);
        assert!(
            arena.alloc(6u8).is_err(),
            "a full arena refuses one more byte"
        );
        assert!(arena.alloc(()).is_ok(), "a zero-sized value always fits");
    });
}

#[test]
fn every_reference_is_aligned_for_its_type() {
    bounded(|| {
        let arena: Arena<256> = Arena::new();
        macro_rules! check {
            ($value:expr, $ty:ty) => {{
                // One byte first, so that the next value needs padding.
                arena.alloc(0u8).expect("there is room");
                let reference: &mut $ty = arena.alloc($value).expect("there is room");
                assert_eq!(
                    address(reference) % align_of::<$ty>(),
                    0,
                    "a reference to {} must be aligned to {} bytes",
                    stringify!($ty),
                    align_of::<$ty>()
                );
                assert_eq!(*reference, $value);
            }};
        }
        check!(7u8, u8);
        check!(7u16, u16);
        check!(7u32, u32);
        check!(7u64, u64);
        check!(7u128, u128);
        check!(7.5f64, f64);
        check!(Aligned16(7), Aligned16);
        check!((7u8, 7u64), (u8, u64));
    });
}

/// Wrappers that put an arena at two different positions relative to a
/// 16-byte boundary.
#[repr(C, align(16))]
struct AtBoundary {
    arena: Arena<64>,
}

#[repr(C, align(16))]
struct BehindOneWord {
    word: u64,
    arena: Arena<64>,
}

#[test]
fn placement_does_not_depend_on_where_the_arena_is() {
    bounded(|| {
        fn check(arena: &Arena<64>, place: &str) {
            let first = arena.alloc(Aligned16(1)).expect("16 bytes fit into 64");
            assert_eq!(
                address(first) % 16,
                0,
                "{place}: the first value needs alignment 16 and must get it"
            );
            assert_eq!(
                arena.used(),
                16,
                "{place}: SPEC 6.2: the storage starts at a 16-byte boundary, so the first \
                 value is at offset 0 and needs no padding"
            );
            let second = arena.alloc(2u8).unwrap();
            let third = arena.alloc(Aligned16(3)).unwrap();
            assert_eq!(
                address(third) - address(first),
                32,
                "{place}: 15 bytes of padding"
            );
            assert_eq!(arena.used(), 48, "{place}");
            assert_eq!((first.0, *second, third.0), (1, 2, 3));
        }
        let outer = AtBoundary {
            arena: Arena::new(),
        };
        check(&outer.arena, "arena at a 16-byte boundary");
        let outer = BehindOneWord {
            word: 0,
            arena: Arena::new(),
        };
        check(
            &outer.arena,
            "arena 8 bytes behind a 16-byte boundary, if its alignment allows that",
        );
        assert_eq!(outer.word, 0);
    });
}

#[test]
fn alignments_above_16_work() {
    bounded(|| {
        let arena: Arena<256> = Arena::new();
        arena.alloc(1u8).unwrap();
        let big = arena.alloc(Aligned64(9)).expect(
            "one byte, at most 63 bytes of padding and 64 bytes of value fit into 256 bytes",
        );
        assert_eq!(
            address(big) % 64,
            0,
            "the reference must be aligned to 64 bytes"
        );
        assert_eq!(big.0, 9);
        assert!(lies_inside(big, &arena));
        assert!(
            arena.used() >= 65,
            "one byte and 64 bytes are taken, plus padding"
        );
        assert!(
            arena.used() <= 128,
            "at most 63 bytes of padding are needed"
        );

        // The same for an arena whose storage certainly does not start at a
        // multiple of the alignment. Rounding the *offset* up to 1024 gives
        // a misaligned address here.
        let outer = NotAtABoundary {
            skew: 0,
            arena: Arena::new(),
        };
        outer.arena.alloc(1u8).unwrap();
        let big = outer.arena.alloc(Aligned1024(7)).expect(
            "one byte, at most 1023 bytes of padding and 1024 bytes of value fit into 4096 bytes",
        );
        assert_eq!(
            address(big) % 1024,
            0,
            "SPEC 6.2: for an alignment above 16, round the address up to the alignment, not the \
             offset; this arena does not start at a multiple of 1024"
        );
        assert_eq!(big.0, 7);
        assert!(lies_inside(big, &outer.arena));
        assert!(
            outer.arena.used() >= 1025,
            "one byte and 1024 bytes are taken, plus padding"
        );
        assert!(
            outer.arena.used() <= 1 + 1023 + 1024,
            "at most 1023 bytes of padding are needed"
        );
        assert_eq!(outer.skew, 0);
    });
}

// ---------------------------------------------------------------------------
// SPEC 6.3: a full arena
// ---------------------------------------------------------------------------

#[test]
fn a_value_that_does_not_fit_comes_back() {
    bounded(|| {
        let arena: Arena<16> = Arena::new();
        let kept = arena.alloc([1u8; 12]).unwrap();
        let error = arena
            .alloc(String::from("too big"))
            .expect_err("a String is larger than the 4 bytes that are left");
        assert_eq!(error.to_string(), "insufficient capacity");
        assert_eq!(
            error.into_inner(),
            "too big",
            "the error carries the rejected value"
        );
        assert_eq!(arena.used(), 12, "a failed allocation takes nothing");

        let small = arena
            .alloc(7u32)
            .expect("a smaller value still fits after a failure");
        assert_eq!(arena.used(), 16);
        assert_eq!(
            arena.alloc(1u8).map_err(|e| e.into_inner()),
            Err(1),
            "the arena is full"
        );
        assert_eq!((*kept, *small), ([1u8; 12], 7));
    });
}

#[test]
fn padding_can_make_a_value_not_fit() {
    bounded(|| {
        let arena: Arena<8> = Arena::new();
        arena.alloc(1u8).unwrap();
        assert!(
            arena.alloc(2u64).is_err(),
            "after one byte, a u64 would start at offset 8, which is the end of the storage"
        );
        assert_eq!(
            arena.used(),
            1,
            "a failed allocation does not take the padding either"
        );
        assert!(
            arena.alloc(3u32).is_ok(),
            "a u32 at offset 4 ends exactly at 8"
        );
        assert_eq!(arena.used(), 8);
    });
}

#[test]
fn zero_sized_values_always_fit() {
    bounded(|| {
        #[derive(Debug, PartialEq)]
        struct Marker;

        let arena: Arena<0> = Arena::new();
        assert_eq!(arena.capacity(), 0);
        assert!(arena.alloc(1u8).is_err(), "N = 0: nothing with a size fits");
        for _ in 0..100 {
            assert!(arena.alloc(()).is_ok(), "a zero-sized value needs no room");
        }
        assert_eq!(*arena.alloc(Marker).unwrap(), Marker);
        let empty_array = arena.alloc([0u64; 0]).unwrap();
        assert_eq!(
            address(empty_array) % align_of::<u64>(),
            0,
            "a reference to a zero-sized value must still be aligned"
        );
        assert_eq!(arena.alloc_slice_copy::<u32>(&[]).map(|s| s.len()), Some(0));
        assert_eq!(arena.alloc_str("").map(|s| s.len()), Some(0));
        assert_eq!(
            arena.alloc_slice_fill_with(3, |_| ()).map(|s| s.len()),
            Some(3),
            "a slice of zero-sized elements needs no room"
        );
        assert_eq!(arena.used(), 0, "zero-sized values never change `used`");

        let arena: Arena<8> = Arena::new();
        arena.alloc(1u8).unwrap();
        arena.alloc([0u64; 0]).unwrap();
        assert_eq!(arena.used(), 1, "a zero-sized value adds no padding either");
    });
}

// ---------------------------------------------------------------------------
// SPEC 6.4: slices and strings
// ---------------------------------------------------------------------------

#[test]
fn alloc_slice_copy_copies_a_slice() {
    bounded(|| {
        let arena: Arena<64> = Arena::new();
        arena.alloc(0u8).unwrap();
        let source = [10u32, 20, 30];
        let copy = arena.alloc_slice_copy(&source).expect("12 bytes fit");
        assert_eq!(copy, [10, 20, 30]);
        assert_eq!(
            address(copy) % align_of::<u32>(),
            0,
            "the slice is aligned for its elements"
        );
        assert_eq!(
            arena.used(),
            16,
            "one byte, three bytes of padding, twelve bytes of slice"
        );
        copy[1] = 99;
        assert_eq!(
            source,
            [10, 20, 30],
            "the copy is independent of its source"
        );
        assert_eq!(copy, [10, 99, 30]);
        assert!(lies_inside(copy, &arena));

        let before = arena.used();
        assert!(
            arena.alloc_slice_copy(&[0u8; 49]).is_none(),
            "49 bytes do not fit into 48"
        );
        assert_eq!(arena.used(), before, "a failed allocation takes nothing");
        assert!(
            arena.alloc_slice_copy(&[0u8; 48]).is_some(),
            "48 bytes fit exactly"
        );
        assert_eq!(arena.remaining(), 0);
    });
}

#[test]
fn alloc_str_copies_a_string() {
    bounded(|| {
        let arena: Arena<32> = Arena::new();
        let source = String::from("grüße");
        let copy = arena.alloc_str(&source).expect("7 bytes fit");
        assert_eq!(copy, "grüße");
        assert_eq!(
            arena.used(),
            source.len(),
            "a str takes its length in bytes, with alignment 1"
        );
        copy.make_ascii_uppercase();
        assert_eq!(copy, "GRüßE", "the copy is mutable");
        assert_eq!(source, "grüße", "the copy is independent of its source");
        drop(source);
        assert_eq!(
            copy, "GRüßE",
            "the copy lives in the arena, not in the String"
        );
        assert!(
            arena
                .alloc_str("this text has more than 25 bytes")
                .is_none()
        );
        assert_eq!(arena.used(), 7);
    });
}

#[test]
fn alloc_slice_fill_with_builds_a_slice() {
    bounded(|| {
        let arena: Arena<128> = Arena::new();
        let mut calls = Vec::new();
        let squares = arena
            .alloc_slice_fill_with(5, |index| {
                calls.push(index);
                (index * index) as u64
            })
            .expect("40 bytes fit");
        assert_eq!(
            calls,
            [0, 1, 2, 3, 4],
            "the closure is called once per index, in order"
        );
        assert_eq!(squares, [0, 1, 4, 9, 16]);
        assert_eq!(arena.used(), 40);
        squares[0] = 7;
        assert_eq!(squares[0], 7);

        let words = arena
            .alloc_slice_fill_with(2, |index| ["zero", "one"][index])
            .expect("two string slices fit");
        assert_eq!(
            words,
            ["zero", "one"],
            "the element type does not need to be Copy or Default"
        );
        let none = arena.alloc_slice_fill_with(0, |_| -> u64 { panic!("no element, no call") });
        assert_eq!(none.map(|s| s.len()), Some(0));
    });
}

#[test]
fn alloc_slice_fill_with_refuses_before_it_calls_the_closure() {
    bounded(|| {
        let arena: Arena<64> = Arena::new();
        arena.alloc(1u8).unwrap();
        let too_long = arena.alloc_slice_fill_with(8, |_| -> u64 {
            panic!("the closure must not run when the slice does not fit")
        });
        assert!(
            too_long.is_none(),
            "8 * 8 bytes do not fit behind one byte in 64 bytes"
        );
        assert_eq!(arena.used(), 1, "a failed allocation takes nothing");

        // 8 * len overflows usize. With wrapping arithmetic the size would
        // look like 16 bytes.
        let len = usize::MAX / 8 + 3;
        let overflow = arena.alloc_slice_fill_with(len, |_| -> u64 {
            panic!("the closure must not run when the size overflows")
        });
        assert!(
            overflow.is_none(),
            "a length whose size in bytes overflows usize must be refused"
        );
        assert_eq!(arena.used(), 1);

        // Here 8 * len still fits into usize, but "start + size" does not:
        // the slice would start at offset 8, and 8 + size is one more than
        // usize::MAX. With wrapping arithmetic the end would look like 0.
        let len = usize::MAX / 8;
        let overflow = arena.alloc_slice_fill_with(len, |_| -> u64 {
            panic!("the closure must not run when the end of the slice overflows")
        });
        assert!(
            overflow.is_none(),
            "SPEC 6.2: a slice whose end offset overflows usize must be refused"
        );
        assert_eq!(arena.used(), 1);
    });
}

#[test]
fn the_fill_closure_may_allocate_from_the_same_arena() {
    bounded(|| {
        let arena: Arena<256> = Arena::new();
        // The elements of the outer slice are references to values that the
        // closure allocates while the outer slice is being filled.
        let outer = arena
            .alloc_slice_fill_with(4, |index| {
                arena
                    .alloc(100 + index as u64)
                    .expect("there is room for the inner values")
            })
            .expect("there is room for the outer slice");
        let values: Vec<u64> = outer.iter().map(|inner| **inner).collect();
        assert_eq!(
            values,
            [100, 101, 102, 103],
            "SPEC 6.4: the room for the slice is reserved before the closure runs, so what \
             the closure allocates cannot overlap the slice"
        );
        let slice_start = address(&*outer);
        let slice_end = slice_start + size_of_val(&*outer);
        for inner in outer.iter() {
            let at = address(&**inner);
            assert!(
                at >= slice_end || at + 8 <= slice_start,
                "an inner value lies inside the outer slice"
            );
        }
        *outer[2] = 7;
        assert_eq!(*outer[2], 7);
        assert_eq!(*outer[3], 103);
    });
}

#[test]
fn the_arena_survives_a_panicking_fill_closure() {
    bounded(|| {
        let arena: Arena<128> = Arena::new();
        let before = arena.alloc(11u32).unwrap();
        expect_panic(
            "alloc_slice_fill_with with a closure that panics",
            "fill gave up",
            || {
                arena.alloc_slice_fill_with(4, |index| {
                    if index == 2 {
                        panic!("fill gave up");
                    }
                    index as u64
                })
            },
        );
        assert!(arena.used() <= 40, "at most the reserved room is taken");
        let after = arena
            .alloc(22u32)
            .expect("the arena is usable after the panic");
        assert_eq!((*before, *after), (11, 22));
    });
}

// ---------------------------------------------------------------------------
// SPEC 6.5: destructors and reset
// ---------------------------------------------------------------------------

#[test]
fn the_arena_never_runs_destructors() {
    bounded(|| {
        let ledger = Ledger::new();
        {
            let mut arena: Arena<256> = Arena::new();
            let item = arena.alloc(ledger.item(1)).expect("there is room");
            assert_eq!(item.tag, 1);
            let items = arena
                .alloc_slice_fill_with(3, |index| ledger.item(10 + index as u32))
                .expect("there is room");
            assert_eq!(items.len(), 3);
            arena.reset();
            assert!(
                ledger.dropped().is_empty(),
                "reset must not run destructors"
            );
        }
        assert!(
            ledger.dropped().is_empty(),
            "SPEC 6.5: dropping the arena must not run the destructors of the stored values"
        );
        assert_eq!(ledger.live(), 4);
    });
}

#[test]
fn reset_makes_the_whole_storage_free_again() {
    bounded(|| {
        let mut arena: Arena<32> = Arena::new();
        let first = address(&*arena.alloc([1u8; 32]).unwrap());
        assert!(arena.alloc(1u8).is_err(), "the arena is full");
        arena.reset();
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.remaining(), 32);
        let again = arena
            .alloc([2u64; 4])
            .expect("after reset the whole storage is free");
        assert_eq!(
            address(again),
            first,
            "after reset the first value is at offset 0 again"
        );
        assert_eq!(*again, [2; 4]);
        arena.reset();
        arena.reset();
        assert_eq!(arena.alloc_str("ok").map(|s| &*s), Some("ok"));
    });
}

// ---------------------------------------------------------------------------
// Everything together, against the placement rule as the model
// ---------------------------------------------------------------------------

/// One allocation of the model test: where it is and what it must contain.
enum Piece<'a> {
    Byte(&'a mut u8, u8),
    Word(&'a mut u32, u32),
    Wide(&'a mut u64, u64),
    Bytes(&'a mut [u8], u8),
    Pairs(&'a mut [u16], u16),
}

#[test]
fn random_allocations_match_the_placement_rule_and_never_overlap() {
    bounded(|| {
        const N: usize = 4096;
        let mut rng = Rng::new(0x5EED_0005);
        for round in 0..steps(40, 4) {
            let arena: Arena<N> = Arena::new();
            let mut model_used = 0usize;
            let mut pieces: Vec<Piece<'_>> = Vec::new();
            let mut seed = 0u8;
            loop {
                seed = seed.wrapping_add(1);
                let kind = rng.below(5);
                let count = 1 + rng.below(40);
                let (size, align) = match kind {
                    0 => (1, 1),
                    1 => (4, align_of::<u32>()),
                    2 => (8, align_of::<u64>()),
                    3 => (count, 1),
                    _ => (2 * count, align_of::<u16>()),
                };
                let start = model_used.next_multiple_of(align);
                let fits = start + size <= N;
                let piece = match kind {
                    0 => arena.alloc(seed).ok().map(|r| Piece::Byte(r, seed)),
                    1 => arena
                        .alloc(u32::from(seed) * 0x0101_0101)
                        .ok()
                        .map(|r| Piece::Word(r, u32::from(seed) * 0x0101_0101)),
                    2 => arena
                        .alloc(u64::from(seed) << 40)
                        .ok()
                        .map(|r| Piece::Wide(r, u64::from(seed) << 40)),
                    3 => arena
                        .alloc_slice_copy(&vec![seed; count])
                        .map(|r| Piece::Bytes(r, seed)),
                    _ => arena
                        .alloc_slice_fill_with(count, |_| u16::from(seed) * 257)
                        .map(|r| Piece::Pairs(r, u16::from(seed) * 257)),
                };
                assert_eq!(
                    piece.is_some(),
                    fits,
                    "round {round}: a value of size {size} and alignment {align} with {model_used} \
                     bytes used must {} (SPEC 6.2)",
                    if fits { "fit" } else { "be refused" }
                );
                let Some(piece) = piece else { break };
                model_used = start + size;
                assert_eq!(
                    arena.used(),
                    model_used,
                    "round {round}: `used` after the allocation"
                );
                pieces.push(piece);
            }
            assert_eq!(
                arena.used(),
                model_used,
                "round {round}: a refused allocation takes nothing"
            );

            // Every value must still hold what was written to it: no later
            // allocation may have used its bytes.
            let mut ranges = Vec::new();
            for piece in &pieces {
                let (start, len) = match piece {
                    Piece::Byte(r, want) => {
                        assert_eq!(**r, *want, "round {round}: a u8 was overwritten");
                        (address(&**r), 1)
                    }
                    Piece::Word(r, want) => {
                        assert_eq!(**r, *want, "round {round}: a u32 was overwritten");
                        (address(&**r), 4)
                    }
                    Piece::Wide(r, want) => {
                        assert_eq!(**r, *want, "round {round}: a u64 was overwritten");
                        (address(&**r), 8)
                    }
                    Piece::Bytes(r, want) => {
                        assert!(
                            r.iter().all(|b| b == want),
                            "round {round}: a byte slice was overwritten"
                        );
                        (address(&**r), r.len())
                    }
                    Piece::Pairs(r, want) => {
                        assert!(
                            r.iter().all(|p| p == want),
                            "round {round}: a u16 slice was overwritten"
                        );
                        (address(&**r), 2 * r.len())
                    }
                };
                ranges.push((start, start + len));
            }
            ranges.sort_unstable();
            for pair in ranges.windows(2) {
                assert!(
                    pair[0].1 <= pair[1].0,
                    "round {round}: two allocations overlap"
                );
            }
            // Writing through every reference at the end proves to Miri that
            // all of them are still valid.
            for piece in &mut pieces {
                match piece {
                    Piece::Byte(r, _) => **r = 0,
                    Piece::Word(r, _) => **r = 0,
                    Piece::Wide(r, _) => **r = 0,
                    Piece::Bytes(r, _) => r.fill(0),
                    Piece::Pairs(r, _) => r.fill(0),
                }
            }
        }
    });
}
