//! LESSON 09 — Capstone: generic ordered tree
//! Run: cargo test --test chapter10 l09_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Where should a duplicate go?
//! 2. How does inserting values in sorted order change the depth of the tree?
//! 3. Does storing a generic `T` require `T: Clone`?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain recursive mutable matching, why a node needs
//! indirection and why one Box per node is enough, and how inorder traversal
//! shows that the ordering invariant holds.
//!
//! COMPILER / TRANSFER LAB (optional, not checked by tests): Add a `len`
//! method and a test for the duplicate policy. Move the Box from the variant
//! to the two child fields (`left: Box<Tree<T>>`) and count the allocations of
//! one insert again. Explain why a balanced tree, or the standard BTreeSet, is
//! usually the better choice in production code.
//! Use examples/ch10_scratch.rs, and restore it to compiling code afterwards.

use std::cmp::Ordering;

/// An ordered set. Every value in `left` is smaller than `value`, and every
/// value in `right` is larger.
///
/// The Box wraps the whole node. An empty tree and an empty child therefore
/// use no heap memory, and each stored value costs exactly one allocation.
enum Tree<T> {
    Empty,
    Node(Box<Node<T>>),
}
struct Node<T> {
    value: T,
    left: Tree<T>,
    right: Tree<T>,
}
impl<T: Ord> Tree<T> {
    /// A. Insert an owned value, returning true only when a new node is added.
    /// Equal values are ignored (return false). Do not clone or require Copy.
    /// A new value costs one allocation: the Box of its node.
    fn insert(&mut self, new: T) -> bool {
        match self {
            Tree::Empty => {
                // The empty place is where the value belongs: overwrite it.
                *self = Tree::Node(Box::new(Node {
                    value: new,
                    left: Tree::Empty,
                    right: Tree::Empty,
                }));
                true
            }
            // One comparison gives three outcomes, and `match` makes sure
            // that none of them is forgotten.
            Tree::Node(node) => match new.cmp(&node.value) {
                Ordering::Less => node.left.insert(new),
                Ordering::Greater => node.right.insert(new),
                Ordering::Equal => false,
            },
        }
    }
    /// B. Query by shared reference without changing the tree. Use the
    /// ordering: at each node, continue into at most one child.
    fn contains(&self, needle: &T) -> bool {
        match self {
            Tree::Empty => false,
            Tree::Node(node) => match needle.cmp(&node.value) {
                Ordering::Less => node.left.contains(needle),
                Ordering::Greater => node.right.contains(needle),
                Ordering::Equal => true,
            },
        }
    }
    /// C. Append borrowed values in sorted order to out, preserving any
    /// existing prefix. Return borrows tied to this tree; no Clone bound.
    fn inorder<'a>(&'a self, out: &mut Vec<&'a T>) {
        if let Tree::Node(node) = self {
            node.left.inorder(out);
            out.push(&node.value);
            node.right.inorder(out);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    use std::cmp::Ordering;
    use std::collections::BTreeSet;
    use std::hint::black_box;
    #[test]
    fn a_insert_maintains_unique_nodes() {
        let mut t = Tree::Empty;
        assert!(t.insert(String::from("m")));
        assert!(t.insert("a".into()));
        assert!(t.insert("z".into()));
        assert!(!t.insert("m".into()));
        assert!(t.contains(&"a".into()));
        assert!(!t.contains(&"b".into()));
    }
    #[test]
    fn a_smaller_values_go_left() {
        let mut t = Tree::Empty;
        for n in [4, 2, 7, 3] {
            assert!(t.insert(n));
        }
        let Tree::Node(root) = &t else {
            panic!("the first value becomes the root");
        };
        assert_eq!(root.value, 4);
        assert!(
            matches!(&root.right, Tree::Node(node) if node.value == 7),
            "7 is larger than 4, so it goes right of the root"
        );
        let Tree::Node(two) = &root.left else {
            panic!("2 goes left of 4");
        };
        assert_eq!(two.value, 2);
        assert!(matches!(two.left, Tree::Empty), "nothing is smaller than 2");
        assert!(
            matches!(&two.right, Tree::Node(node) if node.value == 3),
            "3 is smaller than 4 and larger than 2, so it goes right of 2"
        );
    }
    #[test]
    fn a_duplicates_are_rejected_without_allocating() {
        let mut t = Tree::Empty;
        for s in ["m", "a", "z"] {
            t.insert(s.to_owned());
        }
        let dup = String::from("a");
        let (added, allocations) = count(|| t.insert(dup));
        assert!(!added);
        assert_eq!(allocations, 0, "a rejected value needs no new node");
        let needle = String::from("z");
        let (found, allocations) = count(|| t.contains(&needle));
        assert!(found);
        assert_eq!(allocations, 0, "contains must not clone the tree's values");
    }
    #[test]
    fn a_new_value_moves_into_one_new_node() {
        let mut t = Tree::Empty;
        for s in ["m", "a"] {
            t.insert(s.to_owned());
        }
        let value = String::from("z");
        let p = value.as_ptr();
        let (added, allocations) = count(|| t.insert(value));
        assert!(added);
        assert!(
            allocations <= 1,
            "inserting one value made {allocations} allocations; only the Box of \
             the new node is needed"
        );
        let Tree::Node(root) = &t else {
            panic!("the tree has a root");
        };
        let Tree::Node(z) = &root.right else {
            panic!("z goes right of m");
        };
        assert_eq!(
            z.value.as_ptr(),
            p,
            "the node must own the String that was passed in, not a copy"
        );
    }
    #[test]
    fn b_search_finds_exactly_the_stored_values() {
        let mut t = Tree::Empty;
        assert!(!t.contains(&4));
        for n in [4, 2, 7, 1, 3, 6, 9] {
            t.insert(n);
        }
        for n in [1, 2, 3, 4, 6, 7, 9] {
            assert!(t.contains(&n));
        }
        for n in [0, 5, 8, 10] {
            assert!(!t.contains(&n));
        }
    }

    /// A key that records every key taking part in a comparison.
    struct Probe<'a> {
        key: i32,
        seen: &'a RefCell<BTreeSet<i32>>,
    }
    impl Ord for Probe<'_> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.seen.borrow_mut().extend([self.key, other.key]);
            self.key.cmp(&other.key)
        }
    }
    impl PartialOrd for Probe<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl PartialEq for Probe<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.cmp(other) == Ordering::Equal
        }
    }
    impl Eq for Probe<'_> {}

    #[test]
    fn b_search_follows_one_path() {
        let seen = RefCell::new(BTreeSet::new());
        let probe = |key| Probe { key, seen: &seen };
        let mut t = Tree::Empty;
        for key in [40, 20, 60, 10, 30, 50, 70] {
            t.insert(probe(key));
        }

        // 35 is missing. An ordered search compares it with 40, 20 and 30,
        // finds an empty child, and stops.
        seen.borrow_mut().clear();
        assert!(!t.contains(&probe(35)));
        assert_eq!(
            seen.borrow().iter().copied().collect::<Vec<_>>(),
            [20, 30, 35, 40],
            "contains(35) may only compare with the nodes on one path from the \
             root (40, 20, 30); these are all the keys that took part"
        );

        seen.borrow_mut().clear();
        assert!(t.contains(&probe(50)));
        assert_eq!(
            seen.borrow().iter().copied().collect::<Vec<_>>(),
            [40, 50, 60],
            "contains(50) may only compare with 40, 60 and 50; these are all the \
             keys that took part"
        );
    }
    #[test]
    fn c_inorder_borrows_sorted_values() {
        let mut t = Tree::Empty;
        for s in ["m", "a", "z", "b", "a"] {
            t.insert(s.to_owned());
        }
        let prefix = String::from("prefix");
        let mut out = vec![&prefix];
        t.inorder(&mut out);
        assert_eq!(
            out.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            ["prefix", "a", "b", "m", "z"]
        );
        let empty: Tree<String> = Tree::Empty;
        let mut none = Vec::new();
        empty.inorder(&mut none);
        assert!(none.is_empty());
    }
    #[test]
    fn c_inorder_borrows_into_the_tree() {
        let mut t = Tree::Empty;
        for n in [50, 20, 80, 10, 30, 70, 90, 60] {
            t.insert(n);
        }
        let mut out = Vec::with_capacity(16);
        let (_, allocations) = count(|| t.inorder(&mut out));
        assert_eq!(
            out.iter().map(|n| **n).collect::<Vec<_>>(),
            [10, 20, 30, 50, 60, 70, 80, 90]
        );
        assert_eq!(
            allocations, 0,
            "push borrows into `out`; no temporary vectors"
        );
        let Tree::Node(root) = &t else { unreachable!() };
        assert!(
            std::ptr::eq(out[3], &root.value),
            "out must hold references to the values stored in the tree; the \
             fourth one is the root's value"
        );
    }

    fn levels<T>(tree: &Tree<T>) -> usize {
        match tree {
            Tree::Empty => 0,
            Tree::Node(node) => 1 + levels(&node.left).max(levels(&node.right)),
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// These predictions run your `insert`, so they can only pass once task A
    /// works. Write them down first anyway.
    #[test]
    fn p_predictions() {
        // P1. The same seven values are inserted in two different orders:
        // first 4, 2, 6, 1, 3, 5, 7 and then 1, 2, 3, 4, 5, 6, 7.
        // How many levels does each tree have? (An empty tree has 0.)
        let p1: (usize, usize) = (3, 7);
        let mut mixed = Tree::Empty;
        for n in [4, 2, 6, 1, 3, 5, 7] {
            mixed.insert(n);
        }
        let mut sorted = Tree::Empty;
        for n in 1..=7 {
            sorted.insert(n);
        }
        assert_eq!(
            p1,
            (levels(&mixed), levels(&sorted)),
            "P1: insertion order decides the shape"
        );

        // P2. How many heap allocations do these five inserts make in total?
        // Look at where the Box is in the type, and note the duplicate.
        let p2: usize = 4;
        let mut t = Tree::Empty;
        let (_, allocations) = count(|| {
            for n in [5, 3, 8, 3, 9] {
                t.insert(black_box(n));
            }
        });
        black_box(&t);
        assert_eq!(
            p2, allocations,
            "P2: one Box per stored value, none for empty children"
        );
    }
}
