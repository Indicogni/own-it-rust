# Hints for the interpreter capstone

These are design hints. They contain no code, and they do not tell you
which types to write. Each milestone has a first hint (a direction), a
second hint (more specific), and questions to check your understanding.
Try for a while before you read a hint, and read the second one only if
the first did not help.

When a test fails, read its message first. It shows the script, what the
SPEC demands and what your binary did. Then run the script by hand.

---

## Before M1 · The shape of the whole thing

**Hint 1.** An interpreter is a pipeline. Text goes in at one end; at the
other end things happen. Decide what the stages are and, more important,
what *value* each stage hands to the next. Draw it. Every arrow on the
drawing is a type you have to design, and every box is a module.

**Hint 2.**

- Keep each stage ignorant of the later ones. The thing that makes tokens
  should not know what an expression is; the thing that builds the tree
  should not know what a value is. You can then test each one alone.
- Decide early where a stage's errors go. If every stage returns its
  result or one error value of a shared shape (what, and where in the
  text), the top of your program needs to know only that shape.
- The tests talk to the binary, so the binary must exist from the first
  hour. Make `main` read a file and exit with the right status before the
  language can do anything; then grow the middle.
- Standard output and standard input: if the core of your interpreter
  writes to "something that implements `Write`" and not to the real
  standard output, you can run whole programs inside a unit test and look
  at what they printed.

**Check your explanation.**

- For each arrow in your drawing: who owns the data that flows along it,
  and for how long does it have to live?
- Which of your stages can fail, and with what information?

---

## M1 · Expressions

**Hint 1.** Two separate problems: cutting the text into tokens, and
building a tree from the tokens. For the second, do not write one function
per precedence level by habit. Read about binding power in CONCEPTS.md
(section 3) and look at SPEC 4.2 as a *table*: your parser can be driven by
that table.

**Hint 2.**

- The lexer's whole job is "at this position, what is the longest token?".
  Work on characters and remember positions as byte offsets; you will need
  them for M2, and adding them later means touching every token.
- SPEC 4.2 has two things a plain table of left-associative operators does
  not give you: `not`, a prefix operator that binds *weaker* than some
  infix operators, and comparisons that must not chain. Decide how your
  parsing scheme expresses each before you write it. Test both with the
  examples in the SPEC.
- You cannot see `(-a) * b` against `-(a * b)` in an integer result, with
  one exception. Find it; a test uses it.
- For the tree, an `enum` whose variants own their children is the usual
  start. Ask what each node must remember besides its children.
- Arithmetic: Rust already has the operations the SPEC describes,
  including the ones that report overflow. Look for the `checked_` family.
  One of them refuses a case that the SPEC defines; find out which.
- `print` is the only name in M1. Resist special-casing it in the parser;
  a call is a call. How the evaluator finds out what `print` means can be
  simple now and proper in M3.

**Check your explanation.**

- In `1 - 2 - 3`, which subtraction is the root of the tree, and which
  rule of your parser makes it so?
- Why does `not 1 == 2` parse, and `1 + not true` not?
- What does your lexer do with `1..5`, and why is that not a problem for a
  language without fractions?

---

## M2 · Diagnostics

**Hint 1.** A diagnostic is data: a place and a message. Produce that data
where the error is noticed, carry it up unchanged, and turn it into three
lines of text in exactly one function. That function needs the source
text; nothing else does.

**Hint 2.**

- Store positions as byte offsets and compute line and column only when
  you print. One error per run means this happens once; a table of line
  starts is not needed.
- Columns count characters, the source line drops a `\r`, tabs become
  spaces, a span can end on a later line, and the end of input is a
  position. Write a unit test for each of these five before you run the
  acceptance tests; the rendering function is a pure function and easy to
  test.
- SPEC 11.3 says a lexical error is reported only if the parser reaches
  it. If your lexer runs to the end first, it cannot just return the first
  error it meets. Think about what a lexical error could be *as a token*.
- SPEC 11.4: the span of `(a + b) * c` includes the parentheses, the span
  of the sum inside does not. One way is a tree node for parentheses.
  Another is to notice that the parser always knows where an expression
  started and where the last consumed token ended.
- A runtime error has to leave the evaluator from any depth. You already
  have the tool: a function that returns `Result`, and `?`.
- 100 000 opening parentheses: where does a recursive parser keep its
  state? What is the cheapest place to count?

**Check your explanation.**

- For `print(1 +);` and for `print(1 @);`: which part of your program
  notices the problem, and which token is "found"?
- Where in your code is the exit status decided, and how does it learn
  which kind of error happened?

---

## M3 · Bindings and control flow

**Hint 1.** Read SPEC 6.2 twice. The scope of a name is the whole block,
also the lines before the declaration. That one rule decides how you find
names. A design where a declaration "adds the name when it runs" gives the
wrong answer for the example in 6.2, silently.

**Hint 2.**

- You need two things that are easy to mix up: what is known *before the
  program runs* (which names exist in which block, which of them are
  `var`) and what exists *while it runs* (the values). SPEC 6.4 can be
  answered completely from the first kind. Consider a separate walk over
  the tree, between parsing and evaluating, that does only this.
- That walk must know all names of a block before it looks at the block's
  first statement. And it has several possible errors but must report the
  first in the text, which is not always the first it finds.
- At run time, "uninitialized" is a state a binding can be in. Give it a
  representation.
- "Each time a block is entered, all its bindings are made anew" (6.1).
  Keep that sentence in mind for M4; a design that reuses the storage of a
  loop body will pass M3 and fail M4.
- SPEC 4.3, rule 4, is about the *first token of a statement*. It is a
  decision in the statement parser, not in the expression parser.
- `break` and `continue` must leave an unknown number of nested blocks and
  expressions and stop at the loop. So must a runtime error, and it stops
  at the top. If you solved one, look at whether the same mechanism
  carries the other.
- Once names are checked before the run, the evaluator does not need to
  search for a name by its text at all. That is an optimisation you can
  postpone, but decide now where the information would be stored.

**Check your explanation.**

- In SPEC 6.2's second example, which binding does `a` on the line
  `print(a);` mean, at what moment do you know that, and at what moment
  do you know that it has no value?
- Why can "unknown name" be found before the program runs, but "used
  before it is initialized" cannot?

---

## M4 · Functions and closures

**Hint 1.** A function value is two things: the code, and the bindings
that were in scope where the `fn` was written. The second part must stay
alive as long as the function value does, which can be much longer than
the block that created the bindings. Ask of your M3 design: who owns a
block's bindings, and can a second owner join?

**Hint 2.**

- If bindings live on a stack that shrinks when a block ends, a closure
  that outlives its block points at nothing. Either the bindings move to a
  place with shared ownership when they are captured, or they live there
  from the start. The second is simpler; the first is faster. CONCEPTS.md
  section 7 shows both ideas on a toy.
- A call does not run in the caller's scope. Re-read step 2 of the list
  in SPEC section 9, then look at the test with `speak` and `other`.
- Two closures made in the same block entry must see each other's
  assignments (the counter tests). Two entries of the same block must not
  (the loop tests). Both follow from one decision about what is shared.
- The body of a function has to be reachable from every closure made from
  it, also from closures that live longer than the tree of a REPL entry.
  Decide who owns function bodies.
- `return` is one more thing that leaves nested constructs and stops
  somewhere specific.
- Count calls for the limit of 13.3 in one place, and make sure the count
  goes down again on *every* way out of a call: normal return, `return`,
  runtime error. The REPL goes on after an error.
- 1000 nested Tansy calls are several thousand nested Rust calls if your
  evaluator is recursive, and in a debug build a Rust frame is large. The
  main thread of a process has a fixed stack, and on Windows it is 1 MiB.
  A thread you start yourself can have any stack size.
- Closures that capture their own environment form a reference cycle.
  SPEC section 14 says you may leak them. Know that you do, and why.

**Check your explanation.**

- When `make_counter()` returns, which values still refer to `count`'s
  binding? Draw the ownership.
- A function is declared in a loop body and stored in a list on each
  round. How many function values exist afterwards, how many copies of
  the code, how many sets of bindings?

---

## M5 · Lists, strings and `for`

**Hint 1.** SPEC 5.2 fixes the ownership of lists: one list, many ways to
reach it, and it can be changed through any of them. You know the Rust
types that say exactly this in a single-threaded program. Strings are
different: they never change. Does that allow something cheaper?

**Hint 2.**

- Cloning a *value* happens all the time in an evaluator (reading a
  variable clones). Make sure cloning a list value or a string value does
  not copy the elements or the text. A speed test in M7 exists for this.
- Shared mutable data and `for`: the SPEC defines the loop over a list by
  an index that is checked again before each round. If you hold a borrow
  of the list while the body runs, the body's `push` will fail at run
  time. Borrow for as short a time as you can.
- The same care applies to assignment to an element, and to printing: a
  list can contain itself. SPEC 5.3 says what to print then; work out what
  you need to remember while printing to notice it.
- A string is indexed by characters, and Rust strings are indexed by
  bytes. Decide whether you convert on each access or store something
  else, and know what each choice costs.
- The built-in functions have one job each and many error cases. A table
  (name, number of arguments) plus one function that matches on the
  arguments keeps them in one place. Their type errors point at an
  *argument expression*, so the place that calls a built-in must still
  know where the arguments were written.
- Equality of lists is recursive. So is dropping a deeply nested list.
  SPEC 13.5 tells you how deep you must support.

**Check your explanation.**

- After `let a = [1]; let b = a; push(b, 2);` how many lists exist, and
  how many reference counts were changed by the three statements?
- Why does `for x in xs { push(xs, x); }` not end, and why is that the
  specified behaviour and not a bug?

---

## M6 · The command line and the REPL

**Hint 1.** The REPL is not a second interpreter. It is a loop around the
same "run this program text in that top-level scope" operation that the
script runner calls once. What has to survive from one call to the next?
List it; it is more than the values.

**Hint 2.**

- Two kinds of state survive: what your name check knows about the top
  level, and the top-level bindings themselves. An entry with a syntax or
  name error must leave both untouched (12.3).
- SPEC 12.5 numbers the lines of the whole session, and a function typed
  in entry 3 can fail while entry 9 runs. The diagnostic must show the
  line of entry 3. The simplest design that gives you this keeps one thing
  growing for the whole session; think about what your spans are offsets
  *into*.
- "Is the entry complete?" (12.2) is a question about tokens. You have
  something that makes tokens.
- `input()` and the REPL read from the same stream. If each wraps
  standard input in its own buffer, one will swallow lines meant for the
  other. There must be exactly one buffered reader, and both must use it.
- The interactive test keeps the pipe open and waits for your answer. If
  your output sits in a buffer, the test times out. Find every place
  where SPEC 2.3 says "flush".
- `println!` panics when standard output is closed. SPEC 2.4 forbids
  that. Write through a handle, get a `Result`, and let the error travel
  the same road as your other reasons to stop.
- `exit(n)` must flush and must run from any depth. Calling
  `std::process::exit` deep inside the evaluator skips destructors and
  buffered output. You have a mechanism for "stop everything and go to
  the top" already.

**Check your explanation.**

- Trace `let z = 1 / 0`, then `z`, then `let z = 5` through your REPL.
  What does your name check know after each entry, and what is in the
  binding?
- What exactly is written to standard output, byte for byte, when the
  whole input is the line `1 +`? And what to standard error?

---

## M7 · Programs and speed

**Hint 1.** If a program in `programs/` fails, make it smaller until the
failure fits in five lines; then it is an M3 to M5 bug with a name. If a
speed test fails, do not guess: measure. Build with `--release`, run the
test's script by hand under a profiler or with a timer, and find the one
operation that is slow.

**Hint 2.**

- The usual reasons for a very slow tree walker, in order of how often
  they happen: values that copy their contents when cloned; a new hash
  map for every block entry; looking names up by string through a chain of
  hash maps; cloning the function body for each call; unbuffered output.
- The usual reason for a *crash* in M7 is stack depth: `queens.tn` and
  the recursion tests go deeper than toy scripts.
- Measure before and after each change, and keep the numbers (the rubric
  asks for them). An optimisation without a measurement is a rumour.
- When everything passes, read SPEC 13 once more and try the inputs it
  describes as "outside the limits". Decide for each whether your
  interpreter gives an error or dies, and whether you want to fix it.

**Check your explanation.**

- How many heap allocations does one call of `fn id(x) { x }` cost in
  your interpreter? Where are they?
- Which of your data structures would you change first for a bytecode
  design, and which would stay as they are?
