//! LESSON 02 — Reference bindings and identity
//! Run: cargo test --test chapter05 l02_
//! Read first: CONCEPTS.md, section "Lesson 02 · Reference bindings and identity".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can a mutable binding to &i32 be rebound without mutating either integer? Does == on references compare addresses?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain reference binding mutability, referent mutability, nested references, and std::ptr::eq.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Create equal Strings in two allocations. Compare == and ptr::eq. Then borrow a reference again and count which dereferences are explicit when accessing an integer.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

/// A. `slot` is a mutable reference to the caller's shared reference
/// (`&mut &i32`). When `use_second` is true, make the caller's reference point
/// at `second`; otherwise leave it alone. Return the integer that the caller's
/// reference points at afterwards. Neither integer may change: only the
/// reference is rebound.
/// (`'a` says that both integers live long enough for the caller's reference
/// to point at either of them. Lesson 04 explains this notation; you do not
/// need to change it.)
fn retarget<'a>(slot: &mut &'a i32, second: &'a i32, use_second: bool) -> i32 {
    if use_second {
        // `*slot` is the caller's `&i32`. Assigning to it rebinds that
        // reference; no integer is written.
        *slot = second;
    }
    // One star reaches the inner reference, the second star reaches the integer.
    **slot
}
/// B. Return (equal_contents, same_String_object), not buffer identity.
// `&String` on purpose: the task asks whether two references point at the same
// String object, and a `&str` would no longer know which String it came from.
#[allow(clippy::ptr_arg)]
fn compare(left: &String, right: &String) -> (bool, bool) {
    // `==` compares the text. `ptr::eq` compares the addresses of the two
    // String objects, so two empty Strings are still different objects.
    (left == right, std::ptr::eq(left, right))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_rebinding_changes_the_reference_not_the_integers() {
        let a = 3;
        let b = 9;
        let mut r = &a;
        assert_eq!(retarget(&mut r, &b, false), 3);
        assert!(
            std::ptr::eq(r, &a),
            "with use_second == false the caller's reference must still point at `a`"
        );
        assert_eq!(retarget(&mut r, &b, true), 9);
        assert!(
            std::ptr::eq(r, &b),
            "the caller's reference must now point at `second`; returning the \
             right number is not enough"
        );
        assert_eq!(
            retarget(&mut r, &a, false),
            9,
            "the earlier rebinding must still be in effect"
        );
        assert_eq!((a, b), (3, 9), "rebinding a reference changes no integer");
    }
    #[test]
    fn b_identity_differs_from_equality() {
        let a = "same".to_owned();
        let b = a.clone();
        assert_eq!(
            compare(&a, &b),
            (true, false),
            "a clone has equal contents but is a different String object"
        );
        assert_eq!(compare(&a, &a), (true, true));
        assert_eq!(compare(&a, &"other".to_owned()), (false, false));
        // Empty Strings own no heap buffer, so their buffer pointers can be
        // equal even though they are different String objects.
        let (e1, e2) = (String::new(), String::new());
        assert_eq!(
            compare(&e1, &e2),
            (true, false),
            "compare the addresses of the String objects, not of their buffers"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Equal values in different places: `==` versus `ptr::eq`.
        let a = 7;
        let b = 7;
        let (ra, rb) = (&a, &b);
        let p1: (bool, bool) = (true, false);
        assert_eq!(p1, (ra == rb, std::ptr::eq(ra, rb)), "P1: value or address");

        // P2. Assigning through a reference to a reference.
        let x = 1;
        let y = 2;
        let mut r = &x;
        let rr = &mut r;
        *rr = &y;
        let p2: (i32, i32) = (2, 1); // (*r, x)
        assert_eq!(p2, (*r, x), "P2: rebinding r, not changing x");

        // P3. ptr::eq on slices compares the address AND the length.
        let data = [1, 2, 3];
        let p3: (bool, bool) = (false, true);
        assert_eq!(
            p3,
            (
                std::ptr::eq(&data[..2], &data[..3]),
                std::ptr::eq(&data[..2], &data[0..2])
            ),
            "P3: fat-pointer identity"
        );
    }
}
