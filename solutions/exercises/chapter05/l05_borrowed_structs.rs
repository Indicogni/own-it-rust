//! LESSON 05 — Structs that borrow
//! Run: cargo test --test chapter05 l05_
//! Read first: CONCEPTS.md, section "Lesson 05 · Structs that borrow".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can a borrowed view outlive its source String? Does moving the view move its source?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain why the struct has a lifetime parameter and how fields remain tied to one owner.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Construct a view of a String in an inner scope, store the view outside, then try using it after the String drops. Move only the view while keeping the String alive and compare.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

#[derive(Debug, PartialEq)]
struct Record<'a> {
    key: &'a str,
    value: &'a str,
}
/// A. Build a borrowed record at the first '='. Trim each side, reject an empty
/// key or a missing '='; allow an empty value. No owned String allocations.
fn parse_view(line: &str) -> Option<Record<'_>> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some(Record {
        key,
        value: value.trim(),
    })
}
/// B. Return the borrowed value field, preserving the SOURCE lifetime rather
/// than unnecessarily tying it to the short borrow of the Record wrapper.
fn value<'a>(record: &Record<'a>) -> &'a str {
    // `record.value` is a `&'a str`. Copying that reference out keeps the
    // source lifetime 'a; it does not depend on how long `record` is borrowed.
    record.value
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    #[test]
    fn a_view_validates_without_owning() {
        let s = String::from(" mode = calm ");
        let r = parse_view(&s).unwrap();
        assert_eq!(
            r,
            Record {
                key: "mode",
                value: "calm"
            }
        );
        assert_eq!(
            r.key.as_ptr(),
            s[1..].as_ptr(),
            "the key must point into the line (one byte in, after the space)"
        );
        assert_eq!(
            parse_view("x="),
            Some(Record {
                key: "x",
                value: ""
            })
        );
        assert_eq!(
            parse_view("url = a=b"),
            Some(Record {
                key: "url",
                value: "a=b"
            })
        );
        assert_eq!(parse_view(" = x"), None);
        assert_eq!(parse_view("x"), None);
    }
    #[test]
    fn b_source_can_outlive_wrapper() {
        let source = String::from("k=v");
        let out;
        {
            let record = parse_view(&source).unwrap();
            out = value(&record);
        }
        assert_eq!(out, "v");
    }

    #[test]
    fn a_parse_view_does_not_allocate() {
        let line = String::from("  mode = calm  ");
        let (record, allocations) = count(|| parse_view(&line));
        assert_eq!(record.map(|r| r.value), Some("calm"));
        assert_eq!(allocations, 0, "a view borrows from the line");
    }

    /// Test helper: a value that writes its name into `log` at the moment it is
    /// dropped. You only need to know WHAT it does to answer the prediction;
    /// how it is written (`Drop`, `RefCell`) comes in later chapters.
    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }
    struct View<'a, 'b>(&'a Noisy<'b>);

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Size of a Record, in machine words.
        let p1: usize = 4;
        assert_eq!(
            p1,
            size_of::<Record>() / size_of::<usize>(),
            "P1: a struct of two &str"
        );

        // P2. Heap allocations made by building a view and moving it.
        let source = String::from("k=v");
        let (len, allocations) = count(|| {
            let record = Record {
                key: &source[..1],
                value: &source[2..],
            };
            let moved = record;
            moved.value.len()
        });
        let p2: usize = 0;
        assert_eq!(p2, allocations, "P2: moving a view copies two fat pointers");
        assert_eq!(len, 1);

        // P3. A view goes out of scope first: is its referent dropped too?
        let log = RefCell::new(Vec::new());
        {
            let owner = Noisy("owner", &log);
            {
                let _view = View(&owner);
            }
            log.borrow_mut().push("view dropped");
        }
        let p3: Vec<&str> = vec!["view dropped", "owner"];
        assert_eq!(p3, *log.borrow(), "P3: a borrow never drops its referent");
    }
}
