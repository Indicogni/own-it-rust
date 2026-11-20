//@ expect: run
//@ why: Shouter must lend every line as a &str from one buffer that is allocated in `new`; `next` must not allocate.
use m1_types::l03_lending::{LendingIterator, Shouter};
use m1_types::support::alloc::{Counting, count};

#[global_allocator]
static ALLOC: Counting = Counting;

const EXPECTED: [&str; 4] = ["FIRST LINE", "", "THE THIRD LINE IS THE LONGEST", "END"];

fn main() {
    let text = String::from("first line\n\nthe third line is the longest\nend");
    let mut shouter = Shouter::new(&text);

    // Nothing in this closure allocates, so every allocation that is counted
    // was made by `next`.
    let (lines, allocations) = count(|| {
        let mut lines = 0;
        let mut addresses = [std::ptr::null::<u8>(); 4];
        while let Some(line) = shouter.next() {
            // The item is a plain string slice, lent by the shouter.
            let line: &str = line;
            assert!(lines < 4, "Shouter::next never returned None");
            assert_eq!(line, EXPECTED[lines]);
            addresses[lines] = line.as_ptr();
            lines += 1;
        }
        assert!(
            addresses.iter().all(|address| *address == addresses[0]),
            "every line must be lent from the same buffer"
        );
        lines
    });
    assert_eq!(lines, 4, "an empty line is a line");
    assert_eq!(
        allocations, 0,
        "Shouter::next made {allocations} heap allocation(s) for 4 lines: reserve the buffer \
         once in `new` and refill it in `next`"
    );

    let mut one = Shouter::new("MiXed 123 ß");
    assert_eq!(one.next(), Some("MIXED 123 ß"), "only ASCII letters change");
    assert_eq!(one.next(), None);
    assert_eq!(Shouter::new("").count(), 0);
}
