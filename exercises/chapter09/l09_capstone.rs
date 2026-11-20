//! LESSON 09 — Capstone: validated inventory
//! Run: cargo test --test chapter09 l09_
//!
//! PREDICT before running: Which fields does the type need, which methods mutate inventory, and which statistic may change through a shared borrow?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain the fields you chose, the constructor invariant, atomic failure of restock, and why Cell is appropriate for a single-threaded read statistic.
//! COMPILER / TRANSFER LAB: Move the type into a disposable library module and attempt direct field construction from an external binary. Add a maximum stock invariant and revise every constructor/mutator. Discuss the changes needed for sharing read statistics across threads.
//! Use examples/ch09_scratch.rs; repair intentional failures before committing.
//!
//! In this lesson the struct is yours to design. The method signatures are
//! fixed and the tests call only those methods, so the field names and field
//! types are your decision. One test checks the size of your type.

/// One stock line: a name, a quantity, and a count of how often it was read.
struct Inventory {
    // DESIGN: add your fields here.
}
impl Inventory {
    /// A. Trim and own a nonempty name; the read count starts at zero.
    /// Return None if the name is empty after trimming. Allocate at most once.
    fn new(name: &str, stock: u16) -> Option<Self> {
        todo!("Implement this task")
    }
    /// B. Add stock if it fits u16; return false without mutation on overflow.
    fn restock(&mut self, amount: u16) -> bool {
        todo!("Implement this task")
    }
    /// C. Return (borrowed_name, stock) and count one read through &self,
    /// saturating at u8::MAX. Do not clone the name. No other method counts
    /// as a read.
    fn inspect(&self) -> (&str, u16) {
        todo!("Implement this task")
    }
    /// C. How many times `inspect` ran. Calling `reads` is not a read.
    fn reads(&self) -> u8 {
        todo!("Implement this task")
    }
    /// D. Finish the inventory and give its name to the caller: move the
    /// stored text out, do not copy it.
    fn into_name(self) -> String {
        todo!("Implement this task")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_constructor_establishes_invariants() {
        let i = Inventory::new(" 港 ", 3).unwrap();
        assert_eq!(i.reads(), 0, "a new inventory has not been read yet");
        assert_eq!(i.inspect(), ("港", 3));
        assert!(Inventory::new(" \n", 0).is_none());
        assert!(Inventory::new("", 1).is_none());
        assert!(
            Inventory::new("\u{3000}", 1).is_none(),
            "U+3000 is whitespace"
        );
        assert_eq!(Inventory::new("\tbin 4\n", 0).unwrap().into_name(), "bin 4");
    }
    #[test]
    fn b_failed_mutation_leaves_stock_unchanged() {
        let mut i = Inventory::new("x", 65530).unwrap();
        assert!(i.restock(5));
        assert!(!i.restock(1), "65535 + 1 does not fit in a u16");
        assert_eq!(i.reads(), 0, "restock must not count as a read");
        assert_eq!(
            i.inspect().1,
            65535,
            "a rejected restock must leave the stock as it was"
        );
        assert!(i.restock(0));
        assert_eq!(i.inspect().1, 65535);
    }
    #[test]
    fn c_observation_updates_only_the_statistic() {
        let i = Inventory::new("x", 8).unwrap();
        assert_eq!(i.inspect(), ("x", 8));
        assert_eq!(i.inspect(), ("x", 8));
        assert_eq!(i.reads(), 2, "two calls to inspect are two reads");
        assert_eq!(i.reads(), 2, "calling reads must not count as a read");
        for _ in 0..300 {
            i.inspect();
        }
        assert_eq!(
            i.reads(),
            u8::MAX,
            "the read count must stop at u8::MAX, not wrap around"
        );
        assert_eq!(i.inspect(), ("x", 8), "reading never changes the stock");
    }
    #[test]
    fn a_c_owns_the_name_once_and_never_clones_it() {
        let (i, allocations) = count(|| Inventory::new("  crates  ", 1));
        let i = i.unwrap();
        assert!(
            allocations <= 1,
            "owning a trimmed name needs one allocation, not {allocations}"
        );
        let (view, allocations) = count(|| i.inspect().0.len());
        assert_eq!(view, 6);
        assert_eq!(allocations, 0, "inspect should borrow the name");
    }
    #[test]
    fn d_owned_receiver_gives_the_name_away() {
        let i = Inventory::new("  crates  ", 1).unwrap();
        let stored = i.inspect().0.as_ptr();
        let (name, allocations) = count(|| i.into_name());
        assert_eq!(name, "crates");
        assert_eq!(
            name.as_ptr(),
            stored,
            "into_name must hand over the stored text itself, not a copy of it"
        );
        assert_eq!(allocations, 0, "moving the name out needs no allocation");
    }
    #[test]
    fn t_design_uses_the_smallest_tool_for_the_statistic() {
        let i = Inventory::new("x", 1).unwrap();
        let limit = size_of::<String>() + size_of::<usize>();
        assert!(
            size_of_val(&i) <= limit,
            "Inventory is {} bytes; an owned name, a u16 and a one-byte counter fit in {limit}. \
             A Cell<u8> adds one byte, while a RefCell<u8> also stores a borrow flag",
            size_of_val(&i)
        );
    }
}
