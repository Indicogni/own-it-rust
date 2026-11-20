# rgx: design of the reference solution

This is one way to meet the SPEC, with the reasons for its decisions and
the alternatives that were considered. It is not the only good design. Read
it after you have graded your own work, and compare decision by decision.

The solution is about 1 450 lines of code, 1 500 lines of tests and 600
lines of comments, with no dependency and no `unsafe`
(`#![forbid(unsafe_code)]` in both crates).

## 1. The shape

```text
library crate `rgx`                         binary crate `rgx`

pattern text                                arguments
    |  parser.rs                                |  cli/args.rs
    v                                           v
  Ast  --- cost check (lib.rs) ---            Config ----> Regex (library)
    |  compile.rs                               |  cli/walk.rs
    v                                           v
 Program (immutable)                        [Input, Input, ...]   in output order
    |  vm.rs + Scratch (per thread)             |  cli/ordered.rs: workers -> results in order
    v                                           v  cli/search.rs: one input -> bytes
 matches                                    stdout, stderr, exit status (cli/mod.rs)
```

| module | knows | does not know |
|---|---|---|
| `error.rs` | the sixteen kinds, their messages, an offset | anything else |
| `ast.rs` | the syntax tree, its cost, what an assertion means | text, automata |
| `class.rs` | sets of characters as ranges | patterns |
| `parser.rs` | the grammar of SPEC 3.1, where each error points, which expressions can be empty | automata |
| `compile.rs` | how a tree becomes instructions | haystacks |
| `vm.rs` | how to run instructions on a haystack | the tree, the pattern text |
| `lib.rs` | the public types: `Regex`, `RegexBuilder`, `Matcher`, the iterators | files, threads |
| `oracle.rs` | (tests only) SPEC section 4 as a backtracking matcher | the compiler and the matcher |
| `cli/*` | arguments, files, threads, output | anything inside the engine |

The `use` lines go one way only. The program uses five public items of the
library and nothing else.

## 2. The engine

### 2.1 Which automaton, and how it is run

**Decision: a nondeterministic automaton, written as a list of
instructions, simulated one character at a time with all live states at
once.** (This is usually called a Pike VM, after Rob Pike; the construction
of the automaton goes back to Ken Thompson.)

The compiled pattern is `Box<[Inst]>` with seven kinds of instruction:
`Char`, `Any`, `Class`, `Split`, `Jump`, `Assert`, `Match`. A state is an
index into that slice. The matcher keeps the ordered set of states the
automaton can be in at the current position, and moves the whole set over
one character. A set holds each state at most once, so one step costs at
most the number of instructions, and a search costs at most (number of
instructions) x (number of characters + 1). The compiler emits at most
`2 * cost + 1` instructions (a unit test checks this), which gives the
bound of SPEC 4.5. Because no loop can go round without consuming (section
2.3), "the first arrival at a state is the preferred one" is exactly true,
and dropping later arrivals loses nothing.

Alternatives:

- **Backtracking.** Simple, and gives the preference rules for free. It is
  exponential on `(a|aa)+b`, which the SPEC forbids. Backtracking with a
  table of visited (state, position) pairs is linear in time but needs
  memory proportional to cost x length, which SPEC 4.5 also forbids.
- **A deterministic automaton built in advance.** Fast to run, but the
  number of its states can be exponential in the pattern, so compiling
  breaks SPEC 6.2.
- **A deterministic automaton built lazily, with a bounded cache.** This
  is what fast production engines do. It needs either interior mutability
  in `Regex` or a cache per matcher, a fallback when the cache thrashes,
  and care to keep the leftmost-first rule. It is the right next step
  (stretch goal 2) and the wrong first step: the simulation is needed as
  the fallback and as the reference anyway.

### 2.2 Where the preference rules live

A backtracking matcher reports the match of SPEC 4.3 because of the order
in which it tries things. The simulation gets the same result from three
places, and only these three:

1. **The compiler orders the targets of every `Split`**: the earlier
   branch of `|` first, one more round of a loop before the exit.
2. **`vm::follow` walks depth first, first target first**, and the state
   set keeps insertion order. A state that is already in the set is not
   added again: the first arrival came by a preferred path, and two paths
   that are at the same state at the same position have the same future.
3. **`vm::search` cuts the set at the first `Match` it meets.** Everything
   behind that state in the set is less preferred than a match that
   already exists. States before it stay alive and may still produce a
   better (more preferred) match later.

The leftmost start needs no second pass. At every position, while nothing
has matched yet, a new path is started at the **back** of the set, and
each state remembers the offset where its path began (`starts[pc]`). An
earlier start is therefore always ahead of a later one. The simpler way,
one anchored search per start position, is quadratic: the test
`a_failing_search_does_not_start_again_at_every_position` exists for it.

### 2.3 Loops, and why a loop must consume

```text
e+ :   BODY: <e>                 e* :        split BODY, END
             split BODY, END          BODY:  <e>
        END:                                 split BODY, END
                                       END:
```

After a round, the `split` prefers another round. `e{n,}` is `n - 1`
copies and then `e+`. `e{n,m}` is `n` copies and then `m - n` nested
optional copies, as in `ee(e(e)?)?`, so that leaving at any point skips
the rest with one jump.

SPEC 3.7 refuses a loop whose body can match the empty string. The parser
decides this on the way up: every parsing function returns the tree
together with one flag, "can be empty", so the check costs nothing extra
and does not walk a nested group once per enclosing loop.

The rule in the SPEC comes from a failure of this solution, and the story
is worth telling. The first design accepted such loops and gave them a
meaning: "a round that consumes nothing is the last one". The compiled
shape above implements that for the round a path is in: a round that ends
where it began arrives at the `split` with `BODY` already in the state set,
so only `END` is left. A backtracking oracle with the same rule agreed
with the automaton on 24 000 generated cases in a debug build. In a release
build, which runs more cases, it disagreed:

```text
pattern (\b|[^é-ž])+   on a haystack that ends in ...\n\nb
definition:  the round that matches \b consumes nothing, so the loop ends before b
automaton:   the loop goes on and takes b
```

The cause is not a bug in one line. The simulation identifies a path with
its state and its position. What may follow an empty round depends on
more: on whether the round began at this position. Two paths that the
simulation must treat as one (that is where its speed comes from) have
different futures under any rule of the kind "an empty round ends the
loop", "an empty round is not allowed" or "only the first round may be
empty". Perl, JavaScript and the automaton engines each resolve this
differently (the SPEC has the table). This solution found no way to
implement the Perl rule or the JavaScript rule exactly without extra state
per path for every enclosing loop, which would cost the time bound its
simple form.

With loops that must consume, the automaton has **no cycle that consumes
nothing**. The future of a path then depends on its state and position
only, the simulation is exact, and the definition in SPEC 4.2 needs no
special case. After the change, the oracle and the automaton agree on
100 000 generated patterns in the test suite, and they agreed on a further
8.4 million pattern and haystack pairs with other seeds in a one-off run.

### 2.4 Characters, not bytes

The matcher decodes the haystack with `str::chars` and compares `char`s.
The position only ever advances by `len_utf8()` of a decoded character, so
every reported offset is a character boundary **by construction**; there
is no check to forget. Assertions get the characters on both sides of the
position (`Around { prev, next }`) and never look at the haystack.

The alternative is an automaton over bytes with the UTF-8 encoding
compiled into it (stretch goal 4). It avoids decoding and makes a lazy
deterministic automaton practical, at the price of a much harder compiler
for classes: `[^a]` becomes a small automaton of byte ranges.

A class is parsed into a `RangeSet` (sorted, merged ranges of code points;
one sort per class, so a class of 20 000 members is cheap) and compiled
into a `CharSet`: a 128-bit map for ASCII and a binary search for the
rest. The logarithm that SPEC 4.5 allows is this binary search.

Ignoring case is done by the compiler, not by the matcher: an ASCII letter
becomes a two-member class, and a class is folded **before** it is
negated, which is why the tree keeps `negated` apart from the set until
compile time. The matcher has no idea that the mode exists, and the
haystack is never touched.

### 2.5 Limits before work

`Regex::new` is: parse, compute the cost on the tree, refuse if it is over
the limit, compile. Nothing whose size depends on a repetition count
exists before the check.

- The cost uses saturating `u64` arithmetic. Seven nested `{1000}` overflow
  `u64`; saturation makes that "too large" instead of "small".
- A count is read with saturating arithmetic too, so forty digits are just
  a number above 1000.
- The parser recurses only through groups, and `open_groups` stops it at
  200. Concatenations and alternations are flat `Vec`s, so a pattern of
  3 000 branches is one node and nothing in the crate (the compiler, the
  cost function, the derived `Drop`, `Debug` and `Clone`) recurses deeper
  than 200 groups plus one level per quantifier.
- The constructors `Ast::concat` and `Ast::repeat` keep one invariant: **a
  tree costs 0 only if it is `Ast::Empty`**. Without it,
  `((){1000}){1000}...` twenty deep has cost 0, passes the limit, and
  keeps the compiler in 1000^20 loop rounds that emit nothing. With it,
  compiling takes time proportional to the pattern length plus the cost.

Measured in a release build on a recent laptop: the most expensive
patterns to compile (`a?` 50 000 times, cost 100 000) take about 2 ms; a
pattern of cost 10^9 is refused in under a microsecond. An instruction is
12 bytes (a unit test pins this), so the largest program is 2.4 MB.

## 3. The library surface

### 3.1 `Regex` is immutable, a `Matcher` holds the scratch memory

A search needs two state sets and a work list. **Decision: that memory is
a separate type, `Matcher<'r>`, which borrows the `Regex`.**

- `Regex` has no interior mutability, so it is `Send + Sync` without a
  line of code, and eight threads share one with no lock.
- A `Matcher` is made once per thread or per pass. Making it allocates
  seven times (two sets of three arrays, one work list). Searching with it
  allocates **nothing**, for any haystack: the sets are "sparse sets" of
  fixed capacity, and the work list is sized by the number of `Split`
  instructions, which bounds its depth. `tests/own_no_alloc.rs` counts
  this. (The first version reserved 32 entries for the work list and let
  it grow; that test, with sixty nested groups, caught the growth.)
- `find_iter` and `matching_lines` make one matcher per pass, which meets
  the budget of SPEC 8.2 with 7 allocations out of 100.
- `Regex::is_match` and `Regex::find` make a matcher per call. That is the
  honest cost of a convenient `&self` method; code that searches many
  haystacks uses a `Matcher`, as the program does.

Alternatives: a `Mutex<Scratch>` or a thread-local pool inside `Regex`
would let `is_match(&self)` reuse memory, at the cost of a lock or of
hidden state, and it would make the cost of a call depend on what other
threads do. Scratch memory proportional to the pattern (36 bytes per
instruction, 7 MB at the limit) is small enough to give each worker its
own.

### 3.2 Iterators

`Matches<'h, M>` is generic over `M: BorrowMut<Matcher<'r>>`. One
implementation serves `Regex::find_iter`, which owns its matcher, and
`Matcher::find_iter`, which borrows one. It searches inside `next` only,
which is the laziness that SPEC 4.4 asks for. After an empty match it
steps over one character with `chars().next()`, and it searches the whole
haystack from an offset (`find_at`) instead of a sub-slice, so that `^` and
`\b` see what comes before.

`MatchingLines<'r, 'h>` has two lifetimes on purpose: its items are
`(usize, &'h str)` and do not mention `'r`. Lines come from
`str::split_terminator('\n')`, which already has the three properties that
SPEC 7.2 wants: no line for an empty text, no empty line after a final
line feed, and slices of the original text.

### 3.3 What is public

`Regex`, `RegexBuilder` (for `-i`; a builder can gain options without
breaking callers), `Matcher`, `Matches`, `lines`, `Lines`, `MatchingLines`,
`Error`, `ErrorKind` (`#[non_exhaustive]`) and the three limits. The tree,
the program and the matcher's internals are private: they are the parts
most likely to be replaced.

## 4. The program

### 4.1 Plan first, then search

`walk::plan` turns the operands into the complete list of inputs **in
output order** before any file is opened. From then on the program works
on "input 0, 1, 2, ..." and never thinks about paths again. The byte-order
rule of SPEC 7.3 is one `sort_unstable_by` on the entry names of each
directory; sorting names, not full paths, is what puts `a/x` before `a-b`.

Cost of this decision: the whole tree is listed before the first match is
printed, and one small record per file stays in memory. For a tree of a
million files that is a noticeable pause. Walking and searching at the
same time is possible, but the order of the output then has to be carried
through the walk; it was not worth it here.

### 4.2 Ordered results from unordered work

`ordered::for_each_ordered(items, threads, make_state, work, consume)` is
the only place where threads meet, and it knows nothing about files:

- Scoped worker threads take the next index from a `Gate` (a `Mutex` and a
  `Condvar`), run `work`, and send `(index, result)` over a channel.
- The calling thread receives, keeps early results in a `BTreeMap`, and
  calls `consume` strictly in index order.
- **Back-pressure:** a worker may take item `i` only while
  `i < consumed + 4 * threads`. A slow first file can hold back at most
  that many finished results. Without the window, a large first file would
  let the workers finish everything else and keep all of it in memory.
- **Early end:** when `consume` returns `Break` (the output was closed),
  the gate stops handing out work; running workers finish their current
  file and leave.
- **Panics:** a guard stops the gate when a worker unwinds, so nobody
  waits for a result that will never come, and the scope passes the panic
  on.

Only the calling thread writes, to both streams. That is the whole
argument for determinism: what is written is a function of the results in
index order, and the results do not depend on which thread computed them.
Each of the three properties above has a unit test that uses no file.

Alternatives: one thread per file (simple, but thousands of threads and no
bound on memory); a shared work queue with printing under a lock (fast,
and the output order then depends on timing, which is the bug the SPEC
names); splitting large files into chunks for several workers (better
scaling on few large files, and line numbers then need a second pass).

### 4.3 One input

`search::search_input` reads the file with `fs::read`, validates it once
with `str::from_utf8` (no copy), and walks `rgx::lines`. Every line is a
slice of that one buffer. Output is appended to one `Vec<u8>` per input;
the worker returns it, and the main thread writes it through a 64 KiB
`BufWriter` around the locked standard output. A byte of a selected line
is copied twice: into the per-input buffer and into the writer.

`-l` stops at the first selected line. `-o` uses one `find_iter` pass per
line to decide both whether the line is selected and what to print.

Not done: an input's output is buffered whole, and a file is read whole.
A file larger than memory fails with the operating system's error message
instead of being searched (stretch goal 6).

### 4.4 Errors and the exit status

Three kinds, three types: `args::UsageError`, `rgx::Error`, and the
`Outcome::error` text of an input. `cli::run` checks them in the order of
SPEC 7.1. The exit status is one `match` on `(failed, selected)` at the end
of `cli::search`. A closed pipe is recognised by
`io::ErrorKind::BrokenPipe` on the write or the final flush, and ends the
run without a message; nothing uses `println!` for results, because it
panics in exactly that case.

The messages on standard error do not use `eprintln!` either, for the same
reason: it panics when standard error is a pipe whose reader has gone, and
a status of 2 would become a status of 101. `cli::report` writes the line
and ignores a failure, because there is no one left to tell.
`tests/own_closed_stderr.rs` closes the pipe and checks the status. (The
first version of this solution used `eprintln!`; the review found it with
`(sleep 1; rgx x no-such-file) 2>&1 | true`.)

## 5. How it was tested

- **Unit tests in every module** (45): tree shapes, range-set operations,
  instruction listings for small patterns, line splitting, argument
  parsing, the ordered-work properties.
- **The SPEC as tests**: the lists of 4.2, the tables of 4.3 and 4.4, and
  the cost examples of 6.1 are checked literally.
- **A differential test** (`oracle.rs`): SPEC section 4 written as a
  backtracking matcher over the syntax tree, compared with the engine on
  generated patterns and haystacks with a fixed seed (10 000 patterns in a
  debug build, 100 000 in release, four haystacks each, every start
  offset). It shares only the parser with the engine. Section 2.3 tells
  what it found.
- **Random strings**: 100 000 strings over the metacharacters; none may
  panic, every error offset must be the offset of a character, every
  reported range must be a slice.
- **Outside the package**, during development: a similar generator against
  the `regex` crate (version 1.13.2, with its Unicode classes written as
  the ASCII ones). Of 400 000 generated patterns, `rgx` refused 65 046 as
  loops that can be empty; on the others, 1 189 460 comparisons of
  `find_iter` showed no difference. The package itself has no dependency,
  so this check is not in the tree.
- **Miri** over the unit tests and the library acceptance tests. The
  solution has no `unsafe`; the run guards the test helpers and would
  catch a data race.

## 6. Numbers

Release build, a recent laptop with more than eight cores, `speed_report`
(41 MiB in 64 files). The machine was shared with other jobs, and the
numbers moved between runs; these are from the best of three runs.

| pattern | `-j 1` | `-j 8` | speed-up |
|---|---|---|---|
| `took 249 ms` | 102 MiB/s | 814 MiB/s | 8.0 |
| `request [0-9]*7 took [12]?[0-9] ms$` | 92 MiB/s | 629 MiB/s | 6.8 |
| `(host\|node)[0-9]+ \w+\[1?[0-9]\]` | 72 MiB/s | 460 MiB/s | 6.4 |
| `no such text\|^$` | 82 MiB/s | 463 MiB/s | 5.7 |

In the two other runs, one thread reached 73 to 136 MiB/s, and the
speed-up with eight threads was between 3.0 and 6.9. The last pattern
scaled worst in both (3.0 and 3.1); the cause was not found, and with
other jobs on the machine it could not be separated from noise.

Linearity, `(a|aa)+b` on a run of `a` with no `b`: 2.6 ms for 100 000
bytes, 5.2 ms for 200 000, 10.1 ms for 400 000, 20.3 ms for 800 000. That
is 25 to 26 ns per byte at every size.

What limits the speed: every character visits every live state, and
nothing skips ahead. A production tool is many times faster on the first
pattern because it searches for the literal `took 249 ms` with a
specialised substring search before any automaton runs.

## 7. Against the rubric

Honest places where this solution is below "mastery":

- **One thread** (6): it reaches the 60 MiB/s line without any prefilter,
  but a literal pattern should be much faster than a class pattern, and
  here it is not.
- **Scaling** (6): all four patterns met the mastery line in the best
  run, three in the second and two in the third. Why the last pattern
  scaled poorly in two runs out of three is an open question.
- **Allocation** (6): the per-input path of the program is not counted by
  a test; only the library is.
- **Unfriendly file trees** (3): file names that are not valid Unicode are
  printed with replacement characters, and the behaviour on a file larger
  than memory has not been tried.
- **Output** (4): one input's output is held in memory completely.

## 8. What would change next

1. A literal prefilter in `vm::search`, behind the existing `Program`.
2. A lazy deterministic automaton as a second engine behind `Matcher`,
   with the simulation as fallback and the oracle as referee.
3. Chunked reading with a carry-over for the last partial line, so that
   memory is bounded by the longest line.
4. Capturing groups: `starts[pc]` becomes a small array of offsets per
   state, the compiler emits `Save` instructions, and the parser gives
   groups numbers. The program, `ordered.rs` and `class.rs` would not
   change at all.
