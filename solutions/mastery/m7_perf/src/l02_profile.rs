//! LESSON 02 — Reading a profile: self cost, total cost, and counted work
//! Read first: CONCEPTS.md, "Lesson 02". Start after: lesson 01; chapters 13
//! (`Drop`) and 9 (`RefCell`). Ecosystem E08 (reading folded stacks) helps.
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l02_
//! Lab: LABS.md, "Lab 2" records a real profile with a sampling profiler.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a profile says that `sort` has a
//! total cost of 60% and a self cost of 2%. Where do you look next, and what
//! would a self cost of 58% have told you instead?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! In A you build an instrumenting profiler that counts work in units instead
//! of time. In B you use it on a supplied slow function: read the profile it
//! prints, decide what to change, and write a version that meets a budget.
//! EXPLAIN afterward: What is the difference between the self cost and the
//! total cost of a frame, and which one do you sort by to find code to change?
//! Why must `charge` be cheap, and what would happen to a profile if it were
//! not? Before B, which frame did you expect to be the most expensive, and
//! which one was it?
//! JUDGMENT: An instrumenting profiler like this one is exact, but it measures
//! only what you thought of instrumenting, and its own cost changes the
//! program. A sampling profiler (samply, perf, Instruments) needs no changes to
//! the code and sees everything, including the standard library and the
//! kernel, but it is statistical. Start with a sampling profiler to find the
//! region; count work, as here, when you need a number that is exact and
//! stable enough to put into a test.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

/// A. An instrumenting profiler that counts abstract cost units per call path.
///
/// A *frame* is one function-like region with a name. Frames nest: the frame
/// opened last and not yet closed is the *current* frame. A *call path* is the
/// list of names from the outermost open frame to a frame, written
/// `outer;middle;inner`.
///
/// The fields are yours to choose. Every method takes `&self`, so you need
/// interior mutability (`Cell`, `RefCell`).
pub struct Profiler {
    /// The call tree. Node 0 is a pseudo-frame for cost that is charged while
    /// no span is open; every other node is one call path.
    nodes: RefCell<Vec<Node>>,
    /// Index of the current frame's node.
    current: Cell<usize>,
}

/// One call path: the node's name appended to its parent's path.
struct Node {
    name: &'static str,
    parent: usize,
    children: Vec<usize>,
    self_cost: u64,
    calls: u64,
}

/// The path that receives cost charged while no span is open.
const NO_SPAN: &str = "(no span)";

/// Closes its frame when it is dropped. You may change the private fields.
#[must_use = "the frame closes when this span is dropped; bind it: `let _span = ...`"]
pub struct Span<'p> {
    profiler: &'p Profiler,
    /// The node that was current before this span opened.
    parent: usize,
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        self.profiler.current.set(self.parent);
    }
}

impl Profiler {
    pub fn new() -> Profiler {
        Profiler {
            nodes: RefCell::new(vec![Node {
                name: NO_SPAN,
                parent: 0,
                children: Vec::new(),
                self_cost: 0,
                calls: 0,
            }]),
            current: Cell::new(0),
        }
    }

    /// Opens a frame named `name` inside the current frame and makes it the
    /// current one. The frame closes when the returned span is dropped; the
    /// frame that was current before becomes current again. Spans are dropped
    /// in the reverse order of their creation.
    ///
    /// Entering a call path that has been entered before must not allocate.
    pub fn enter(&self, name: &'static str) -> Span<'_> {
        let parent = self.current.get();
        let mut nodes = self.nodes.borrow_mut();
        let known = nodes[parent]
            .children
            .iter()
            .copied()
            .find(|&child| nodes[child].name == name);
        let node = known.unwrap_or_else(|| {
            let node = nodes.len();
            nodes.push(Node {
                name,
                parent,
                children: Vec::new(),
                self_cost: 0,
                calls: 0,
            });
            nodes[parent].children.push(node);
            node
        });
        nodes[node].calls += 1;
        self.current.set(node);
        Span {
            profiler: self,
            parent,
        }
    }

    /// Adds `units` to the self cost of the current frame. Cost that is
    /// charged while no frame is open goes to the call path `(no span)`.
    ///
    /// This is called from the hottest loops of the profiled program, so it
    /// must not allocate.
    pub fn charge(&self, units: u64) {
        self.nodes.borrow_mut()[self.current.get()].self_cost += units;
    }

    /// How many times a frame named `name` was opened, on any call path.
    pub fn calls(&self, name: &str) -> u64 {
        let nodes = self.nodes.borrow();
        nodes
            .iter()
            .skip(1)
            .filter(|node| node.name == name)
            .map(|node| node.calls)
            .sum()
    }

    /// The self cost of `name`: all units charged while a frame with that name
    /// was the current frame, added up over every call path.
    pub fn self_cost(&self, name: &str) -> u64 {
        let nodes = self.nodes.borrow();
        nodes
            .iter()
            .filter(|node| node.name == name)
            .map(|node| node.self_cost)
            .sum()
    }

    /// The total cost of `name`: all units charged while a frame with that
    /// name was open, that is, its self cost plus everything charged in frames
    /// below it. A unit is counted once, also when `name` appears several
    /// times on one call path (recursion).
    pub fn total_cost(&self, name: &str) -> u64 {
        let nodes = self.nodes.borrow();
        // A unit charged at `node` counts if `name` is on the path to `node`.
        let on_path = |mut node: usize| {
            loop {
                if nodes[node].name == name {
                    return true;
                }
                if node == 0 {
                    return false;
                }
                node = nodes[node].parent;
            }
        };
        (0..nodes.len())
            .filter(|&node| on_path(node))
            .map(|node| nodes[node].self_cost)
            .sum()
    }

    /// The profile as *folded stacks*, the text format that flamegraph tools
    /// read: one line `path cost` per call path whose self cost is not zero,
    /// for example `main;load;parse 30`. Lines are sorted by path (plain
    /// string order) and each line ends with `\n`.
    pub fn folded(&self) -> String {
        let mut lines = self.paths();
        lines.sort();
        lines
            .iter()
            .map(|(path, cost)| format!("{path} {cost}\n"))
            .collect()
    }

    /// The `n` call paths with the highest self cost, highest first, as
    /// `(path, self cost)`. Paths with equal cost are ordered by path. Paths
    /// with a self cost of zero are left out.
    pub fn hot_paths(&self, n: usize) -> Vec<(String, u64)> {
        let mut paths = self.paths();
        paths.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        paths.truncate(n);
        paths
    }

    /// Every call path with a self cost above zero, in no particular order.
    fn paths(&self) -> Vec<(String, u64)> {
        let nodes = self.nodes.borrow();
        let mut paths = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if node.self_cost == 0 {
                continue;
            }
            let mut names = vec![node.name];
            let mut at = index;
            while nodes[at].parent != 0 {
                at = nodes[at].parent;
                names.push(nodes[at].name);
            }
            names.reverse();
            paths.push((names.join(";"), node.self_cost));
        }
        paths
    }
}

impl Default for Profiler {
    fn default() -> Profiler {
        Profiler::new()
    }
}

/// A value that charges one unit to a profiler for every comparison and for
/// every clone. Supplied; do not edit it. It stands for a value that is
/// expensive to compare and to copy, such as a long string.
pub struct Key<'p> {
    pub id: u32,
    profiler: &'p Profiler,
}

impl std::fmt::Debug for Key<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Key({})", self.id)
    }
}

impl<'p> Key<'p> {
    pub fn new(id: u32, profiler: &'p Profiler) -> Key<'p> {
        Key { id, profiler }
    }
}

impl PartialEq for Key<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.profiler.charge(1);
        self.id == other.id
    }
}

impl Eq for Key<'_> {}

impl PartialOrd for Key<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.profiler.charge(1);
        self.id.cmp(&other.id)
    }
}

impl Clone for Key<'_> {
    fn clone(&self) -> Self {
        self.profiler.charge(1);
        Key {
            id: self.id,
            profiler: self.profiler,
        }
    }
}

/// The distinct keys of `keys`, in ascending order. Supplied: correct and
/// slow. Do not edit it; it is the program that you profile in B.
pub fn distinct_sorted_slow<'p>(keys: &[Key<'p>], profiler: &'p Profiler) -> Vec<Key<'p>> {
    let _whole = profiler.enter("distinct_sorted_slow");
    let mut distinct: Vec<Key<'p>> = Vec::new();
    for key in keys {
        let known = {
            let _span = profiler.enter("search");
            distinct.contains(key)
        };
        if !known {
            let _span = profiler.enter("insert");
            distinct.push(key.clone());
        }
        let _span = profiler.enter("keep_sorted");
        distinct.sort();
    }
    distinct
}

/// B. The distinct keys of `keys`, in ascending order: the same result as
/// [`distinct_sorted_slow`], for a small fraction of the cost.
///
/// First finish A and run
/// `cargo test --manifest-path mastery/m7_perf/Cargo.toml l02_ -- b_show --nocapture`
/// to print the profile of the slow version. Write down in NOTES.md which
/// frame you will remove first and why, then write this function.
///
/// Budget: for `n` keys, at most `n * (2 * L + 3)` units in total
/// (comparisons plus clones), where `L` is `log2(n)` rounded up. Open
/// whatever spans you find useful.
pub fn distinct_sorted<'p>(keys: &[Key<'p>], profiler: &'p Profiler) -> Vec<Key<'p>> {
    let _whole = profiler.enter("distinct_sorted");
    // Sort references, so that only the keys that survive are cloned.
    let mut order: Vec<&Key<'p>> = keys.iter().collect();
    {
        let _span = profiler.enter("sort");
        // One sort of everything: about n * log2(n) comparisons. A stable
        // sort is not needed, because equal keys are interchangeable here.
        order.sort_unstable();
    }
    {
        let _span = profiler.enter("dedup");
        // Equal keys are neighbours now: one comparison per neighbour pair.
        order.dedup();
    }
    let _span = profiler.enter("clone");
    order.into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;

    /// A tiny deterministic generator (a linear congruential generator).
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    fn sample_ids(n: usize, distinct: u64) -> Vec<u32> {
        let mut state = 7;
        (0..n)
            .map(|_| (next(&mut state) % distinct) as u32)
            .collect()
    }

    /// Fails with a pointer to task A while the profiler is unfinished, so
    /// that the tests of B do not fail with a puzzling number.
    fn profiler_works() {
        let profiler = Profiler::new();
        {
            let _outer = profiler.enter("outer");
            profiler.charge(1);
            let _inner = profiler.enter("inner");
            profiler.charge(2);
        }
        assert!(
            profiler.total_cost("outer") == 3 && profiler.self_cost("inner") == 2,
            "finish task A first: the tests of B read costs from your Profiler"
        );
    }

    fn sample_profile() -> Profiler {
        let profiler = Profiler::new();
        {
            let _main = profiler.enter("main");
            profiler.charge(2);
            {
                let _load = profiler.enter("load");
                profiler.charge(5);
                for _ in 0..10 {
                    let _parse = profiler.enter("parse");
                    profiler.charge(3);
                }
            }
            {
                let _save = profiler.enter("save");
                profiler.charge(4);
                let _parse = profiler.enter("parse");
                profiler.charge(1);
            }
        }
        profiler
    }

    #[test]
    fn a_self_cost_counts_only_the_frame_itself() {
        let profiler = sample_profile();
        assert_eq!(profiler.self_cost("main"), 2);
        assert_eq!(profiler.self_cost("load"), 5);
        assert_eq!(profiler.self_cost("save"), 4);
        assert_eq!(
            profiler.self_cost("parse"),
            31,
            "the self cost of a name adds up over all call paths: 10 * 3 under load, 1 under save"
        );
        assert_eq!(profiler.self_cost("never_entered"), 0);
    }

    #[test]
    fn a_total_cost_includes_the_frames_below() {
        let profiler = sample_profile();
        assert_eq!(profiler.total_cost("main"), 42);
        assert_eq!(
            profiler.total_cost("load"),
            35,
            "5 of its own plus 30 in parse"
        );
        assert_eq!(profiler.total_cost("save"), 5);
        assert_eq!(profiler.total_cost("parse"), 31);
        assert_eq!(profiler.total_cost("never_entered"), 0);
    }

    #[test]
    fn a_calls_are_counted_per_name() {
        let profiler = sample_profile();
        assert_eq!(profiler.calls("main"), 1);
        assert_eq!(profiler.calls("parse"), 11);
        assert_eq!(profiler.calls("never_entered"), 0);
    }

    #[test]
    fn a_folded_stacks_have_one_sorted_line_per_path() {
        let profiler = sample_profile();
        assert_eq!(
            profiler.folded(),
            "main 2\nmain;load 5\nmain;load;parse 30\nmain;save 4\nmain;save;parse 1\n"
        );
    }

    #[test]
    fn a_paths_without_self_cost_are_left_out() {
        let profiler = Profiler::new();
        {
            let _outer = profiler.enter("outer");
            let _inner = profiler.enter("inner");
            profiler.charge(9);
        }
        assert_eq!(profiler.folded(), "outer;inner 9\n");
        assert_eq!(profiler.total_cost("outer"), 9);
        assert_eq!(Profiler::new().folded(), "");
    }

    #[test]
    fn a_cost_outside_every_span_is_not_lost() {
        let profiler = Profiler::new();
        profiler.charge(4);
        {
            let _work = profiler.enter("work");
            profiler.charge(1);
        }
        profiler.charge(2);
        assert_eq!(profiler.folded(), "(no span) 6\nwork 1\n");
    }

    #[test]
    fn a_recursion_is_counted_once_in_the_total() {
        let profiler = Profiler::new();
        fn walk(profiler: &Profiler, depth: u32) {
            let _span = profiler.enter("walk");
            profiler.charge(1);
            if depth > 0 {
                walk(profiler, depth - 1);
            }
        }
        walk(&profiler, 3);
        assert_eq!(profiler.calls("walk"), 4);
        assert_eq!(profiler.self_cost("walk"), 4);
        assert_eq!(
            profiler.total_cost("walk"),
            4,
            "4 units were charged while `walk` was open; nested `walk` frames must not count them again"
        );
        assert_eq!(
            profiler.folded(),
            "walk 1\nwalk;walk 1\nwalk;walk;walk 1\nwalk;walk;walk;walk 1\n"
        );
    }

    #[test]
    fn a_span_closes_on_every_way_out() {
        fn checked(profiler: &Profiler, input: &str) -> Result<u32, std::num::ParseIntError> {
            let _span = profiler.enter("checked");
            profiler.charge(1);
            let number: u32 = input.parse()?; // may leave early
            profiler.charge(1);
            Ok(number)
        }
        let profiler = Profiler::new();
        let _main = profiler.enter("main");
        assert!(checked(&profiler, "x").is_err());
        profiler.charge(10);
        assert_eq!(checked(&profiler, "5"), Ok(5));
        profiler.charge(10);
        assert_eq!(
            profiler.self_cost("main"),
            20,
            "after `checked` returned (early or not), `main` must be the current frame again"
        );
        assert_eq!(profiler.self_cost("checked"), 3);
        assert_eq!(profiler.folded(), "main 20\nmain;checked 3\n");
    }

    #[test]
    fn a_hot_paths_are_ordered_by_self_cost() {
        let profiler = sample_profile();
        assert_eq!(
            profiler.hot_paths(2),
            [
                ("main;load;parse".to_owned(), 30),
                ("main;load".to_owned(), 5)
            ]
        );
        assert_eq!(profiler.hot_paths(100).len(), 5);
        assert_eq!(profiler.hot_paths(0), []);

        let ties = Profiler::new();
        for name in ["b", "c", "a"] {
            let _span = ties.enter(name);
            ties.charge(7);
        }
        assert_eq!(
            ties.hot_paths(3),
            [
                ("a".to_owned(), 7),
                ("b".to_owned(), 7),
                ("c".to_owned(), 7)
            ],
            "paths with equal cost are ordered by path"
        );
    }

    #[test]
    fn a_the_profiler_stays_out_of_the_way() {
        let profiler = Profiler::new();
        let _main = profiler.enter("main");
        {
            let _first = profiler.enter("step"); // the path main;step exists from here on
        }
        let (_, allocations) = count(|| {
            for _ in 0..1_000 {
                profiler.charge(1);
            }
        });
        assert_eq!(
            allocations, 0,
            "charge allocated {allocations} time(s) in 1,000 calls; it runs in the hottest \
             loops, so it must find the current frame without building anything"
        );
        let (_, allocations) = count(|| {
            for _ in 0..1_000 {
                let _span = profiler.enter("step");
                profiler.charge(1);
            }
        });
        assert_eq!(
            allocations, 0,
            "entering a known call path allocated {allocations} time(s) in 1,000 calls; \
             look the path up without building a string for it"
        );
        assert_eq!(profiler.self_cost("step"), 1_000);
        assert_eq!(profiler.calls("step"), 1_001);
    }

    #[test]
    fn b_show_the_profile_of_the_slow_version() {
        profiler_works();
        let profiler = Profiler::new();
        let ids = sample_ids(2_000, 500);
        let keys: Vec<Key> = ids.iter().map(|&id| Key::new(id, &profiler)).collect();
        let distinct = distinct_sorted_slow(&keys, &profiler);
        let expected: std::collections::BTreeSet<u32> = ids.iter().copied().collect();
        assert!(
            distinct
                .iter()
                .map(|key| key.id)
                .eq(expected.iter().copied())
        );
        println!("--- folded stacks of distinct_sorted_slow, 2,000 keys ---");
        print!("{}", profiler.folded());
        println!(
            "--- total: {} units ---",
            profiler.total_cost("distinct_sorted_slow")
        );
        let parts: u64 = ["search", "insert", "keep_sorted"]
            .iter()
            .map(|name| profiler.self_cost(name))
            .sum();
        assert_eq!(
            profiler.total_cost("distinct_sorted_slow"),
            parts,
            "the total cost of the outer frame is the sum of the self costs below it"
        );
        assert_eq!(profiler.calls("keep_sorted"), 2_000);
    }

    #[test]
    fn b_same_result_as_the_slow_version() {
        profiler_works();
        let profiler = Profiler::new();
        for (n, distinct) in [(0, 1), (1, 1), (2, 1), (50, 7), (300, 300), (400, 3)] {
            let keys: Vec<Key> = sample_ids(n, distinct)
                .into_iter()
                .map(|id| Key::new(id, &profiler))
                .collect();
            let expected: Vec<u32> = distinct_sorted_slow(&keys, &profiler)
                .iter()
                .map(|key| key.id)
                .collect();
            let actual: Vec<u32> = distinct_sorted(&keys, &profiler)
                .iter()
                .map(|key| key.id)
                .collect();
            assert_eq!(
                actual, expected,
                "{n} keys with up to {distinct} distinct ids"
            );
        }
    }

    #[test]
    fn b_meets_the_work_budget() {
        profiler_works();
        for (n, distinct) in [(2_000usize, 500u64), (2_000, 2_000), (4_096, 16)] {
            let profiler = Profiler::new();
            let keys: Vec<Key> = sample_ids(n, distinct)
                .into_iter()
                .map(|id| Key::new(id, &profiler))
                .collect();
            let result = {
                let _run = profiler.enter("budget_run");
                distinct_sorted(&keys, &profiler)
            };
            assert!(result.len() <= distinct as usize);
            let spent = profiler.total_cost("budget_run");
            let log2_up = u64::from(n.next_power_of_two().trailing_zeros());
            let budget = n as u64 * (2 * log2_up + 3);
            assert!(
                spent <= budget,
                "distinct_sorted spent {spent} units (comparisons plus clones) on {n} keys; the \
                 budget is {budget}. Sort once instead of after every key, and do not search \
                 the whole result for every key"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. For each of 100 different values, search the list of values
        // seen so far from the front, and append the value if it is absent.
        // How many `==` comparisons are made in total?
        let compared = Cell::new(0u64);
        let mut seen: Vec<u32> = Vec::new();
        for value in 0..100u32 {
            let known = seen.iter().any(|old| {
                compared.set(compared.get() + 1);
                *old == value
            });
            if !known {
                seen.push(value);
            }
        }
        let p1: u64 = 4950;
        assert_eq!(
            p1,
            compared.get(),
            "P1: comparisons made by 100 linear searches (left: your prediction, right: actual)"
        );

        // P2. `main` does 2 units of work itself and calls `load` once.
        // `load` does 5 units itself and calls `parse` 10 times. Every call
        // of `parse` does 3 units and calls nothing.
        let parse_total = 10 * 3;
        let load_total = 5 + parse_total;
        let main_total = 2 + load_total;
        // (self cost of load, total cost of load, total cost of main)
        let p2: (u64, u64, u64) = (5, 35, 37);
        assert_eq!(
            p2,
            (5, load_total, main_total),
            "P2: (self of load, total of load, total of main) (left: your prediction, right: actual)"
        );

        // P3. A profile of a run: 60% parse, 30% sort, 10% write. You make
        // `sort` 10 times faster and change nothing else. Then you imagine a
        // `sort` that takes no time at all. What is the run time in each
        // case, in percent of the old run time?
        let shares = [60u32, 30, 10];
        let ten_times = shares[0] + shares[1] / 10 + shares[2];
        let free = shares[0] + shares[2];
        let p3: (u32, u32) = (73, 70);
        assert_eq!(
            p3,
            (ten_times, free),
            "P3: new run time in percent with (sort 10 times faster, sort for free) (left: your prediction, right: actual)"
        );
    }
}
