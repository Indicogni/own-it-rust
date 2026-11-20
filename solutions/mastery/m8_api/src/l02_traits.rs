//! LESSON 02 — Traits, bounds and auto traits: write the program that breaks
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` and `k_classes` at the
//! bottom (replace each `guess()` with your answer). Then consider: a private
//! field is not part of a type's public interface. Can a change to a private
//! field still break a user? And what does `impl Iterator` in a return type
//! promise, and what does it leak?
//!
//! IMPLEMENT:
//! - Task W: the six witness programs in `fixtures/l02/witness/`. In lesson 01
//!   you were given a program and asked whether a change breaks it. Now you are
//!   given the change and write the program. Each file describes its change;
//!   the table `CHANGES` below has the exact edits to `fixtures/l02/courier.rs`.
//! - Task C in this file.
//!
//! Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Changes T6 and T8 edit no public signature. Explain, for
//! each, how the compiler decides that the old program is now wrong, and name
//! one thing a maintainer can put into the library's own test suite so that
//! such a change cannot be released by accident.
//!
//! Reference: <https://doc.rust-lang.org/cargo/reference/semver.html>

use crate::l01_breaking::Change;

/// The eight changes of this lesson. Each one is applied alone to version 1.0.0
/// of `fixtures/l02/courier.rs`. You do not edit this table.
pub const CHANGES: [Change; 8] = [
    Change {
        title: "T1: `Transport` gains a required method",
        edits: &[
            (
                "    fn send(&mut self, bytes: &[u8]) -> usize;\n}",
                "    fn send(&mut self, bytes: &[u8]) -> usize;\n\n    \
                     /// Pushes out everything that is still buffered.\n    \
                     fn flush(&mut self);\n}",
            ),
            // The library's own implementation is adjusted.
            (
                "        bytes.len()\n    }\n}",
                "        bytes.len()\n    }\n\n    fn flush(&mut self) {}\n}",
            ),
        ],
    },
    Change {
        title: "T2: `Transport` gains a provided method",
        edits: &[(
            "    fn send(&mut self, bytes: &[u8]) -> usize;\n}",
            "    fn send(&mut self, bytes: &[u8]) -> usize;\n\n    \
                 /// Pushes out everything that is still buffered.\n    \
                 fn flush(&mut self) {}\n}",
        )],
    },
    Change {
        title: "T3: the sealed trait `Codec` gains a required method",
        edits: &[
            (
                "    fn encode(&self, text: &str) -> Vec<u8>;\n}",
                "    fn encode(&self, text: &str) -> Vec<u8>;\n\n    \
                     /// A short name for log lines.\n    \
                     fn name(&self) -> &'static str;\n}",
            ),
            (
                "        text.as_bytes().to_vec()\n    }\n}",
                "        text.as_bytes().to_vec()\n    }\n\n    \
                     fn name(&self) -> &'static str {\n        \"plain\"\n    }\n}",
            ),
            (
                "        format!(\"{text}\\n\").into_bytes()\n    }\n}",
                "        format!(\"{text}\\n\").into_bytes()\n    }\n\n    \
                     fn name(&self) -> &'static str {\n        \"line\"\n    }\n}",
            ),
        ],
    },
    Change {
        title: "T4: `deliver` requires `T: Transport + Send`",
        edits: &[(
            "pub fn deliver<T: Transport, M: Display>",
            "pub fn deliver<T: Transport + Send, M: Display>",
        )],
    },
    Change {
        title: "T5: `deliver` accepts `T: Transport + ?Sized`",
        edits: &[(
            "pub fn deliver<T: Transport, M: Display>",
            "pub fn deliver<T: Transport + ?Sized, M: Display>",
        )],
    },
    Change {
        title: "T6: the private field of `Stats` becomes `Rc<Cell<usize>>`",
        edits: &[
            (
                "    messages: Arc<AtomicUsize>,\n}",
                "    messages: std::rc::Rc<std::cell::Cell<usize>>,\n}",
            ),
            (
                "        self.messages.fetch_add(1, Ordering::Relaxed);",
                "        self.messages.set(self.messages.get() + 1);",
            ),
            (
                "        self.messages.load(Ordering::Relaxed)",
                "        self.messages.get()",
            ),
        ],
    },
    Change {
        title: "T7: `Transport` gains a provided, generic method",
        edits: &[(
            "    fn send(&mut self, bytes: &[u8]) -> usize;\n}",
            "    fn send(&mut self, bytes: &[u8]) -> usize;\n\n    \
                 /// Sends every chunk and returns the total that was accepted.\n    \
                 fn send_all<I: IntoIterator<Item = Vec<u8>>>(&mut self, chunks: I) -> usize {\n        \
                     chunks.into_iter().map(|chunk| self.send(&chunk)).sum()\n    \
                 }\n}",
        )],
    },
    Change {
        title: "T8: `words` builds its iterator in another way; the signature is the same",
        edits: &[(
            "pub fn words(text: &str) -> impl Iterator<Item = &str> {\n    text.split_whitespace()\n}",
            "pub fn words(text: &str) -> impl Iterator<Item = &str> {\n    \
                 let words: Box<dyn Iterator<Item = &str>> =\n        \
                     Box::new(text.split(' ').filter(|word| !word.is_empty()));\n    \
                 words\n}",
        )],
    },
];

/// C. A counter of events. `hit` adds one, `hits` reads the count, and clones
/// share one count: a hit through any clone is seen by all of them.
///
/// The published version 1.0.0 of `Tally` was `Send` and `Sync`, and users move
/// clones to worker threads. Nothing in the signatures below says so. Choose the
/// fields so that it stays true, and so that `hit` can keep taking `&self`.
#[derive(Debug, Clone, Default)]
pub struct Tally {
    // An atomic inside an `Arc`: shared ownership and shared mutation that are
    // both safe across threads. `Rc<Cell<u64>>` would behave the same in one
    // thread and silently drop `Send` and `Sync`.
    hits: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Tally {
    pub fn new() -> Tally {
        Tally::default()
    }

    pub fn hit(&self) {
        // The count is only ever read as a number; it does not guard other
        // data, so no ordering stronger than `Relaxed` is needed.
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implements;
    use crate::l01_breaking::{Class, apply};
    use crate::support::{self, Build};

    /// Runs the witness `file` against courier 1.0.0 and against the library
    /// with change `number` (1 to 8).
    #[track_caller]
    fn witness(file: &str, number: usize) {
        let change = &CHANGES[number - 1];
        let relative = format!("fixtures/l02/witness/{file}");
        let source = support::read(&relative);
        let path = support::package_dir().join(&relative);
        let expect = support::field(&source, "EXPECT:").unwrap_or_default();

        let version_1 = support::library_from_file("courier", "fixtures/l02/courier.rs");
        let build = Build {
            externs: &[("courier", &version_1)],
            ..Build::default()
        };
        let before = support::program(&format!("l02-{file}-v1"), &path, &build);
        assert!(
            before.outcome == "ok",
            "{relative}\n  A witness must be a correct program for courier 1.0.0: it must compile, \
             run, and exit normally. With 1.0.0 it gives `{}`:\n{}",
            before.outcome,
            support::head(&before.output, 30)
        );

        let changed = apply(&support::read("fixtures/l02/courier.rs"), change);
        let changed = support::library_from_text("courier", &changed, &Build::default())
            .unwrap_or_else(|errors| {
                panic!(
                    "{}: the changed library does not compile:\n{errors}",
                    change.title
                )
            });
        let build = Build {
            externs: &[("courier", &changed)],
            ..Build::default()
        };
        let after = support::program(&format!("l02-{file}-changed"), &path, &build);
        assert!(
            after.outcome != "ok" && after.outcome != "panic",
            "{relative}\n  {}\n  Your program still compiles with the changed library, so it is \
             not a witness. It must use the library in the one way that the change takes away.",
            change.title
        );
        assert!(
            expect != "?" && !expect.is_empty(),
            "{relative}\n  PREDICT: your program is a witness. Now replace the `?` in its \
             `EXPECT:` line by the error code that you expect, then run again."
        );
        assert!(
            support::same_outcome(&expect, &after.outcome),
            "{relative}\n  Your program is a witness, but you predicted `{expect}` and the \
             compiler gives `{}`:\n{}",
            after.outcome,
            support::head(&after.output, 24)
        );
    }

    #[test]
    fn w1_required_method() {
        witness("w1_required_method.rs", 1);
    }

    #[test]
    fn w2_provided_method() {
        witness("w2_provided_method.rs", 2);
    }

    #[test]
    fn w3_tightened_bound() {
        witness("w3_tightened_bound.rs", 4);
    }

    #[test]
    fn w4_private_field() {
        witness("w4_private_field.rs", 6);
    }

    #[test]
    fn w5_generic_method() {
        witness("w5_generic_method.rs", 7);
    }

    #[test]
    fn w6_hidden_type() {
        witness("w6_hidden_type.rs", 8);
    }

    /// Every change must leave the library itself compiling; otherwise the
    /// question "does it break a user?" would not come up.
    #[test]
    fn w0_every_changed_library_compiles() {
        let version_1 = support::read("fixtures/l02/courier.rs");
        for change in &CHANGES {
            let changed = apply(&version_1, change);
            if let Err(errors) = support::library_from_text("courier", &changed, &Build::default())
            {
                panic!(
                    "{}: the changed library does not compile:\n{errors}",
                    change.title
                );
            }
        }
    }

    #[test]
    fn c_clones_share_one_count() {
        let tally = Tally::new();
        assert_eq!(tally.hits(), 0);
        let other = tally.clone();
        tally.hit();
        other.hit();
        other.hit();
        assert_eq!(
            tally.hits(),
            3,
            "a hit through a clone must be seen by the original"
        );
        assert_eq!(
            other.hits(),
            3,
            "a hit through the original must be seen by a clone"
        );
        assert_eq!(
            Tally::new().hits(),
            0,
            "a new tally must not share the count of an old one"
        );
    }

    #[test]
    fn c_tally_keeps_its_auto_traits() {
        assert!(
            implements!(Tally: Send),
            "Tally must be Send: users of 1.0.0 move clones into worker threads. \
             A field such as `Rc<..>` takes Send away from the whole type"
        );
        assert!(
            implements!(Tally: Sync),
            "Tally must be Sync: users of 1.0.0 share `&Tally` between threads. \
             A field such as `Cell<..>` or `RefCell<..>` takes Sync away"
        );
    }

    /// K. Predict BEFORE running: the class that the Cargo SemVer reference gives
    /// each change. T6 and T8 are not asked: the reference has no entry for them,
    /// and your witnesses show what they do. For T3, the reference has an entry
    /// for a required method and names sealing as the way out; decide which
    /// risk is left when nobody outside can implement the trait.
    #[test]
    fn k_classes() {
        let hint = "Look for this kind of change under \"Traits\" and \"Generics\" in the \
                    Cargo SemVer reference.";
        let k1_required_method: Class = Class::Major;
        let k2_provided_method: Class = Class::PossiblyBreaking;
        let k3_required_method_of_a_sealed_trait: Class = Class::PossiblyBreaking;
        let k4_tightened_bound: Class = Class::Major;
        let k5_loosened_bound: Class = Class::Minor;
        let k7_no_longer_dyn_compatible: Class = Class::Major;
        support::assert_answer(
            "T1 class",
            &format!("{k1_required_method:?}"),
            0x7fba_2786_9295_63ed,
            hint,
        );
        support::assert_answer(
            "T2 class",
            &format!("{k2_provided_method:?}"),
            0x6327_a229_48ec_c797,
            hint,
        );
        support::assert_answer(
            "T3 class",
            &format!("{k3_required_method_of_a_sealed_trait:?}"),
            0x3872_f6b2_b423_5738,
            "Who can implement a sealed trait? So the implementors' side of the change is \
             gone. What is left for the callers is the same as for T2.",
        );
        support::assert_answer(
            "T4 class",
            &format!("{k4_tightened_bound:?}"),
            0x9096_b9ff_1787_5aa8,
            hint,
        );
        support::assert_answer(
            "T5 class",
            &format!("{k5_loosened_bound:?}"),
            0x51d0_0d61_6d3f_40cd,
            hint,
        );
        support::assert_answer(
            "T7 class",
            &format!("{k7_no_longer_dyn_compatible:?}"),
            0x4ce1_6412_7666_8483,
            hint,
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// `implements!(Type: Bound)` is `true` when the type satisfies the bound.
    #[test]
    fn p_predictions() {
        use std::cell::Cell;
        use std::rc::Rc;
        use std::sync::{Arc, Mutex};

        // P1. A boxed trait object without `+ Send` in its type.
        let p1: bool = false;
        assert_eq!(
            p1,
            implements!(Box<dyn Iterator<Item = u8>>: Send),
            "P1: a trait object has exactly the auto traits that its type names"
        );

        // P2. `Arc` is the thread-safe pointer. Does that make this type Send?
        let p2: bool = false;
        assert_eq!(
            p2,
            implements!(Arc<Cell<u8>>: Send),
            "P2: `Arc<T>` is Send only when `T` is Send and Sync, because every \
             clone can reach the same `T` from its own thread"
        );

        // P3. And a mutex around the same cell: is it Sync?
        let p3: bool = true;
        assert_eq!(
            p3,
            implements!(Mutex<Cell<u8>>: Sync),
            "P3: `Mutex<T>` is Sync when `T` is Send: the lock lets one thread at \
             a time reach the `T`"
        );

        // P4. A struct with two fields. How many of the four bounds
        // (Send, Sync for each of the two types) hold?
        struct Shared {
            _name: String,
            _hits: Rc<Cell<u32>>,
        }
        struct Local {
            _name: String,
            _hits: Cell<u32>,
        }
        let p4: usize = 1;
        let held = [
            implements!(Shared: Send),
            implements!(Shared: Sync),
            implements!(Local: Send),
            implements!(Local: Sync),
        ];
        assert_eq!(
            p4,
            held.iter().filter(|held| **held).count(),
            "P4: a struct has an auto trait only when every field has it. \
             `Rc` is neither Send nor Sync; `Cell<u32>` is Send and not Sync"
        );
    }
}
