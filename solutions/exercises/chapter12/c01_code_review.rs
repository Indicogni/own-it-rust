//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter12 c01_
//! Then: cargo clippy --test chapter12
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every broken trait contract, and every unnecessary
//!    allocation.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep the types and trait signatures; change only the bodies.
//! 4. Run clippy and resolve its warnings for this file.

use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign};

/// A tag compared without regard to ASCII case: "Rust" == "rUST", but "É" and
/// "é" differ. Tags are used as HashSet and HashMap keys.
#[derive(Debug, Clone)]
struct Tag(String);

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        // Compares in place: no lowercase copies.
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the same normalized identity that `eq` compares, so equal tags
        // hash equally. ASCII lowercasing never changes a non-ASCII byte.
        for byte in self.0.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
        // Like `str`'s own Hash: a terminator keeps ("ab", "c") and ("a", "bc")
        // apart when tags are hashed one after another.
        state.write_u8(0xff);
    }
}

/// Polynomial coefficients, lowest power first: `Poly(vec![1, 2])` is 1 + 2x.
#[derive(Debug, Clone, PartialEq)]
struct Poly(Vec<i64>);

impl Add for Poly {
    type Output = Poly;
    /// Coefficient-wise sum; the result is as long as the longer operand.
    /// Both operands arrive by value, so the sum must not allocate, whichever
    /// operand is the longer one.
    fn add(self, rhs: Poly) -> Poly {
        // The longer operand already has room for the whole sum.
        let (mut long, short) = if self.0.len() >= rhs.0.len() {
            (self, rhs)
        } else {
            (rhs, self)
        };
        long += &short;
        long
    }
}

impl AddAssign<&Poly> for Poly {
    /// Same sum as `Add`, written into `self`.
    fn add_assign(&mut self, rhs: &Poly) {
        let overlap = self.0.len().min(rhs.0.len());
        for (mine, theirs) in self.0.iter_mut().zip(&rhs.0) {
            *mine += theirs;
        }
        // Whatever `rhs` has beyond our length is added to implicit zeros.
        self.0.extend_from_slice(&rhs.0[overlap..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::run_op;
    use practice::alloc::count;
    use std::hash::DefaultHasher;
    use std::hint::black_box;

    fn tag(s: &str) -> Tag {
        Tag(s.to_string())
    }

    fn hash_of(t: &Tag) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    // The operators run through `run_op` (see mod.rs), which fails with a message
    // if two operator methods call each other forever. Allocations are counted
    // per thread, so `count` is called inside the same closure.
    #[test]
    fn a_tags_ignore_ascii_case_only() {
        let (case, letter, non_ascii, prefix) = run_op("Tag == Tag", || {
            (
                tag("Rust") == tag("rUST"),
                tag("Rust") == tag("Rest"),
                tag("É") == tag("é"),
                tag("ab") == tag("abc"),
            )
        });
        assert!(case, "\"Rust\" and \"rUST\" differ only in ASCII case");
        assert!(!letter, "\"Rust\" and \"Rest\" are different tags");
        assert!(
            !non_ascii,
            "only ASCII letters are folded: É and é stay different"
        );
        assert!(
            !prefix,
            "a tag is not equal to a longer tag that starts with it"
        );
    }

    #[test]
    fn a_equal_tags_hash_equally() {
        assert_eq!(
            hash_of(&tag("Rust")),
            hash_of(&tag("rUST")),
            "tags that compare equal must produce the same hash"
        );
        assert_eq!(hash_of(&tag("")), hash_of(&tag("")));
    }

    #[test]
    fn a_different_tags_still_hash_differently() {
        assert_ne!(
            hash_of(&tag("Rust")),
            hash_of(&tag("Rest")),
            "the hash must still depend on the text, or every tag lands in one bucket"
        );
    }

    #[test]
    fn a_comparing_tags_does_not_allocate() {
        let (equal, allocations) = run_op("Tag == Tag", || {
            let (a, b) = (tag("Serde-JSON"), tag("serde-json"));
            count(|| black_box(&a) == black_box(&b))
        });
        assert!(equal);
        assert_eq!(allocations, 0, "compare in place; std has a method for it");
    }

    #[test]
    fn a_hashing_a_tag_does_not_allocate() {
        let t = tag("Serde-JSON");
        let mut hasher = DefaultHasher::new();
        let ((), allocations) = count(|| black_box(&t).hash(&mut hasher));
        black_box(hasher.finish());
        assert_eq!(
            allocations, 0,
            "hash runs on every map lookup: feed the bytes to the hasher without \
             building a lowercase String first"
        );
    }

    #[test]
    fn b_add_keeps_the_longer_tail() {
        let (right_longer, left_longer, left_empty) = run_op("Poly + Poly", || {
            (
                Poly(vec![1, 2]) + Poly(vec![10, 20, 30]),
                Poly(vec![10, 20, 30]) + Poly(vec![1]),
                Poly(vec![]) + Poly(vec![5]),
            )
        });
        assert_eq!(right_longer, Poly(vec![11, 22, 30]));
        assert_eq!(left_longer, Poly(vec![11, 20, 30]));
        assert_eq!(left_empty, Poly(vec![5]));
    }

    #[test]
    fn b_add_reuses_an_operand_buffer() {
        let (sum, allocations) = run_op("Poly + Poly", || {
            let (a, b) = (Poly(vec![1, 2, 3]), Poly(vec![1, 1]));
            count(|| black_box(black_box(a) + black_box(b)))
        });
        assert_eq!(sum, Poly(vec![2, 3, 3]));
        assert_eq!(
            allocations, 0,
            "a + b owns both buffers already: the longer left one has room for the sum"
        );
    }

    #[test]
    fn b_add_reuses_the_right_buffer_when_it_is_longer() {
        let (sum, allocations) = run_op("Poly + Poly", || {
            let (a, b) = (Poly(vec![1, 1]), Poly(vec![1, 2, 3]));
            count(|| black_box(black_box(a) + black_box(b)))
        });
        assert_eq!(sum, Poly(vec![2, 3, 3]));
        assert_eq!(
            allocations, 0,
            "the right operand is longer here: build the sum in its buffer \
             instead of growing the left one"
        );
    }

    #[test]
    fn b_add_assign_works_in_place() {
        let (a, same_buffer, allocations) = run_op("Poly += &Poly", || {
            let mut a = Poly(vec![1, 2, 3]);
            let b = Poly(vec![-1, -2]);
            let before = a.0.as_ptr();
            let ((), allocations) = count(|| a += black_box(&b));
            let same_buffer = a.0.as_ptr() == before;
            (a, same_buffer, allocations)
        });
        assert_eq!(a, Poly(vec![0, 0, 3]));
        assert!(
            same_buffer,
            "`+=` must write into the left operand's own buffer, not replace it"
        );
        assert_eq!(allocations, 0, "+= should not copy either side");
    }

    #[test]
    fn b_add_assign_grows_only_when_it_must() {
        let (short, allocations) = run_op("Poly += &Poly", || {
            let mut short = Poly(vec![1]);
            let long = Poly(vec![1, 1, 1]);
            let ((), allocations) = count(|| short += black_box(&long));
            (short, allocations)
        });
        assert_eq!(short, Poly(vec![2, 1, 1]));
        assert!(
            allocations <= 1,
            "a longer right side needs one growth of the left buffer and nothing else; \
             counted {allocations} allocations"
        );
    }
}
