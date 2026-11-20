//! LESSON 06 — Structs and methods
//! Run: cargo test --test chapter02 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Two struct values are built separately with the same field values. Does
//!    `==` call them equal?
//! 2. Which methods of `Novel` below can change the novel, and how do you see
//!    that in their first parameter?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain what `&self` and `&mut self` mean in a method,
//! and why `Novel::new` has no `self` at all.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, define a
//! struct with two fields WITHOUT any `#[derive(...)]` line and try to print
//! a value of it with `println!("{:?}", value);`. Read the error and its
//! help. Repair it and leave the file compiling.

/// A novel on a reading list. The `derive` line lets tests print a `Novel`
/// with `{:?}` and compare two novels with `==`.
#[derive(Debug, PartialEq)]
struct Novel {
    title: String,
    pages: u32,
    pages_read: u32,
}

impl Novel {
    /// A. Build a novel that nobody has started to read: `pages_read` is 0.
    fn new(title: &str, pages: u32) -> Novel {
        todo!("A: build a Novel value with its three fields")
    }

    /// B. Return true when the novel has more than 300 pages.
    fn is_long(&self) -> bool {
        todo!("B: read self.pages")
    }

    /// C. Record that `pages` more pages were read. `pages_read` never
    /// becomes larger than the number of pages in the novel.
    fn read(&mut self, pages: u32) {
        todo!("C: change self.pages_read")
    }

    /// D. Return a progress line such as "Dune: 120 of 412 pages".
    fn progress(&self) -> String {
        todo!("D: format! with three fields")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_new_novel_is_unread() {
        let novel = Novel::new("Dune", 412);
        assert_eq!(novel.title, "Dune");
        assert_eq!(novel.pages, 412);
        assert_eq!(novel.pages_read, 0, "a new novel has no pages read");
        assert_eq!(
            novel,
            Novel {
                title: String::from("Dune"),
                pages: 412,
                pages_read: 0
            }
        );
    }

    #[test]
    fn b_long_novels_have_more_than_300_pages() {
        assert!(Novel::new("Dune", 412).is_long());
        assert!(!Novel::new("Pamphlet", 24).is_long());
        assert!(
            !Novel::new("Exactly", 300).is_long(),
            "300 pages are not MORE than 300"
        );
        assert!(Novel::new("Just over", 301).is_long());
    }

    #[test]
    fn c_reading_adds_pages() {
        let mut novel = Novel::new("Dune", 412);
        novel.read(100);
        novel.read(20);
        assert_eq!(novel.pages_read, 120);
        assert_eq!(
            novel.pages, 412,
            "reading does not change the length of the novel"
        );
    }

    #[test]
    fn c_reading_stops_at_the_last_page() {
        let mut novel = Novel::new("Short", 50);
        novel.read(40);
        novel.read(40);
        assert_eq!(
            novel.pages_read, 50,
            "40 + 40 pages were reported for a novel of 50 pages: pages_read must stop at 50"
        );
        let mut huge = Novel::new("Short", 50);
        huge.read(u32::MAX);
        huge.read(u32::MAX);
        assert_eq!(
            huge.pages_read, 50,
            "very large numbers must not overflow: compare before you add, or use \
             `saturating_add`"
        );
    }

    #[test]
    fn d_progress_line() {
        let mut novel = Novel::new("Dune", 412);
        assert_eq!(novel.progress(), "Dune: 0 of 412 pages");
        novel.read(120);
        assert_eq!(novel.progress(), "Dune: 120 of 412 pages");
    }

    // Two small types for the predictions.
    #[derive(Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    struct Counter {
        n: u32,
    }

    impl Counter {
        fn bump(&mut self) {
            self.n = self.n * 2 + 1;
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. What exactly does `{:?}` print for this value? Write the text.
        let p1: &str = guess();
        assert_eq!(
            p1,
            format!("{:?}", Point { x: 1, y: 2 }),
            "P1: the Debug text of Point {{ x: 1, y: 2 }} (left: your prediction, right: actual)"
        );

        // P2. Two points built separately, then one of them is changed.
        let a = Point { x: 1, y: 2 };
        let mut b = Point { x: 1, y: 2 };
        let equal_before = a == b;
        b.y = 3;
        let equal_after = a == b;
        let p2: (bool, bool) = guess();
        assert_eq!(
            p2,
            (equal_before, equal_after),
            "P2: (a == b before the change, a == b after it) \
             (left: your prediction, right: actual)"
        );

        // P3. A method that changes its value, called three times.
        let mut counter = Counter { n: 0 };
        counter.bump();
        counter.bump();
        counter.bump();
        let p3: u32 = guess();
        assert_eq!(
            p3, counter.n,
            "P3: counter.n after three calls of bump (left: your prediction, right: actual)"
        );
    }
}
