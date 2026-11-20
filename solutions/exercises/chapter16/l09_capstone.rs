//! LESSON 09 — Capstone: a task scheduler and audit
//! Run: cargo test --test chapter16 l09_
//! Read first: "Lesson 09" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: Choose a container for pending priorities, uniqueness, and
//! completion order before coding. Predict duplicate tasks and equal-priority ties.
//! For task C, write down every field you plan to give the Scheduler and the one
//! operation that each field makes cheap.
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: What complexity do heap insertion/pop and hash lookup suggest? Which
//! output order must be made explicit? What does a cancelled task cost in your design?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Write a design note for an indexed collection, graph, persistent map, and small-vector
//! workload. Identify when a standard collection suffices and what measurements would
//! justify an external collection crate.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::ops::RangeInclusive;
/// A. For duplicate names keep only the highest priority.
fn merge(tasks: &[(&str, u8)]) -> HashMap<String, u8> {
    let mut best: HashMap<String, u8> = HashMap::new();
    for &(name, priority) in tasks {
        // `entry` would need an owned String for every task. Looking up with
        // the borrowed name first allocates only for names that are new.
        match best.get_mut(name) {
            Some(current) => *current = (*current).max(priority),
            None => {
                best.insert(name.to_string(), priority);
            }
        }
    }
    best
}
/// B. Execute at most limit unique names: highest priority first, then ascending
/// lexical name for ties. Return owned names; zero limit returns empty.
fn schedule(tasks: &[(&str, u8)], limit: usize) -> Vec<String> {
    // Tuples compare field by field: the priority decides first, and
    // `Reverse` makes the smaller name the greater entry among equals.
    let mut heap: BinaryHeap<(u8, Reverse<String>)> = merge(tasks)
        .into_iter()
        .map(|(name, priority)| (priority, Reverse(name)))
        .collect();
    std::iter::from_fn(|| heap.pop())
        .take(limit)
        .map(|(_, Reverse(name))| name)
        .collect()
}
/// C. A scheduler that stays alive while tasks arrive, are cancelled and run,
/// in any order. A and B work on one finished list; here the same rules must
/// hold after every single call.
///
/// The design is yours: replace the comment in the struct with your fields.
/// No operation may rebuild a whole collection: `submit`, `cancel` and
/// `run_next` should each cost a few lookups or O(log n) steps, also when
/// thousands of tasks are pending.
struct Scheduler {
    // Design: a heap with lazy deletion. The map is the truth; the heap only
    // gives the order and may hold outdated entries, which `run_next` skips.
    // This makes `submit` and `cancel` cheap, at the price of memory for
    // outdated entries until they reach the top. A
    // `BTreeSet<(Reverse<u8>, String)>` in place of the heap would allow
    // removing an entry directly, at the price of more work in `submit`.
    /// Every pending task with its current priority.
    pending: HashMap<String, u8>,
    /// Run order: highest priority first, then the smallest name.
    queue: BinaryHeap<(u8, Reverse<String>)>,
    /// The names of the latest runs, oldest at the front.
    history: VecDeque<String>,
    history_limit: usize,
    /// Number of runs for each priority; ordered keys give range queries.
    runs: BTreeMap<u8, usize>,
}
impl Scheduler {
    /// An empty scheduler that remembers the names of the last `history_limit` runs.
    fn new(history_limit: usize) -> Self {
        Self {
            pending: HashMap::new(),
            queue: BinaryHeap::new(),
            history: VecDeque::new(),
            history_limit,
            runs: BTreeMap::new(),
        }
    }
    /// Make `name` pending with this priority. If `name` is already pending,
    /// it stays one task and keeps the higher of the two priorities.
    fn submit(&mut self, name: &str, priority: u8) {
        match self.pending.get_mut(name) {
            Some(current) if *current >= priority => return,
            Some(current) => *current = priority,
            None => {
                self.pending.insert(name.to_string(), priority);
            }
        }
        // A raised task gets a second queue entry. The entry with the old
        // priority stays behind as an outdated one.
        self.queue.push((priority, Reverse(name.to_string())));
    }
    /// Withdraw a pending task so that it never runs. Return true if `name`
    /// was pending. A withdrawn task may be submitted again later; its old
    /// priority is then forgotten.
    fn cancel(&mut self, name: &str) -> bool {
        // Only the map changes. The queue entry becomes outdated.
        self.pending.remove(name).is_some()
    }
    /// The number of pending tasks. Each name counts once.
    fn pending(&self) -> usize {
        self.pending.len()
    }
    /// Run one task: the pending task with the highest priority, and among
    /// equal priorities the one with the smallest name. It stops being
    /// pending (the same name may be submitted again and run again), and the
    /// run is recorded for `recent` and `audit`. None when nothing is pending.
    fn run_next(&mut self) -> Option<String> {
        while let Some((priority, Reverse(name))) = self.queue.pop() {
            // An entry is current only if the map still holds this name with
            // exactly this priority. Anything else was cancelled, raised, or
            // has already run.
            if self.pending.get(&name) != Some(&priority) {
                continue;
            }
            self.pending.remove(&name);
            *self.runs.entry(priority).or_insert(0) += 1;
            if self.history_limit > 0 {
                if self.history.len() == self.history_limit {
                    self.history.pop_front();
                }
                self.history.push_back(name.clone());
            }
            return Some(name);
        }
        None
    }
    /// The names of the most recent runs, oldest first: at most
    /// `history_limit` of them.
    fn recent(&self) -> Vec<&str> {
        self.history.iter().map(String::as_str).collect()
    }
    /// For each priority in `priorities` at which at least one task has run:
    /// (priority, number of runs), in ascending priority order.
    /// Callers guarantee start <= end.
    fn audit(&self, priorities: RangeInclusive<u8>) -> Vec<(u8, usize)> {
        self.runs
            .range(priorities)
            .map(|(&priority, &runs)| (priority, runs))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    #[test]
    fn a_merging_keeps_highest_priority_and_all_names() {
        assert_eq!(
            merge(&[("a", 2), ("a", 7), ("b", 0), ("a", 3)]),
            HashMap::from([("a".into(), 7), ("b".into(), 0)])
        );
        assert_eq!(
            merge(&[("x", 255), ("x", 0), ("é", 1)]),
            HashMap::from([("x".into(), 255), ("é".into(), 1)])
        );
        assert!(merge(&[]).is_empty());
    }
    #[test]
    fn b_scheduler_has_deterministic_ties_and_limits() {
        // `within` fails the test with a message if a loop never ends, for
        // example one that waits for `limit` names when fewer tasks exist.
        within(10, "the loop in schedule", || {
            let tasks = [("z", 3), ("a", 3), ("low", 1), ("z", 8), ("b", 3)];
            assert_eq!(schedule(&tasks, 9), ["z", "a", "b", "low"]);
            assert_eq!(schedule(&tasks, 2), ["z", "a"]);
            assert!(schedule(&tasks, 0).is_empty());
            assert!(schedule(&[], 5).is_empty());
        });
    }
    #[test]
    fn b_duplicates_run_once_at_their_highest_priority() {
        within(10, "the loop in schedule", || {
            let tasks = [("b", 1), ("a", 1), ("b", 5), ("c", 5), ("a", 0)];
            assert_eq!(schedule(&tasks, 3), ["b", "c", "a"]);
            assert_eq!(schedule(&tasks, 1), ["b"]);
            let ties = [("B", 2), ("a", 2), ("A", 2)];
            assert_eq!(schedule(&ties, 3), ["A", "B", "a"], "byte-wise name order");
        });
    }

    /// Runs tasks until none is pending and returns their names in run order.
    fn run_all(scheduler: &mut Scheduler) -> Vec<String> {
        std::iter::from_fn(|| scheduler.run_next()).collect()
    }
    #[test]
    fn c_runs_by_priority_then_name() {
        // `within` fails the test with a message if `run_next` never returns.
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(8);
            assert_eq!(s.run_next(), None, "nothing is pending in a new scheduler");
            for (name, priority) in [("z", 3), ("a", 3), ("low", 1), ("z", 8), ("b", 3)] {
                s.submit(name, priority);
            }
            assert_eq!(s.pending(), 4, "z was submitted twice but is one task");
            assert_eq!(run_all(&mut s), ["z", "a", "b", "low"]);
            assert_eq!(s.pending(), 0);
            assert_eq!(s.run_next(), None);
        });
    }
    #[test]
    fn c_a_second_submit_keeps_the_higher_priority() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(8);
            s.submit("a", 7);
            s.submit("a", 2);
            s.submit("b", 5);
            s.submit("c", 1);
            s.submit("c", 6);
            assert_eq!(
                run_all(&mut s),
                ["a", "c", "b"],
                "a keeps priority 7, c rises to 6, and each task runs once"
            );
            assert_eq!(
                s.audit(0..=255),
                [(5, 1), (6, 1), (7, 1)],
                "each run is counted at the priority the task had when it ran"
            );
        });
    }
    #[test]
    fn c_cancelled_tasks_never_run() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(8);
            s.submit("a", 7);
            s.submit("b", 5);
            s.submit("c", 1);
            assert!(s.cancel("a"));
            assert!(!s.cancel("a"), "a is no longer pending");
            assert!(!s.cancel("nobody"));
            assert_eq!(s.pending(), 2);
            s.submit("a", 2);
            assert_eq!(s.pending(), 3);
            assert_eq!(
                run_all(&mut s),
                ["b", "a", "c"],
                "a came back with priority 2; its cancelled priority 7 must be forgotten"
            );
            assert_eq!(s.audit(0..=255), [(1, 1), (2, 1), (5, 1)]);
            assert!(!s.cancel("b"), "a task that has run is not pending");
        });
    }
    #[test]
    fn c_cancel_then_submit_again_runs_once() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(8);
            s.submit("a", 5);
            assert!(s.cancel("a"));
            s.submit("a", 5);
            assert_eq!(s.pending(), 1);
            assert_eq!(s.run_next().as_deref(), Some("a"));
            assert_eq!(
                s.run_next(),
                None,
                "a was submitted twice with a cancel in between; only one task was pending"
            );
            assert_eq!(s.recent(), ["a"]);
        });
    }
    #[test]
    fn c_a_finished_task_can_be_submitted_again() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(8);
            s.submit("a", 5);
            assert_eq!(s.run_next().as_deref(), Some("a"));
            s.submit("b", 5);
            s.submit("a", 5);
            assert_eq!(run_all(&mut s), ["a", "b"]);
            assert_eq!(s.recent(), ["a", "a", "b"], "a ran twice");
            assert_eq!(s.audit(5..=5), [(5, 3)]);
        });
    }
    #[test]
    fn c_history_keeps_only_the_latest_runs() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(3);
            for (name, priority) in [("e", 9), ("d", 8), ("c", 7), ("b", 6), ("a", 5)] {
                s.submit(name, priority);
            }
            assert!(s.recent().is_empty(), "nothing has run yet");
            s.run_next();
            s.run_next();
            assert_eq!(s.recent(), ["e", "d"], "oldest run first");
            run_all(&mut s);
            assert_eq!(
                s.recent(),
                ["c", "b", "a"],
                "only the last 3 runs are remembered"
            );
            let mut forgetful = Scheduler::new(0);
            forgetful.submit("x", 1);
            assert_eq!(forgetful.run_next().as_deref(), Some("x"));
            assert!(
                forgetful.recent().is_empty(),
                "a limit of 0 remembers nothing"
            );
            assert_eq!(
                forgetful.audit(0..=255),
                [(1, 1)],
                "the audit does not depend on the history limit"
            );
        });
    }
    #[test]
    fn c_audit_counts_runs_per_priority_in_a_range() {
        within(10, "the Scheduler", || {
            let mut s = Scheduler::new(2);
            let priorities = [9, 9, 4, 4, 4, 0, 255, 7, 7, 1];
            for (i, priority) in priorities.into_iter().enumerate() {
                s.submit(&format!("t{i}"), priority);
            }
            assert!(s.audit(0..=255).is_empty(), "submitting is not running");
            assert!(s.cancel("t3"), "t3 is pending with priority 4");
            assert_eq!(run_all(&mut s).len(), 9);
            assert_eq!(
                s.audit(0..=255),
                [(0, 1), (1, 1), (4, 2), (7, 2), (9, 2), (255, 1)],
                "ascending priority; the cancelled task is not counted"
            );
            assert_eq!(s.audit(4..=7), [(4, 2), (7, 2)], "both ends are included");
            assert!(s.audit(5..=6).is_empty());
            assert_eq!(s.audit(255..=255), [(255, 1)]);
        });
    }
    #[test]
    fn c_run_next_does_not_rebuild_the_queue() {
        let (ran, allocations) = within(20, "the Scheduler", || {
            let mut s = Scheduler::new(0);
            for i in 0..300_u32 {
                s.submit(&format!("task{i:03}"), (i % 5) as u8);
            }
            for i in (0..300).step_by(3) {
                assert!(s.cancel(&format!("task{i:03}")));
            }
            // Allocations are counted per thread, so `count` runs inside `within`.
            count(|| run_all(&mut s).len())
        });
        assert_eq!(ran, 200, "300 submitted, 100 cancelled");
        assert!(
            allocations <= ran + 60,
            "200 runs made {allocations} allocations. One per returned name is fine, \
             but run_next must not build a new vector, map or heap on every call"
        );
    }
}
