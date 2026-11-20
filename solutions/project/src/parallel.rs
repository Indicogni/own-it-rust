//! Milestone m19 (Chapter 19): many files on worker threads.
//!
//! The contract that makes parallelism safe to adopt: **the results are
//! exactly the sequential results**. Workers may finish in any order; the
//! code puts results back in input order and merges them in that order.

use crate::error::Error;
use crate::filter::Filter;
use crate::pipeline::Tally;
use crate::pipeline::process_reader;
use crate::report::{Group, Summary};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

/// m19 A. `items.iter().map(f).collect()`, computed by up to `jobs` threads.
///
/// - `jobs == 0` means `std::thread::available_parallelism()` (1 if unknown).
/// - Use at most `min(jobs, items.len())` workers. With one worker (or no
///   items), run everything on the calling thread and spawn nothing.
/// - Otherwise use `std::thread::scope`, so workers can borrow `items` and
///   `f`. Hand out work dynamically: a worker takes the next unclaimed item
///   when it finishes one (an `AtomicUsize` index, or a channel of indexes).
///   Send `(index, result)` pairs back over an `mpsc` channel and put each
///   result in its slot.
/// - If `f` panics, the panic propagates to the caller (the scope does this).
pub fn map_ordered<T, R, F>(items: &[T], jobs: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let jobs = if jobs == 0 {
        thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        jobs
    };
    let workers = jobs.min(items.len());
    if workers <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<R>> = items.iter().map(|_| None).collect();
    thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        for _ in 0..workers {
            let tx = tx.clone();
            let (next, f) = (&next, &f);
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    if tx.send((i, f(item))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (i, result) in rx {
            slots[i] = Some(result);
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.expect("every item was processed"))
        .collect()
}

/// m19 B. Filter and summarize several files with up to `jobs` threads.
///
/// Each file is read with [`crate::pipeline::process_reader`] into its own
/// [`Summary`]; malformed lines are only counted. Merge the per-file tallies
/// and summaries in path order. If a file cannot be opened, return
/// [`Error::Open`] for the *first* such path in path order (the other files
/// may still have been read). The result is the same for every `jobs`.
pub fn summarize_files<P>(
    paths: &[P],
    filter: &Filter,
    group: &Group,
    jobs: usize,
) -> Result<(Tally, Summary), Error>
where
    P: AsRef<Path> + Sync,
{
    let results = map_ordered(paths, jobs, |path| {
        let path = path.as_ref();
        let file = File::open(path).map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let mut summary = Summary::new(group.clone());
        let tally = process_reader(BufReader::new(file), filter, &mut summary, &mut |_| {})?;
        Ok::<_, Error>((tally, summary))
    });
    let mut total = (Tally::default(), Summary::new(group.clone()));
    for result in results {
        let (tally, summary) = result?;
        total.0 += tally;
        total.1 += summary;
    }
    Ok(total)
}
