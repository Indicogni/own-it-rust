//! MIRI LAB 06 — A queue that mixes Box and raw pointers (lesson 06)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l06_queue
//!   MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l06_queue
//!
//! 1. PREDICT. Read the program. The `unsafe` block has a SAFETY comment.
//!    Decide what it leaves out, and at which line each of Miri's two
//!    aliasing models will stop.
//! 2. RUN both commands. Read the first `error:` line, the source line that it
//!    points to, and the two `help:` notes that say where the pointer was
//!    created and where it lost its permission.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 06.
//! 4. REPAIR: do not change this file. The repair is task B of the lesson: a
//!    queue whose nodes are reached only through raw pointers.
//!
//! A native `cargo run --example miri_l06_queue` prints the expected result
//! on every machine this was tried on. The program is still wrong: the
//! compiler is allowed to assume that a `Box` is the only way to its value.

use std::ptr;

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

/// A first-in, first-out queue. The nodes are owned through `head`. `tail` is
/// a shortcut to the last node, so that `push_back` need not walk the list.
struct Queue<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Node<T>,
}

impl<T> Queue<T> {
    fn new() -> Queue<T> {
        Queue {
            head: None,
            tail: ptr::null_mut(),
        }
    }

    fn push_back(&mut self, value: T) {
        let mut node = Box::new(Node { value, next: None });
        let raw: *mut Node<T> = &mut *node;
        if self.tail.is_null() {
            self.head = Some(node);
        } else {
            // SAFETY: `tail` points to the last node. That node is alive,
            // because the queue owns it through `head`, and a value on the
            // heap does not move when its `Box` is moved.
            unsafe { (*self.tail).next = Some(node) };
        }
        self.tail = raw;
    }

    fn pop_front(&mut self) -> Option<T> {
        let node = self.head.take()?;
        let Node { value, next } = *node;
        self.head = next;
        if self.head.is_none() {
            self.tail = ptr::null_mut();
        }
        Some(value)
    }

    fn peek_front(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }
}

fn main() {
    let mut queue = Queue::new();
    queue.push_back(String::from("a"));
    queue.push_back(String::from("b"));
    println!("front: {:?}", queue.peek_front());
    queue.push_back(String::from("c"));
    let mut seen = Vec::new();
    while let Some(word) = queue.pop_front() {
        seen.push(word);
    }
    println!("{seen:?}");
    assert_eq!(seen, ["a", "b", "c"]);
}
