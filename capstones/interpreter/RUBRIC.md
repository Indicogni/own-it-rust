# Rubric for the interpreter capstone

The tests tell you whether your interpreter behaves as the SPEC says. This
rubric is for everything a test cannot see. Go through it when all tests
pass, and be honest: the point is to find what to improve, not to collect a
grade.

Each row has three levels. "Working" is a finished capstone. "Strong" is
what a careful professional would hand in. Most first attempts are a mix.
Every level is written so that you can check it by looking at your code or
by running a command.

## 1. Gate: the things that are simply true or false

All of these must hold before the rest matters.

- [ ] `cargo test` passes in debug and with `--release`, three times in a
      row each.
- [ ] `cargo clippy --all-targets` prints no warnings, and no lint is
      switched off for a whole file or crate.
- [ ] `cargo build` prints no warnings.
- [ ] The package still has no dependencies: `[dependencies]` in
      `Cargo.toml` is empty.
- [ ] `grep -rn "unsafe" src/` finds nothing except, at most, a
      `#![forbid(unsafe_code)]` line.
- [ ] `tansy programs/queens.tn | head -1` ends without the word
      `panicked` on your terminal.
- [ ] You did not copy the model solution, or any other interpreter, into
      your `src/`.

## 2. Design: the stages and their boundaries

| | not yet | working | strong |
|---|---|---|---|
| Stages | one file, or stages that call into each other in both directions | separate modules for tokens, tree, evaluation and the command line; each one uses only earlier stages | you can state in one sentence what each module takes and gives, and `use` lines confirm it: the lexer does not mention the tree, the parser does not mention values |
| The parser | precedence decided by special cases spread over the parser, or copies of one function that no longer agree with each other | either one function per level that follows the grammar of SPEC 4.1 line by line, or one mechanism for all binary operators that is driven by data (a table of binding powers) | `not` and the comparison rule of SPEC 4.2 are each expressed in one place with a comment, and you can say what you would have to change to add an operator or a level, and why your choice between the two styles fits this language |
| Names | names are looked up by their text while the program runs | the checks of SPEC 6.4 are a pass of their own that runs before evaluation | that pass also decides *where* each name lives, so the evaluator never compares or hashes a name |
| The command line | `main` contains interpreter logic | `main` parses arguments, calls the library and sets the exit status | script mode and the REPL call the same "run this text in this session" function; the REPL adds only prompting and reading |
| Input and output | `println!` and `stdin()` in the evaluator | the evaluator writes and reads through values passed in (`Write`, `BufRead`) | a unit test runs a complete Tansy program and checks its output without starting a process |
| Public surface | everything is `pub` | what the binary and your tests do not need is private or `pub(crate)` | every `pub` item has a doc comment that says what it is for |

## 3. Correctness that the tests do not reach

Try each of these by hand. For each, "working" means a diagnostic or the
right output; a Rust panic message, a stack overflow abort or a hang is
"not yet".

| input | not yet | working | strong |
|---|---|---|---|
| a file of random bytes; a file that is not UTF-8 | panic or garbage | a diagnostic or `cannot read`, exit status 2 or 3 | you have a test in your own suite that feeds a few thousand pseudo-random token sequences to the lexer and parser in-process |
| `-` repeated 100 000 times before a `1`; `1` followed by 100 000 times ` + 1` | crash | accepted, or `nesting too deep` (SPEC 13.2) | you can say for each recursive function in your code what bounds its depth |
| 1000 nested calls where each call sits inside a deeply nested expression | crash | `stack overflow` as a diagnostic | you measured how much native stack one level of your evaluator needs, and the number is written next to the limit that depends on it |
| `let a = []; push(a, a); print(a == a, a);` | hang or crash | `true [[...]]` | you also know what `[a] == [a]` and a comparison of two different self-containing lists do in your implementation, and why |
| a 1 MiB script consisting of one string literal | slow or wrong | works | your lexer does not copy the source text more than once |
| `tansy` reading from a terminal, with Ctrl-C and Ctrl-D | leaves the terminal in a bad state | exits cleanly | |

## 4. Errors and control flow

| | not yet | working | strong |
|---|---|---|---|
| Representation | errors are strings built where they are printed | one diagnostic type (span and message) used by all stages | the kind of error (syntax or name, runtime) is carried by a type, not found out again from the message text |
| Travel | `eprintln!` and `process::exit` from deep inside | errors travel up as `Err` and are printed in one place | `return`, `break`, `continue` and `exit` use the same road as errors, and one `enum` lists every reason evaluation can stop early |
| Panics | `unwrap` or `expect` on things that depend on the input | every `unwrap`, `expect`, indexing and `unreachable!` outside tests has a comment that says why it cannot fail | there are fewer than ten of them, and the invariants they rely on are checked where they are established (for example by a unit test of the name pass) |
| Rendering | line and column computed in several places | one function turns a diagnostic and a source text into the three lines | that function has unit tests for tabs, `\r\n`, characters wider than one byte, a span over two lines and the end of input |
| Bookkeeping | a counter (call depth, nesting) is wrong after an error, so the REPL misbehaves later | counters are restored on every path | the restoring is structural (one function wraps "enter, run, leave"), not a matter of remembering to do it before each `return` |

## 5. Ownership and memory

| | not yet | working | strong |
|---|---|---|---|
| The value type | cloning a value copies a string's text or a list's elements | cloning a value is at most a reference-count change | you know `size_of` your value type, and why it has that size |
| Lists | a list is copied when it is assigned or passed (SPEC 5.2 violated in some case) | shared and mutable through one mechanism | no borrow of a list is held while Tansy code runs; you can show the three places where that matters |
| Environments | bindings found by walking a chain of hash maps | blocks own their bindings in an indexable structure | a block that declares nothing costs nothing at run time; a call costs a small, known number of allocations |
| Closures | closures copy the values they use | closures share bindings as SPEC 9 demands | you can draw who owns what after `make_counter()` returns, and you can name the reference cycles your design leaks |
| The tree | the tree is cloned to run a function | function bodies are shared, not copied | dropping a deeply nested tree or list cannot overflow the native stack within the limits of SPEC 13, and you can say why |
| Lifetimes | `'static` or `String` clones used to silence the borrow checker | a clear rule for what borrows from the source text and what owns its data | the rule is written down in one comment, and the REPL (where source texts come and go) follows it |

## 6. Idiom

| | not yet | working | strong |
|---|---|---|---|
| Types | `i32` flags, strings as type tags, parallel vectors | enums for tokens, tree nodes, operators, values, built-in functions | illegal states are hard to build: for example an assignment target is its own type, not "any expression, checked later" |
| Matching | long `if`/`else if` chains on kinds; wildcard arms that hide forgotten cases | `match` with all variants named where it matters | adding a variant to your tree makes the compiler show you every place that must handle it |
| Arithmetic | `as` casts and unchecked `+` | checked operations for everything SPEC 7.2 defines | no `as` between integer types anywhere; conversions that can fail are handled |
| Text | byte indexing into `&str`; `chars().nth()` in loops without knowing its cost | character boundaries respected everywhere | you can say for `len`, indexing and `for` over a string what each costs in your implementation |
| Traits | hand-written `to_string` functions | `Display` for values and for errors | the display form and the quoted form share one implementation |
| Naming and comments | comments repeat the code | comments say why; names say what | each module starts with a short description of its job and of the SPEC sections it implements |

## 7. Tests of your own

| | not yet | working | strong |
|---|---|---|---|
| Unit tests | none: only the acceptance tests | unit tests for the lexer, the parser and the diagnostic renderer | a unit test fails first, and names the faulty stage, when you break any one stage on purpose (try three) |
| Parser tests | none | precedence checked through results | the tree's shape is checked directly (for example by printing it with full parentheses), including the rules for `not`, unary minus and comparisons |
| Edge cases | found by the acceptance tests | your tests cover the smallest and largest integer, empty string and list, and the end of input | each limit of SPEC 13 has a test at the limit and one step beyond |
| Speed of the suite | | your own tests run in under a second | none of them starts a process |

## 8. Performance

Use `--release` for every number.

| | not yet | working | strong |
|---|---|---|---|
| The speed tests | time out, or pass only in release | pass in debug with a wide margin | each takes less than two seconds in debug |
| A number | "it feels fast" | you timed `fib(27)` (as in `tests/m7_programs.rs`, with 27) and a loop of a million rounds, and wrote the times down | you profiled one of them, changed one thing because of the profile, and wrote down the time before and after |
| Allocation | unknown | you can say what allocates in a call and in a loop round | a loop round that declares nothing and calls nothing allocates nothing; you checked it with a counting allocator (Mastery M7) |
| Output | one system call per `print` | buffered, and flushed where SPEC 2.3 says | you measured the difference with a script that prints 200 000 lines |
| Honest limits | | you know which operations are slower than a user would expect (string indexing? `+` on long strings in a loop?) | they are listed in a comment or in your notes, each with the reason you left it that way |

For orientation only: the model solution, a tree walker without any tuning
beyond what its DESIGN.md describes, runs `fib(27)` in well under a second
in a release build (about 0.05 seconds of processor time on the Apple
silicon laptop it was written on).

## 9. Unsafe code and dependencies

This project needs neither.

| | not yet | working | strong |
|---|---|---|---|
| `unsafe` | present, to "make it fast" or to get around the borrow checker | none | `#![forbid(unsafe_code)]` at the top of the crate, so that it stays that way |
| Dependencies | a crate added for something small | none | you can name what you would use a crate for in a real product (line editing, fuzzing, benchmarks) and why it is not needed here |

If you do a stretch goal that needs `unsafe` (packing values into 8 bytes,
raw terminal mode), the standard of the unsafe chapter and of Mastery M2
applies: the smallest possible block, a `// SAFETY:` comment that states
the reason and not the wish, a safe interface around it, and a run under
Miri for everything that does not talk to the operating system.

## 10. Evidence to keep

Collect these in a notes file of your own, outside `src/`. They are what
you would show a reviewer.

1. The output of `cargo test` in debug and release, with the test counts.
2. The output of `cargo clippy --all-targets`.
3. A drawing or a paragraph for each: the stages and what flows between
   them; who owns a binding that a closure captured.
4. The list of every `unwrap`, `expect`, slice index and `unreachable!`
   outside tests, each with its reason.
5. Two timing numbers before and after one change, with the command you
   used.
6. The five inputs from section 3 and what your interpreter did with each.
7. One thing you would do differently if you started again.
