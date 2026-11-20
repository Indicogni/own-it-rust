# Rubric for the final assessment

The acceptance tests tell you whether `rgx` does what the SPEC says. This
rubric grades what a test cannot see: the design, the correctness of the
corners no test reaches, how idiomatic the code is, your own tests, the
performance, and the handling of `unsafe`. You apply it to your own work.
Be strict. A grade that flatters you teaches you nothing, and nobody else
reads it.

There are three levels.

- **Pass**: the project is finished. It is correct as far as the tests can
  tell, and nothing in it is careless.
- **Proficient**: what a careful professional would hand in for review.
- **Mastery**: work you could publish and maintain, and whose every
  decision you can defend with a reason or a number.

Every cell is written so that you can check it: by running a command, by
reading your code for a named thing, or by doing a small experiment. If you
cannot show that a cell holds, it does not hold. A level includes the
levels before it.

Fill in the table in section 9 as you go.

## 1. The gate

All of these must be true before any level applies. They are true or
false; there is nothing to judge.

- [ ] `cargo test` passes, three times in a row.
- [ ] `cargo test --release` passes, three times in a row.
- [ ] `cargo build` and `cargo build --release` print no warnings.
- [ ] `cargo clippy --all-targets` prints no warnings, and no lint is
      switched off for a whole crate or file.
- [ ] `[dependencies]` in `Cargo.toml` is empty.
- [ ] The files in `tests/` are unchanged.
- [ ] `cargo run -q -- . src | head -1` ends at once, without the word
      `panicked` on your terminal. (Without `head`: any program that stops
      reading early.)
- [ ] After a full test run no `rgx` process is running and no directory
      named `rgx-test-*` is left in the temporary directory of your system.
- [ ] You did not read the reference solution, and you did not copy code
      from another regular-expression engine.

## 2. Design

| | pass | proficient | mastery |
|---|---|---|---|
| **Stages** | Reading a pattern, building the automaton, running it, and the command line are separate modules. | Each stage has one input type and one output type, and the `use` lines confirm that knowledge flows one way: the parser does not mention the automaton, the matcher does not mention the syntax tree, the program does not mention either. | A second matcher could be added without touching the parser or the program. You have shown it: a second implementation (a slow reference, or a faster engine) runs behind the same internal boundary. |
| **Library and program** | `main.rs` calls the library for all pattern work. | The library does no input or output, starts no threads and prints nothing. The binary holds no matching logic. `main` turns arguments into a request, runs it, and maps the result to an exit status in one place. | The public surface of the library is deliberate: every `pub` item has a doc comment, `#![warn(missing_docs)]` is on, and the case-insensitive mode is offered in a way that can gain a second option later without breaking a caller. |
| **The automaton** | You can draw it for `a(b\|c)*d` and say what one state is in your code. | States refer to each other by index, not by pointer or reference count. The compiled pattern is never written after it is built: no lock, no `Cell`, no `RefCell` inside `Regex`, and `Send + Sync` follows without `unsafe impl`. | The memory that a search writes to is a type of its own, apart from the compiled pattern, so that one thread makes it once and uses it for every haystack. You can state the largest number of states for a pattern of cost `c`, and a test of yours checks it. |
| **The syntax tree** | An enum; one variant per kind of expression. | Values that the SPEC forbids cannot be built: a count with `n > m` or above 1000, or a loop around something that can be empty, never reaches the compiler. The cost of section 6.1 is computed in one function on the tree, before anything is compiled. | A wide pattern is not a deep tree: an alternation of 3000 branches is one node. For **every** recursive function in the crate, including the code that drops the tree, you can name what bounds its depth. |
| **Ordered parallel work** | Several threads search, and the output is in order. | The part that keeps results in order knows nothing about files or patterns. It has tests of its own that use no file. | Results that wait for their turn are bounded: a slow first file cannot make the program hold the results of all the others. When the output closes, workers stop taking new files. A panic in one worker ends the run instead of blocking it. Each of the three is shown by a test. |
| **Errors** | `Error` carries a message and an offset. | The sixteen kinds are an enum, and each message exists in exactly one place. The program keeps usage errors, pattern errors and input failures apart as types, not as strings. | Nothing in the crate compares or parses an error message. The exit status is a function of typed results and is computed in one expression. |

## 3. Correctness that the tests do not reach

Do each experiment. A Rust panic message, a stack overflow, a hang, or
output that differs between two runs is below "pass".

| | pass | proficient | mastery |
|---|---|---|---|
| **The meaning of a match** | All acceptance tests pass. | Your own tests contain every example of SPEC sections 4.2, 4.3 and 4.4. | You wrote the definition of SPEC 4.2 as a second, direct matcher, and a test compares it with your engine on at least ten thousand generated pattern and haystack pairs with a fixed seed. |
| **Hostile patterns** | No pattern you tried made `Regex::new` panic. | A test of yours feeds `Regex::new` at least 100 000 random strings over the metacharacters and a few letters, and asserts that every error offset is a character boundary inside the pattern. | The same test searches with every pattern that compiled and asserts that every reported range is a valid slice of the haystack. |
| **Arithmetic** | The cost tests pass. | Every sum and product of numbers that come from the pattern is checked or saturating, and you can point to each line. No `as` cast in the crate can truncate without a comment that says why it cannot. | You can say which integer type each quantity has (offset, state index, count, cost) and why it is wide enough, with the limit of SPEC section 6 that guarantees it. |
| **The call stack** | The tests of deep and wide patterns pass. | You can list every recursive function and its bound. | Nothing recurses on the length of the haystack, the number of branches, or the length of a concatenation, in the parser, the compiler, the matcher, `Drop`, `Debug` or `Clone`. |
| **Unfriendly file trees** | A directory you may not read (remove your permission and try) gives one `rgx:` line and status 2, and the other files are still searched. | A symbolic link that points to its own parent directory does not make the program loop. An empty file, a file of one line feed, and a file of a megabyte without a line feed all behave as SPEC 7.2 says. | A file name that is not valid Unicode is searched and reported without a panic (try it on Linux). A file larger than your memory is refused with a message, or handled; in neither case does the program crash. |
| **Closing the output** | `cargo run -q -- . src \| head -1` ends without a message. | The same with `-j 8` on a large tree returns in well under a second, because the workers stop. | A write error that is not a closed pipe (on Linux: `rgx . src > /dev/full`) gives an `rgx:` line and status 2. A closed standard error does not make the program panic: after `(sleep 1; rgx x no-such-file; echo $? > status.txt) 2>&1 \| true` the file holds 2, not 101 (`eprintln!` panics when its write fails). |
| **Determinism** | The tests of M6 pass ten times in a row. | The outputs of `-j 1` and `-j 8` over a tree of thousands of files are equal, twenty times in a row, including standard error. | You can explain why they must be equal by pointing at the structure of your program, not at the result of the experiment: which one thread writes, and what it waits for. |

## 4. Idiom

| | pass | proficient | mastery |
|---|---|---|---|
| **Iterators** | `find_iter` and `matching_lines` return something that implements `Iterator`. | They are named types with their lifetimes written out, and they implement `Debug`. The laziness that SPEC 4.4 demands is visible in `next`. | They implement `FusedIterator`, and one implementation of the matching loop serves both an iterator that owns its scratch memory and one that borrows it. |
| **Lifetimes** | The test `the_lines_outlive_the_regex` compiles. | No `'static` bound, no `clone()` and no `to_owned()` exists only to avoid naming a lifetime. | You can say what each lifetime in your public signatures stands for, and which test stops compiling if two of them are merged. |
| **`unwrap`, `expect`, indexing** | None on a value that depends on the pattern, the haystack, the arguments or the file system. | Each remaining one states an invariant in its message, and the invariant is established within the same module. | The library has none outside tests, or each is impossible to trigger by construction and a comment says which type or limit makes it so. |
| **UTF-8** | Offsets are byte offsets on character boundaries. | Characters are decoded with the standard library (`chars`, `char_indices`), never by hand with shifts and masks, unless a measurement justified it and tests cover every sequence length. A file is validated once and is not copied to become a `&str`. | You can state how many times a byte of an input file is copied between the read and the write, and the number is as small as your design allows. |
| **Output** | Output is written through a buffer, not one system call per line. | One writer holds the lock of standard output for the whole run. Every `write` result is handled; nothing uses `println!` or `print!` for results (they panic when the pipe closes). | Workers never touch standard output or standard error. They hand finished bytes to the one thread that writes. |
| **Standard traits** | `Regex: Debug`, `Error: Display + Error`. | `Debug` of a `Regex` is short and readable (the pattern, not ten thousand states). `Error` derives `Clone`, `PartialEq` and `Eq`. | The conversions a user expects exist where they make sense (`FromStr`, `Display` for `Regex`), and none exists that could surprise (no `From<&str>` that panics). |
| **Lints** | Clippy's default set is clean. | No `#[allow]` without a comment that gives the reason. | You ran `cargo clippy -- -W clippy::pedantic` once, read every warning, and fixed or consciously kept each. |

## 5. Tests of your own

| | pass | proficient | mastery |
|---|---|---|---|
| **Unit tests** | Some exist next to the code. | Each module has them: the tree for a handful of patterns, the operations on character sets, the automaton for three small patterns written out state by state, line splitting, argument parsing. | Every example table of SPEC sections 3 to 6 is a test, so that a change in the engine that breaks the SPEC fails in your suite before it fails in the acceptance tests. |
| **Tests of structure** | None. | Tests that see inside: the number of states for a known pattern, the result of merging overlapping ranges, the plan of inputs for a directory. | A test for each property that the acceptance tests can only check indirectly: no allocation in a search after set-up (counted), bounded waiting results in the parallel part, early stop on a closed output. |
| **Generated tests** | None. | One test with generated input and a fixed seed. | The differential test of section 3, and its failures print a pattern and a haystack small enough to debug by hand. |
| **Quality** | The tests are deterministic. | No test sleeps to wait for something, and none asserts on a duration. | Every bug that cost you more than half an hour has a test whose name says what went wrong. |

## 6. Performance

Measure with the release profile:

```text
cargo test --release --test m6_parallel -- --ignored --nocapture speed_report
```

It searches 41 MiB of generated log lines in 64 files with four patterns
and prints the throughput for 1, 2, 4 and 8 threads. Run it a few times on
a machine that is doing nothing else, and take the best run: the numbers
move a lot with other load. For orientation: the reference solution, which
is a plain automaton simulation with no special cases for literals,
reached 70 to 135 MiB/s with one thread on a recent laptop with more than
eight cores. With eight threads it was 5 to 8 times as fast in its best
run and 3 to 7 times in others, depending on the pattern. Judge the
scaling rows only up to the number of cores you have.

| | pass | proficient | mastery |
|---|---|---|---|
| **One thread** | At least 5 MiB/s for each of the four patterns. | At least 25 MiB/s for each. | At least 60 MiB/s for each; or a documented optimisation (a prefilter, a second engine) with numbers before and after on the same machine. |
| **Scaling** | `-j 4` is faster than `-j 1` for every pattern. | For at least three of the four patterns, `-j 4` is at least 2.5 times as fast as `-j 1`. | For at least three of the four patterns, `-j 4` is at least 3 times and `-j 8` at least 5 times as fast, and you can name what limits it beyond that. |
| **The bound** | The tests of M3 pass. | You can state the cost of `is_match`, `find` and one `next` in terms of the pattern's cost and the haystack's length, and show the loop that each factor comes from. `is_match` stops at the first position where the answer is certain. | You measured it: for one worst-case pattern of your choice, doubling the haystack doubles the time, and the numbers for four sizes are written down. |
| **Allocation** | The budget tests of M4 pass. | You counted: after set-up, a search allocates nothing. In the program, the number of allocations per input file does not grow in proportion to the number of its lines. | A test of yours counts the allocations of the program's own path from file content to output bytes, and you know the peak memory of `rgx -c x` on a file of 1 GiB in terms of the file size. |
| **Compiling** | The tests of M3 pass. | You measured the time to compile a pattern at the cost limit (for example `a?` written 50 000 times), and it is below 50 ms in a release build. | You know how many bytes one state takes and therefore how large the biggest compiled pattern is, and you chose the layout with that number in mind. |

## 7. Unsafe hygiene

This project needs no `unsafe`. The straightforward way through this
section is the first row; the second exists for those who decided
otherwise.

| | pass | proficient | mastery |
|---|---|---|---|
| **Without `unsafe`** | `grep -rn "unsafe" src/` finds nothing. | `#![forbid(unsafe_code)]` stands at the root of the library and of the binary, so that it stays that way. | As proficient, and the performance rows are at proficient or better: the speed was reached in safe code. |
| **With `unsafe`** (instead of the row above) | Every `unsafe` block has a `// SAFETY:` comment, and the Miri command of the README passes. | Each block is as small as it can be and sits behind a safe function. Its comment names the invariant, where it is established, and why no input can break it. No `unsafe impl Send` or `Sync` exists. | For each use, a measurement on the same machine shows what it gains over the safe version, the safe version is kept as a test oracle, and Miri runs over your own unit tests too (`--lib`). |
| **Thread safety** | `Regex` is `Send + Sync`. | It is so automatically; a test of yours asserts it at compile time. | The same test states, for each public iterator type, whether it is meant to be `Send`, and the reason is in a comment. |

## 8. Defend your design

Write the answers down, in a `DESIGN.md` of your own, before you look at
any other solution. One short paragraph each is enough. An answer that
says "I did not think about this" is an honest answer and a useful one.

**The engine**

1. What exactly is one state of your automaton, and what is the largest
   number of states for a pattern of cost `c`? Why?
2. Give the proof, in five sentences or fewer, that one search costs at
   most on the order of `cost * length`. Which line of your code would have
   to change to break it?
3. A backtracking matcher gets the preference rules of SPEC 4.2 for free.
   Where do they live in your matcher? Point at the two or three places.
4. SPEC 3.7 refuses a loop around an expression that can be empty. Where
   do you decide that, and what does the decision cost for a deeply nested
   pattern? What exactly would go wrong in your matcher, with which
   pattern, if the check were removed?
5. How does a search find the leftmost start without starting again at
   every position? What would a simpler way cost?
6. What happens in your matcher when it is in the middle of a character
   of four bytes? Could your engine ever report an offset inside one? Why
   not?
7. How does `-i` reach the automaton, and why is `[^a]` handled correctly?
   What would it take to make the mode Unicode-aware?
8. Which pattern within the limits is the most expensive for you to
   compile, and which is the most expensive to run? How do you know?

**The library**

9. What does a search allocate, when, and who owns that memory? How did
   you meet the budget of SPEC 8.2, and which simpler design would have
   failed it?
10. Why can the items of `matching_lines` outlive the `Regex`? Write the
    signature from memory and say what each lifetime means.
11. Why is your `Regex` `Send + Sync`? What would you have to add to make
    `is_match` reuse scratch memory between calls on `&self`, and what
    would that cost?
12. Which items of your library are public, and which did you decide to
    keep private although the program could have used them?

**The program**

13. Which thread does what? Describe the path of one file from the command
    line to standard output, and name every point where a thread can
    block.
14. Why is your output the same for every number of threads? What is the
    largest amount of finished output that can be waiting in memory?
15. What happens, step by step, when the reader of your output goes away
    while eight workers are busy?
16. You read each file completely before searching it. What would have to
    change to search a file that is larger than memory, and which rule of
    SPEC 7.4 gets harder?

**Looking back**

17. Which decision did you change during the work, and what made you
    change it?
18. Which part of your code do you trust least? What test would raise
    your trust?
19. What did you measure, and which measurement surprised you?
20. If the next requirement were capturing groups, which modules would
    change, and which would not?

## 9. Your grade

Write the level of each row here: `-` for below pass, then `P`, `PR` or
`M`.

```text
2. Design          stages __  library/program __  automaton __  tree __  parallel __  errors __
3. Correctness     meaning __  hostile __  arithmetic __  stack __  files __  closing __  determinism __
4. Idiom           iterators __  lifetimes __  unwrap __  utf-8 __  output __  traits __  lints __
5. Tests           unit __  structure __  generated __  quality __
6. Performance     one thread __  scaling __  bound __  allocation __  compiling __
7. Unsafe          unsafe (one of the two rows) __  thread safety __
8. Defence         questions answered in writing: __ of 20
```

Your overall level:

- **Pass**: the gate holds, and every row is at pass or better.
- **Proficient**: the gate holds, every row is at pass or better, at most
  five rows are below proficient, none of those five is in section 3, and
  at least fifteen of the twenty questions are answered.
- **Mastery**: the gate holds, every row is at proficient or better, at
  least half of the rows are at mastery, including "the meaning of a
  match", "the bound" and "ordered parallel work", and all twenty
  questions are answered.

Then read the reference solution and its `DESIGN.md`, and write three
more lines: one thing it does that you would adopt, one thing you did
better, and one of your answers in section 8 that you would now change.
