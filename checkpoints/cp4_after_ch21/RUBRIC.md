# Checkpoint 4 rubric: `seal`

[All checkpoints](../README.md) · [Specification](SPEC.md)

Use this file after your sitting has ended. It tells you which level you
reached and, for everything that did not work, which lesson to review.

There are two levels. Every criterion can be checked by running a command
or by reading your own code. Nothing depends on opinion.

## Level 1: pass

You pass when all of these are true.

- [ ] You worked closed-notes, in one sitting of at most 4 hours (see
      [the checkpoint rules](../README.md#rules)).
- [ ] `cargo test --test behaviour` passes.
- [ ] `cargo test --release --test behaviour` passes.
- [ ] You ran each of the two commands three times, and they passed every
      time. A test of threads that passes only sometimes has not passed.
- [ ] The files in `tests/` are unchanged, and `Cargo.toml` has no
      dependencies.
- [ ] `grep -rn "unsafe" src/` finds nothing.

A pass means: you can read and write text and bytes through the standard
I/O traits, spread work over threads and collect the results, and write a
future, a waker and a small executor yourself, without help.

## Level 2: proficient

You are proficient when you pass and all of these are true as well.

- [ ] `cargo test` and `cargo test --release` pass completely, three times
      each. This adds `tests/technique.rs`.
- [ ] You deleted the `#![allow(dead_code, unused_variables)]` line at the
      top of `src/lib.rs`, and `cargo clippy --all-targets` prints no
      warning. You added no other `allow` attribute.
- [ ] **Nothing waits by checking again.** `grep -nE "sleep|yield_now|spin_loop|try_recv|try_lock|is_finished" src/lib.rs`
      finds nothing outside your tests and comments.
- [ ] **Every lock is held briefly.** For each `lock()` in your code you
      can point to the line where the guard is dropped, and no call of a
      waker, of `work`, of `open` or of a reader happens while a guard is
      alive.
- [ ] **Every atomic ordering has a reason.** Each `Ordering::` in your
      code has a comment that says why this ordering is enough. "SeqCst to
      be safe" does not count as a reason.
- [ ] **Shared state is described.** The type that the background thread
      and the future share has a comment that lists its states and says
      who moves it from one state to the next.
- [ ] **One buffer, used again.** `digest` and `read_manifest` each create
      their buffer before the loop, not inside it.
- [ ] `grep -nE "unwrap\(\)|expect\(|panic!|\[[a-z_.]*\.\.[a-z_.]*\]|\[[a-z_0-9]+\]" src/lib.rs`:
      for every hit outside your tests you can say, in one sentence, why it
      cannot fail for any input. This includes every `unwrap()` on a
      `lock()` and on a `join()`. If you cannot, it counts as not met.
- [ ] `src/lib.rs` has at least five tests of your own. At least one of
      them tests the text format with a name you invented, and at least
      one runs threads.

Proficient means: your code is correct under every schedule of the
threads, and you can say why. It streams where it could have collected,
it sleeps where it could have spun, and it has no place where a wake-up
can fall between two steps.

## If a test failed

Find the failed test in the first column. The names of related tests start
with the same word. "Chapter 21 · 04" means lesson 04 of chapter 21; read
its section in that chapter's `CONCEPTS.md` again, then repeat the lesson
and its drills.

### Behaviour tests

| failed tests | what they check | review |
|---|---|---|
| `digest_of_known_inputs`, `digest_of_a_megabyte`, `digest_takes_any_reader` | a loop that reads from any `Read` into a buffer; wrapping arithmetic | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 01 streaming bytes; [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy |
| `digest_accepts_short_reads`, `digest_retries_interrupted_reads`, `digest_returns_other_errors` | the contract of `read`: fewer bytes than asked for is normal, `Ok(0)` is the end, `Interrupted` means "again" | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 01 streaming bytes |
| `display_writes_digest_two_spaces_and_name` | width, fill and radix in a format string | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 06 the formatting language |
| `display_keeps_ordinary_characters`, `display_escapes_backslash_and_line_endings` | `Display` for your own type; replacing single characters | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 07 `Display` and `fmt::Write`, · 03 search patterns and replacement |
| `parse_a_line`, `parse_accepts_upper_case_digits`, `parse_takes_the_rest_of_the_line_as_the_name` | `FromStr`; cutting a string at a known position | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 05 `FromStr` and textual conversion, · 03 byte positions |
| `parse_digest_errors`, `parse_separator_errors` | a byte offset can be inside a character; `from_str_radix` accepts more than digits | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 01 Unicode scalar values and encoding, · 03 byte positions, · 05 textual conversion |
| `parse_reads_escapes`, `parse_name_and_escape_errors`, `parse_the_first_check_that_fails_decides`, `parse_reverses_display` | walking through a string character by character with its byte offsets | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 04 text iteration, · 02 constructing owned text |
| `entry_error_messages`, `manifest_error_messages_and_sources`, `manifest_error_converts_from_io_error` | error types with `Display`, `source` and `From` | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 05 multiple error types, · 06 custom errors with a source chain |
| `read_a_manifest`, `read_accepts_any_buffered_reader`, `read_reports_the_line_number`, `read_reports_io_errors` | reading lines from any `BufRead` | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 02 buffered text |
| `read_handles_windows_line_endings`, `read_skips_empty_lines_only` | what exactly belongs to a line ending; `trim` removes more than that | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 02 buffered text; [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 04 trimming |
| `write_a_manifest`, `write_then_read_gives_the_entries_back`, `write_returns_errors` | writing formatted text to any `Write` | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 03 writers and flushing |
| `write_repeats_short_writes` | `write` may take only a part; `write_all` and `write!` repeat; who flushes | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 03 writers and flushing |
| `check_all_reports_in_entry_order`, `check_all_keeps_entry_order_when_files_finish_out_of_order`, `check_all_without_entries`, `check_all_opens_every_entry_once`, `check_all_reports_open_and_read_errors` | threads that borrow, take work from a shared place and hand results back | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 01 fork and join, · 02 borrowed parallel work, · 07 atomic counters |
| `check_all_takes_readers_that_cannot_change_threads` | which values cross a thread boundary and which stay on one thread | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 04 `Send` and `Sync` contracts |
| `check_dir_reads_files` | building a path and opening a file | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 05 paths |
| `block_on_returns_the_output`, `block_on_runs_async_functions_that_await`, `block_on_takes_futures_that_borrow_and_are_not_send` | pinning a future on the stack and polling it with a context | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 04 a parker `block_on`; [Chapter 20](../../exercises/chapter20/CONCEPTS.md) · 01 lazy futures, · 02 borrowed and owned futures |
| `block_on_polls_again_after_a_wake_from_inside_poll`, `block_on_sleeps_until_another_thread_wakes_it`, `block_on_wakers_may_be_used_afterwards` | a waker made from `Arc<impl Wake>`; what `Pending` promises | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 02 pending and wakeups, · 04 a parker `block_on` |
| `in_background_gives_the_result_of_the_work`, `in_background_runs_on_another_thread`, `in_background_starts_without_being_polled`, `in_background_returns_while_the_work_is_still_running`, `in_background_jobs_can_be_awaited_in_any_order` | `Future` implemented by hand, completed by another thread | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 01 implement `Future`, · 03 a completion cell |
| `in_background_turns_a_panic_into_an_error` | a thread that panics must still tell whoever waits for it | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 01 worker failures; [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 07 deliberate recovery |
| `digest_all_keeps_the_order_of_the_readers`, `digest_all_without_readers`, `digest_all_reports_errors_and_panics_in_place` | an `async fn` that awaits several futures | [Chapter 20](../../exercises/chapter20/CONCEPTS.md) · 01 lazy futures and `await`, · 02 owned futures |
| a test reports "was still running after 20 s" | a thread waits for something that never comes: a lock that is still held, a channel whose sender is alive, a wake-up that was lost | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 03 ownership and channel closure, · 05 mutex invariants; [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 02 pending and wakeups |

### Technique tests

| failed tests | what they check | review |
|---|---|---|
| `digest_needs_a_fixed_amount_of_memory`, `digest_reads_large_pieces` | streaming with one buffer of a few kilobytes: not `read_to_end`, not one byte per call | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 01 streaming bytes |
| `display_does_not_allocate`, `write_manifest_does_not_allocate` | writing pieces to a formatter or a writer instead of building a `String` first | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 07 `Display`, `fmt::Write` and `format_args!`; [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 03 writers |
| `parse_allocates_only_the_name` | one `String` with the right capacity; checks that need no allocation come first | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 02 constructing owned text, · 08 delayed allocation |
| `read_manifest_stops_at_the_first_failure` | reading line by line, not the whole input first | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 02 buffered text |
| `check_all_reads_with_all_workers_at_the_same_time` | the work really runs on several threads | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 01 fork and join, · 02 borrowed parallel work |
| `check_all_never_has_more_readers_than_workers` | a fixed number of workers, not one thread per item | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 08 capstone: bounded parallel pipeline |
| `check_all_a_slow_file_does_not_keep_the_others_waiting` | workers take the next item from a shared counter or queue; the items are not divided in advance | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 07 atomic counters, · 03 channels |
| `block_on_sleeps_between_polls` | `thread::park` (or a condition variable) instead of polling in a loop | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 04 a parker `block_on` |
| `block_on_does_not_lose_a_wake_that_comes_before_it_sleeps` | a wake-up is remembered in a flag or a token until the sleeper looks | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 04 a parker `block_on`; [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 06 condition predicates |
| `background_poll_does_not_wait_for_the_work` | `poll` returns at once; it never joins, receives or sleeps | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 01 implement `Future`, · 02 pending and wakeups |
| `background_wakes_the_most_recent_waker`, `background_gives_a_finished_result_at_the_next_poll` | every `Pending` poll stores the waker of that poll; a poll after the end returns the result | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 02 pending and wakeups, · 03 a completion cell |
| `background_keeps_a_result_that_arrives_during_poll`, `background_never_loses_a_wake_up` | "is the result there?" and "store the waker" happen under one lock; the result is stored before the waker is used | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 03 a completion cell; [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 05 mutex invariants |
| `background_work_goes_on_when_the_future_is_dropped` | dropping a future cancels the waiting, not the thread; shared ownership frees the result | [Chapter 20](../../exercises/chapter20/CONCEPTS.md) · 05 cancellation by dropping; [Chapter 4](../../exercises/chapter04/CONCEPTS.md) · 07 `Rc` and `Arc` |
| `digest_all_reads_all_readers_at_the_same_time` | start all the work first, then await; `await` in a loop runs one after the other | [Chapter 20](../../exercises/chapter20/CONCEPTS.md) · 01 lazy futures and `await` |
| the test program stops with "MEMORY LIMIT" | a loop that never ends and stores something each time | [Chapter 18](../../exercises/chapter18/CONCEPTS.md) · 01 streaming bytes |

### A test fails only sometimes

That is the mark of a race: two threads do their steps in an order that
you did not plan for. Do not run the test again until it passes. Find the
two steps instead. The usual places:

- the background thread stores the result and uses the waker in two steps,
  and `poll` runs between them;
- `poll` looks for the result, lets go of the lock, and stores the waker
  afterwards;
- `block_on` decides to sleep after the waker has already been used.

Review [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 02 and · 03,
and [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 05 and · 09.

## If you ran out of time

| what happened | what it suggests | review |
|---|---|---|
| Most time went into the escapes and the byte offsets of `FromStr` | walking through text by hand needs practice | [Chapter 17](../../exercises/chapter17/CONCEPTS.md) · 03 and · 04, then the chapter's capstone from an empty file |
| Most time went into compiler errors in `check_all`: lifetimes of threads, `Send`, `Sync`, "closure may outlive" | scoped threads and the thread-safety traits are not yet familiar | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 02 and · 04 |
| `check_all` was written quickly, and then tests hung | you wrote the threads before you had decided who owns what and who waits for whom; draw that first | [Chapter 19](../../exercises/chapter19/CONCEPTS.md) · 03 and · 08 capstone |
| Most time went into `Pin`, `Context` and `Waker` signatures | the parts of `Future::poll` are not yet familiar | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 01 and · 04, each from an empty file |
| `Background` worked in most runs and hung in some | the shared state has a gap between two steps | [Chapter 21](../../exercises/chapter21/CONCEPTS.md) · 03 |
| You did not reach sections 6 to 8 | the first half took too long; time yourself on chapters 17 and 18 before the next attempt | the capstones of [Chapter 17](../../exercises/chapter17/CONCEPTS.md) and [Chapter 18](../../exercises/chapter18/CONCEPTS.md) |

## What to do with the result

- **Not passed:** review the lessons from the tables. Wait at least one
  week, then take the checkpoint again from the empty starter. Do not look
  at the reference solution in between.
- **Passed, not proficient:** read the reference solution and its
  commentary in `solutions/checkpoints/cp4_after_ch21/` of the course
  repository. For each technique test or rubric line that failed, change
  your own code until it holds. Then go on to chapter 22.
- **Proficient:** read the commentary anyway and compare the decisions
  with yours. Then go on to chapter 22.
