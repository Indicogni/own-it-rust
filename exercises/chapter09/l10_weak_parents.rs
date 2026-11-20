//! LESSON 10 — Weak and parent links
//! Run: cargo test --test chapter09 l10_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: If a parent owns its children with Rc and every child also owns its parent with Rc, when is the tree freed? What does Weak::upgrade return after the last Rc is gone?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain which direction of a tree owns and which only observes, what the strong and weak counts measure, and why upgrade returns an Option.
//! COMPILER / TRANSFER LAB: Change your parent field to a strong Rc and run the tests; read which ones fail and work out why no folder is dropped. Restore Weak. Then try to read a field through a Weak without calling upgrade and inspect the diagnostic.
//! Use examples/ch09_scratch.rs; repair intentional failures before committing.
//!
//! Study order: do this lesson after lesson 08 and before the capstone (09).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A shared handle to one folder. `Rc` lets the tree and your variables hold
/// the same folder; `RefCell` lets you change it through any of the handles.
type Link = Rc<RefCell<Folder>>;

/// A. DESIGN: a folder knows its name, its parent folder (if it has one) and
/// its child folders in the order they were attached. Add the two missing
/// fields. A parent keeps its children alive. A child can find its parent,
/// but must NOT keep the parent alive. The tests use only the functions
/// below, so the field names and types are your decision.
struct Folder {
    name: String,
    // DESIGN: add a field for the parent and a field for the children.
}

// Test instrumentation: every dropped folder writes its name into a log.
// Do not change this block.
thread_local! {
    static DROPPED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
impl Drop for Folder {
    fn drop(&mut self) {
        DROPPED.with(|log| log.borrow_mut().push(self.name.clone()));
    }
}

/// A. Create a folder that has no parent and no children.
fn folder(name: &str) -> Link {
    todo!("Implement this task")
}
/// B. Put `child` inside `parent`. The parent gets a strong pointer to the
/// child, after the children it already has. The child gets a weak pointer
/// back to the parent. You may assume that `child` has no parent yet and that
/// the two handles are different folders.
fn attach(parent: &Link, child: &Link) {
    todo!("Implement this task")
}
/// C. The names of the children, in the order they were attached.
fn child_names(node: &Link) -> Vec<String> {
    todo!("Implement this task")
}
/// C. A handle to the parent. None if the folder has no parent, or if the
/// parent has already been dropped.
fn parent_of(node: &Link) -> Option<Link> {
    todo!("Implement this task")
}
/// D. Walk up the parent links and return a handle to the topmost folder
/// that is still alive. A folder without a live parent is its own root.
fn root_of(node: &Link) -> Link {
    todo!("Implement this task")
}
/// E. Take `child` out of its parent: remove exactly this folder from the
/// parent's children (compare pointers with `Rc::ptr_eq`; two folders may
/// share a name) and clear the child's parent link. Return false and change
/// nothing if the child has no live parent.
fn detach(child: &Link) -> bool {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};

    /// Returns the names dropped on this thread since the last call.
    fn take_drop_log() -> Vec<String> {
        DROPPED.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }

    #[test]
    fn a_c_new_folder_stands_alone() {
        let home = folder("home");
        assert_eq!(home.borrow().name, "home");
        assert!(parent_of(&home).is_none(), "a new folder has no parent");
        assert!(
            child_names(&home).is_empty(),
            "a new folder has no children"
        );
        assert_eq!(
            Rc::strong_count(&home),
            1,
            "only the caller owns a new folder"
        );
        assert_eq!(
            Rc::weak_count(&home),
            0,
            "nothing observes a new folder yet"
        );
    }
    #[test]
    fn b_attach_links_both_directions() {
        let root = folder("root");
        let src = folder("src");
        let docs = folder("docs");
        attach(&root, &src);
        attach(&root, &docs);
        assert_eq!(child_names(&root), ["src", "docs"]);
        assert!(child_names(&src).is_empty());
        let parent = parent_of(&docs).expect("docs was attached to root");
        assert!(
            Rc::ptr_eq(&parent, &root),
            "parent_of must return a handle to the same folder that attach received"
        );
        assert!(
            parent_of(&root).is_none(),
            "attach must not change the parent's own parent"
        );
    }
    #[test]
    fn b_counts_show_who_owns_whom() {
        let root = folder("root");
        let src = folder("src");
        let docs = folder("docs");
        attach(&root, &src);
        attach(&root, &docs);
        assert_eq!(
            Rc::strong_count(&src),
            2,
            "a child has two owners here: its parent and this test"
        );
        assert_eq!(
            Rc::strong_count(&root),
            1,
            "a child must not own its parent: keep a Weak, not an Rc, in the parent link"
        );
        assert_eq!(
            Rc::weak_count(&root),
            2,
            "each of the two children holds one weak pointer to the parent"
        );
        assert_eq!(Rc::weak_count(&src), 0, "nothing points weakly at a leaf");
    }
    #[test]
    fn b_parent_keeps_its_children_alive() {
        let root = folder("root");
        let watcher = {
            let tmp = folder("tmp");
            attach(&root, &tmp);
            Rc::downgrade(&tmp)
        }; // the test's own strong pointer to `tmp` is gone here
        let tmp = watcher.upgrade().expect(
            "the parent must own its children with Rc, so they outlive the caller's handle",
        );
        assert_eq!(child_names(&root), ["tmp"]);
        assert!(parent_of(&tmp).is_some_and(|p| Rc::ptr_eq(&p, &root)));
    }
    #[test]
    fn c_child_does_not_keep_its_parent_alive() {
        let child = folder("child");
        let root = folder("root");
        attach(&root, &child);
        let watcher = Rc::downgrade(&root);
        drop(root);
        assert!(
            watcher.upgrade().is_none(),
            "root is still alive after its only owner was dropped: the child's parent link must be Weak"
        );
        assert!(
            parent_of(&child).is_none(),
            "a parent that was dropped cannot be upgraded"
        );
        assert_eq!(
            Rc::strong_count(&child),
            1,
            "the dropped parent no longer owns the child"
        );
    }
    #[test]
    fn d_root_of_walks_up_through_the_parent_links() {
        // The walk runs on a helper thread: a loop that never moves up to the
        // parent fails with a message here instead of hanging the test run.
        let (from_leaf, from_middle, from_root, owners, after_drop) = within(
            5,
            "root_of (does your loop move up to the parent on every step?)",
            || {
                let root = folder("root");
                let src = folder("src");
                let bin = folder("bin");
                attach(&root, &src);
                attach(&src, &bin);
                let from_leaf = Rc::ptr_eq(&root_of(&bin), &root);
                let from_middle = Rc::ptr_eq(&root_of(&src), &root);
                let from_root = Rc::ptr_eq(&root_of(&root), &root);
                let owners = (
                    Rc::strong_count(&root),
                    Rc::strong_count(&src),
                    Rc::strong_count(&bin),
                );
                drop(root); // `src` is the topmost live folder now
                let after_drop = Rc::ptr_eq(&root_of(&bin), &src);
                (from_leaf, from_middle, from_root, owners, after_drop)
            },
        );
        assert!(from_leaf, "the root of a leaf is two levels up");
        assert!(from_middle, "the root of a middle folder is one level up");
        assert!(from_root, "a folder without a parent is its own root");
        assert_eq!(
            owners,
            (1, 2, 2),
            "after root_of returns and its result is dropped, the owners are the test's \
             variables and the parents' lists. More means that root_of kept a handle it \
             upgraded, or that a parent link is a strong Rc"
        );
        assert!(
            after_drop,
            "the walk stops at the first parent that no longer exists"
        );
    }
    #[test]
    fn e_detach_removes_exactly_that_child() {
        let root = folder("root");
        let first = folder("copy");
        let second = folder("copy"); // the same name, but a different folder
        let last = folder("last");
        attach(&root, &first);
        attach(&root, &second);
        attach(&root, &last);
        assert!(detach(&second));
        assert_eq!(child_names(&root), ["copy", "last"]);
        assert_eq!(
            Rc::strong_count(&first),
            2,
            "detach removed the wrong folder from the parent: compare with Rc::ptr_eq, not by name"
        );
        assert_eq!(
            Rc::strong_count(&second),
            1,
            "the parent must give up its strong pointer to the detached child"
        );
        assert!(
            parent_of(&second).is_none(),
            "detach must clear the child's parent link"
        );
        assert_eq!(
            Rc::weak_count(&root),
            2,
            "only the two remaining children still point back at root"
        );
        assert!(
            !detach(&second),
            "a folder without a parent cannot be detached again"
        );
        assert_eq!(child_names(&root), ["copy", "last"]);
        // A detached folder can be attached somewhere else.
        attach(&last, &second);
        assert!(parent_of(&second).is_some_and(|p| Rc::ptr_eq(&p, &last)));
        assert_eq!(child_names(&last), ["copy"]);
    }
    #[test]
    fn e_detach_needs_a_live_parent() {
        let lone = folder("lone");
        assert!(
            !detach(&lone),
            "a folder that was never attached has no parent"
        );
        let child = folder("child");
        {
            let root = folder("root");
            attach(&root, &child);
        }
        assert!(
            !detach(&child),
            "the parent was dropped, so there is nothing to detach from"
        );
    }
    #[test]
    fn t_dropping_the_handles_drops_the_whole_tree() {
        let _ = take_drop_log(); // start with an empty log on this thread
        {
            let root = folder("root");
            let src = folder("src");
            let bin = folder("bin");
            let docs = folder("docs");
            attach(&root, &src);
            attach(&src, &bin);
            attach(&root, &docs);
            assert!(
                take_drop_log().is_empty(),
                "building a tree must not drop any folder"
            );
        } // every handle of this test is dropped here
        let mut dropped = take_drop_log();
        dropped.sort();
        assert_eq!(
            dropped,
            ["bin", "docs", "root", "src"],
            "every folder must be dropped once the last handle is gone. A missing name is a \
             leak: a child that owns its parent with Rc forms a cycle that is never freed"
        );
    }

    struct Peer<'a> {
        name: &'static str,
        partner: RefCell<Option<Rc<Peer<'a>>>>,
        log: &'a RefCell<Vec<&'static str>>,
    }
    impl Drop for Peer<'_> {
        fn drop(&mut self) {
            self.log.borrow_mut().push(self.name);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. One Rc, two Weak pointers, and one upgrade that succeeded.
        // What are the strong and weak counts of `owner` now?
        let owner = Rc::new(String::from("root"));
        let observer = Rc::downgrade(&owner);
        let second_observer = Weak::clone(&observer);
        let upgraded = observer.upgrade();
        let p1: (usize, usize) = guess(); // (strong, weak)
        assert_eq!(
            p1,
            (Rc::strong_count(&owner), Rc::weak_count(&owner)),
            "P1: (strong count, weak count)"
        );

        // P2. Both strong pointers are dropped; the two Weak pointers remain.
        // Does upgrade still give a value, and what strong count does the
        // Weak report?
        drop(upgraded);
        drop(owner);
        let p2: (bool, usize) = guess(); // (upgrade().is_some(), strong count)
        assert_eq!(
            p2,
            (observer.upgrade().is_some(), second_observer.strong_count()),
            "P2: a Weak after the last Rc is gone"
        );

        // P3. Two peers own each other with Rc. Both handles are dropped.
        // How many peers have been dropped at that point? And how many after
        // the cycle is broken by hand?
        let log = RefCell::new(Vec::new());
        let a = Rc::new(Peer {
            name: "a",
            partner: RefCell::new(None),
            log: &log,
        });
        let b = Rc::new(Peer {
            name: "b",
            partner: RefCell::new(Some(Rc::clone(&a))),
            log: &log,
        });
        *a.partner.borrow_mut() = Some(Rc::clone(&b)); // a owns b, b owns a
        let watcher = Rc::downgrade(&a);
        drop(a);
        drop(b);
        let while_cycle_exists = log.borrow().len();
        if let Some(a) = watcher.upgrade() {
            let b = a.partner.borrow_mut().take(); // a no longer owns b
            drop(b);
        }
        let p3: (usize, usize) = guess(); // (while the cycle exists, after breaking it)
        assert_eq!(
            p3,
            (while_cycle_exists, log.borrow().len()),
            "P3: drops logged for a strong cycle, before and after it is broken"
        );
    }
}
