# Capstone: Tansy, an interpreter

[All chapters](../../README.md) · [Specification](SPEC.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write an interpreter for a small scripting language, from an empty
crate: the program `tansy` that runs a script file and has an interactive
session.

```text
$ cat hello.tn
fn greet(name) { "hello, " + name + "!" }
for who in ["world", "Tansy"] { print(greet(who)); }
print(1 / 0);

$ tansy hello.tn
hello, world!
hello, Tansy!
hello.tn:3:7: error: division by zero
    print(1 / 0);
          ^^^^^

$ tansy
> let squares = []
> for n in 1..6 { push(squares, n * n); }
> squares
[1, 4, 9, 16, 25]
```

Underneath is everything a language needs: a lexer that turns text into
tokens, a parser for a grammar with eight levels of operator precedence, a
tree (or bytecode) that represents the program, name resolution with
lexical scope, an evaluator with first-class functions and closures,
strings and shared lists, and error messages that point at the exact place
in the source.

This is a **blank-file project**. There are no lesson files, no types and
no function signatures to fill in. You get a precise specification and 278
acceptance tests that run your finished binary and look only at what a user
would see: standard output, standard error and the exit status. Which
modules, types and functions exist is your design.

| file | what it is |
|---|---|
| [SPEC.md](SPEC.md) | the language and the program, completely: read all of it first |
| [CONCEPTS.md](CONCEPTS.md) | the ideas you need, with small compiled examples that are not Tansy |
| [HINTS.md](HINTS.md) | design hints per milestone; no code |
| [RUBRIC.md](RUBRIC.md) | how to judge your own result |
| `src/main.rs` | a stub that compiles and does nothing useful |
| `src/lib.rs` | an empty library for your interpreter, if you want one |
| `programs/` | six Tansy programs with their expected output |
| `tests/` | the acceptance tests, one file per milestone |

The model solution, with its own `DESIGN.md`, is in
`solutions/capstones/interpreter/` of the course repository. Look at it
after you have finished, or when you have been stuck on one problem for a
long time. It is one possible design, not the answer.

## Prerequisites

You need most of the course. The table says what each part is used for.

| from | you use it for |
|---|---|
| [Chapter 3](../../exercises/chapter03/README.md), Numbers, text and arrays | `i64` and checked arithmetic, `char` against bytes, slices |
| Chapters [4](../../exercises/chapter04/README.md) and [5](../../exercises/chapter05/README.md), Ownership and moves; Borrowing with references | who owns the tokens, the tree and the values; borrowing from the source text |
| [Chapter 6](../../exercises/chapter06/README.md), Expressions and control flow | `match` on tokens and tree nodes; the block-as-expression idea, which Tansy copies |
| [Chapter 7](../../exercises/chapter07/README.md), Handling errors | `Result` and `?` through every stage; errors as data with a position; exit statuses |
| [Chapter 8](../../exercises/chapter08/README.md), Modules, crates and Cargo | a library plus a small binary, modules with a clear direction of dependency |
| Chapters [9](../../exercises/chapter09/README.md) and [10](../../exercises/chapter10/README.md), Structs and methods; Enums and pattern matching | the token and tree types; `Rc`, `RefCell` and `Weak` from the interior-mutability lessons |
| [Chapter 11](../../exercises/chapter11/README.md), Generic code with traits | `Display` for values, `dyn Write` and `dyn BufRead` so that the core can be tested without a process |
| [Chapter 13](../../exercises/chapter13/README.md), Everyday traits | `Clone`, `Drop`, `Default`, `From`; what dropping a deep tree does |
| [Chapter 14](../../exercises/chapter14/README.md), Closures and captures | you implement closures for Tansy; you need to know exactly what one is |
| Chapters [15](../../exercises/chapter15/README.md) and [16](../../exercises/chapter16/README.md), Iterators and adapters; Standard collections | `Peekable`, `Vec` as a stack and as an arena, `HashMap` for names |
| [Chapter 17](../../exercises/chapter17/README.md), Working with text | UTF-8, character boundaries, columns, escape sequences |
| [Chapter 18](../../exercises/chapter18/README.md), Files, streams and processes | reading a file, buffered standard streams, flushing, a pipe that closes |
| [Chapter 19](../../exercises/chapter19/README.md), Threads and shared state | one thing only: starting a thread with a larger stack |
| Mastery [M1](../../mastery/m1_types/README.md), the type system in depth | lifetimes in a parser that borrows its input; trait objects |
| Mastery [M7](../../mastery/m7_perf/README.md), performance engineering | measuring before changing; allocation counts; data layout of the value type |
| Mastery [M9](../../mastery/m9_toolbox/README.md), the professional toolbox | lints, test strategy, and (stretch goal) fuzzing a parser |

Helpful but not required: [Chapter 12](../../exercises/chapter12/README.md) (operator overloading), to see how
Rust itself treats `+` and `==`; Mastery [M2](../../mastery/m2_unsafe/README.md) (unsafe), if you want to
understand how real interpreters pack their values. You do not need any
`unsafe` code in this project, and the model solution has none.

You do **not** need to have written an interpreter or studied compilers.
[CONCEPTS.md](CONCEPTS.md) explains lexing, Pratt parsing, environments,
closures and the alternatives from the beginning.

Time: as a rough estimate, plan 30 to 50 hours. M1 6 to 9, M2 4 to 6, M3 6
to 9, M4 5 to 8, M5 4 to 7, M6 3 to 6, M7 2 to 5. The estimates assume
that you read the SPEC before you start and that you write unit tests of
your own as you go.

## Setting up

Nothing to install: the package uses the standard library only.
`rust-toolchain.toml` selects Rust 1.99.0, and rustup fetches it on first
use.

Run every command in this directory (`capstones/interpreter/`).

```text
cargo build                 # builds target/debug/tansy
cargo test                  # all acceptance tests (they all fail at the start)
cargo test --test m1_expressions
cargo run -- script.tn      # run a script
cargo run                   # the REPL; end it with Ctrl-D (Ctrl-Z, Enter on Windows)
```

## How to work

1. Read [SPEC.md](SPEC.md) from start to end once. You will not remember
   all of it; you will know where things are. Then read
   [CONCEPTS.md](CONCEPTS.md).
2. Run `cargo test --test m1_expressions`. Every test fails, and each
   failure shows the script, what was expected and what your binary did.
   That is the starting line.
3. Work through the milestones in order (SPEC section 15). Each has its
   own test file:

   ```text
   cargo test --test m1_expressions      # literals, operators, print
   cargo test --test m2_diagnostics      # errors with a position
   cargo test --test m3_bindings         # let, var, blocks, if, while
   cargo test --test m4_functions        # functions and closures
   cargo test --test m5_collections      # lists, strings, for, built-ins
   cargo test --test m6_repl             # command line, exit, input, REPL
   cargo test --test m7_programs         # whole programs, speed
   ```

4. Before each milestone, decide on paper what data flows between the
   parts of your program, and which type carries it. This is the actual
   work of the capstone. [HINTS.md](HINTS.md) has questions for each
   milestone that help you find the decisions; it does not make them.
5. Write your own unit tests next to your code. The acceptance tests tell
   you that something is wrong; a unit test of your lexer tells you where.
   The rubric counts them.

Things that help:

- A failing acceptance test prints the script. Save it to a file and run
  it yourself: `cargo run -- try.tn`.
- When a diagnostic test fails by one column, count characters, not bytes,
  and remember that columns start at 1.
- Keep `cargo clippy --all-targets` clean as you go.
- When you change something central (the value type, how scopes are
  stored), run all earlier milestones again before you go on.

## Running the examples

```text
cargo run --release -- programs/queens.tn
cargo run --release -- programs/wordfreq.tn < programs/wordfreq.in
```

| program | what it shows |
|---|---|
| `programs/primes.tn` | lists as arrays, nested loops, `break` |
| `programs/queens.tn` | recursion, backtracking with one shared list, a function passed as a callback |
| `programs/bank.tn` | closures as objects: state in captured bindings, messages as strings |
| `programs/rpn.tn` | a small interpreter inside your interpreter; errors as values |
| `programs/life.tn` | lists of lists, `%` with negative numbers, list equality |
| `programs/wordfreq.tn` | reading standard input, strings as character sequences, sorting |

Write programs of your own. The fastest way to find a mistake in an
interpreter is to use it.

## How to assess your work

1. `cargo test` passes, in debug and with `--release`, three times in a
   row. (Three times because a test that passes only sometimes is a bug
   in buffering or process handling, and you want to know.)
2. `cargo clippy --all-targets` prints no warnings.
3. `grep -rn "unsafe" src/` finds nothing, or only a
   `#![forbid(unsafe_code)]` line.
4. `grep -rnE "unwrap\(\)|expect\(|panic!|unreachable!|todo!|\[[a-z_]+\]" src/`:
   look at every line it finds outside your tests and say why that line
   cannot fail for any input. If you cannot say it, it is a bug waiting.
5. Try to break it. Feed it a file of random bytes, a file with 100 000
   opening parentheses, a script that recurses forever, a script that
   builds a list containing itself and prints it, and
   `cargo run -- programs/queens.tn | head -1`. Every one must end with a
   message or with correct output, never with a Rust panic or a crash.
6. Time `cargo run --release -- ` on a script of your own that runs for
   about a second, change one thing in your evaluator that you believe
   makes it faster, and measure again. Write down both numbers.
7. Go through [RUBRIC.md](RUBRIC.md). It covers what tests cannot see:
   the structure of the code, how errors travel, how values are owned.
8. Compare with the model solution and read its `DESIGN.md`. Different is
   fine. Look for decisions you made without noticing that you were making
   one.

## Licence

The language Tansy, its specification, the example programs and the tests
are original work for this course and are covered by the MIT licence of
the course repository.
