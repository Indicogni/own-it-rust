//! LESSON 08 — Capstone: owned dispatch queue
//! Run: cargo test --test chapter04 l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: In a batch of jobs that are edited independently,
//! which objects should be moved, which cloned, and which shared? Write your
//! answer in NOTES.md before you write code.
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Draw every allocation and every owner after `dispatch`.
//! Explain why sharing a configuration that nobody changes does not mean that
//! the job labels, which are changed, should be shared too.
//!
//! COMPILER / TRANSFER LAB (optional): Change the design so that jobs are handed
//! out one at a time with `Option::take`. Then decide whether `Rc` could be
//! replaced by `Arc` without changing who owns what, and what it would cost.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

use std::rc::Rc;
/// A. Consume labels. Return (label, shared_config) jobs in original order.
/// All jobs must point at ONE allocation containing config. Move `config`
/// into that allocation and move each label into its job: no clones of text.
fn dispatch(labels: Vec<String>, config: String) -> Vec<(String, Rc<String>)> {
    todo!("Implement this task")
}
/// B. Consume one job and return its label with "@" and the config text
/// appended, as "label@config". Append to the label you were given (its
/// buffer may grow); do not build a new String and do not clone either input.
fn finish(job: (String, Rc<String>)) -> String {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_dispatch_moves_labels_and_shares_config() {
        let label = String::from("one");
        let label_text = label.as_ptr();
        let config = String::from("zone");
        let config_text = config.as_ptr();
        let jobs = dispatch(vec![label, "two".into()], config);
        assert_eq!(
            jobs[0].0.as_ptr(),
            label_text,
            "the first job must own the original label (same heap buffer): move the \
             labels, do not clone them"
        );
        assert!(
            Rc::ptr_eq(&jobs[0].1, &jobs[1].1),
            "every job must point at the same Rc allocation: make one Rc and clone the pointer"
        );
        assert_eq!(
            jobs[0].1.as_str().as_ptr(),
            config_text,
            "the shared config must be the String that was passed in (moved into the \
             Rc), not a copy of its text"
        );
        assert_eq!(
            Rc::strong_count(&jobs[0].1),
            2,
            "two jobs, so the config must have exactly two owners after dispatch returns"
        );
        assert_eq!(jobs[1].0, "two");
        assert!(dispatch(vec![], "unused".into()).is_empty());
    }
    #[test]
    fn b_finish_consumes_a_job() {
        let mut jobs = dispatch(vec!["one".into(), "two".into()], "cfg".into());
        assert_eq!(finish(jobs.remove(0)), "one@cfg");
        assert_eq!(
            Rc::strong_count(&jobs[0].1),
            1,
            "finishing a job must drop that job's Rc, so one owner of the config is left"
        );
        assert_eq!(finish((String::new(), Rc::new(String::new()))), "@");
    }

    #[test]
    fn a_dispatch_never_clones_labels_or_config() {
        let labels: Vec<String> = (0..200).map(|i| i.to_string()).collect();
        let (jobs, allocations) = count(|| dispatch(labels, "zone".into()));
        assert_eq!(jobs.len(), 200);
        assert!(
            allocations < 20,
            "dispatch made {allocations} heap allocations for 200 labels; moving them needs \
             only the new vector and one Rc, so labels or the config were cloned"
        );
    }

    #[test]
    fn b_finish_reuses_the_label_buffer() {
        let mut label = String::with_capacity(32);
        label.push_str("one");
        let label_text = label.as_ptr();
        let job = (label, Rc::new(String::from("cfg")));
        let (out, allocations) = count(|| finish(job));
        assert_eq!(out, "one@cfg");
        assert_eq!(
            out.as_ptr(),
            label_text,
            "finish must return the label it was given with text appended (same heap buffer), \
             not a newly built String"
        );
        assert_eq!(
            allocations, 0,
            "finish made {allocations} heap allocation(s); this label already has room for \
             \"@cfg\", so appending needs none"
        );
    }
}
