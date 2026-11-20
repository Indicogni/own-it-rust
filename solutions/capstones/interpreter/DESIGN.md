# Tansy: design notes for the reference solution

This is one way to build the interpreter capstone: a tree-walking evaluator
over a tree whose names were resolved to addresses beforehand, about 2900
lines of Rust plus 740 lines of unit tests. It passes every acceptance
test in debug and release, has no `unsafe` and no dependencies, and is
meant to be read. These notes explain the decisions that are not obvious
from the code, with the alternatives, and end with what I would change.
Your solution may differ in every one of them and still be good.

The files in this directory are an overlay: copy `src/` over
`capstones/interpreter/src/` and the package builds as the solution.
`Cargo.toml`, the tests and the programs are the starter's, unchanged.

## The map of the code

```text
text --lexer--> tokens --parser--> tree --resolve--> tree + addresses --interp--> effects

src/source.rs     Span, Diagnostic, and the one function that prints a diagnostic
    lexer.rs      text -> tokens; "does this entry need another line?"
    ast.rs        the tree: Expr, Stmt, Block, FnDecl
    parser.rs     tokens -> tree: recursive descent for statements, Pratt for expressions
    resolve.rs    the checks of SPEC 6.4; every name becomes (depth, slot)
    value.rs      Value, equality, display and quoted form, Env
    builtins.rs   names and arities of the nine built-in functions
    interp.rs     the evaluator
    session.rs    parse + resolve + evaluate on state that outlives one run
    cli.rs        arguments, the script runner, the REPL loop, exit statuses
    lib.rs        module list, the stack size, the large-stack helper
    main.rs       real stdin/stdout/stderr, and nothing else
```

The arrows point one way. `lexer` knows `source`. `parser` knows `lexer`
and `ast`. `resolve` knows `ast`. `interp` knows `ast` and `value`. Only
`session` knows all stages, and only `cli` knows about prompts and exit
statuses. The whole driver works on `&mut dyn Write` and
`&mut dyn BufRead`, so the unit tests in `cli.rs` run complete REPL
sessions without a process.

## One error type, classified by who returns it

`Diagnostic` is a span and a message. All three kinds of error use it.
Which kind it is, and so the exit status, is not stored in it: a
diagnostic that comes back from the parser or the resolver is wrapped in
`Failure::Compile`, one from the evaluator in `Failure::Runtime`
(`session.rs`). The kind follows from where the error came from, so it
cannot be wrong, and nobody looks at message text to find it out.

`Source::render` is the only code that knows about lines, columns, tabs
and carets. It scans the text for line feeds each time it is called. That
is linear in the size of the text, and it is called once per run (once per
entry in the REPL), so a table of line starts would be code without a
benefit.

## The lexer never fails

A lexical error becomes a token, `TokenKind::Invalid(LexError)`, and
lexing stops there. The parser reports it if and when it reaches that
token: `Parser::unexpected` turns an `Invalid` token into its own message
instead of "expected X, found Y". This gives SPEC 11.3 ("the error that
comes first in the text wins") for free. A lexer that returned `Err` would
report a stray `@` on line 90 before the missing parenthesis on line 3.

All tokens are produced before parsing starts. The alternative, a lexer
that the parser pulls from, saves a `Vec` and costs a lookahead buffer;
the statement parser needs two tokens of lookahead (`fn` followed by a
name or not). For source files the `Vec` is the simpler trade.

`tokenize` takes a start offset. A script is lexed from 0; a REPL entry
from where it begins in the session's text (see "The REPL" below).

## The parser

Statements are recursive descent; `Parser::sequence` is a direct
transcription of the five rules of SPEC 4.3. Expressions are a Pratt loop
(`Parser::expr`) driven by the function `infix`, which is the precedence
table of SPEC 4.2 as data. Two rules of the language do not fit a plain
table of left-associative operators; each is one place in the code:

- **`not`** is a prefix operator with a power (5) *below* the comparison
  and arithmetic operators. `Parser::prefix` accepts it only when the
  caller's minimum power allows (`min_power <= 5`). That one condition is
  why `not a == b` parses as `not (a == b)` and why `1 + not x` is a
  syntax error, as the grammar says.
- **Comparisons do not chain.** They have a higher right power than left,
  like every left-associative operator, so the right operand stops before
  a second comparison operator. The loop would then happily build
  `(a < b) < c`; one check after the right operand reports the second
  operator instead.

**Spans without a node for parentheses.** SPEC 11.4 wants the span of
`(a + b) * c` to start at the parenthesis, and the span of the sum inside
to exclude it. `Parser::expr` remembers the first token it looked at and
the parser tracks the end of the last token it consumed; every node built
in the loop spans from the one to the other. The parentheses of an operand
are inside that range because they were consumed; the parentheses around
the expression itself are consumed by the caller, afterwards. A
parenthesised expression is therefore returned as it is, no node, no extra
level for the evaluator.

**Nesting.** Three counters bound the recursion of the parser and the
depth of the tree it can build: open delimiters (100, exact, SPEC 13.1),
pending prefix operators and `if`s (1000), and operators applied at one
level (1000). The last two are the reference's choice inside the freedom
SPEC 13.2 gives. Without them a file of 500 000 `-` characters would
overflow the stack in the parser, and `1 + 1 + ...` would build a tree
that cannot be dropped.

## One tree, annotated in place

The resolver does not build a second tree. The parser leaves every `Addr`
and every `slots` count at zero, and `resolve.rs` fills them in through
`&mut`. Alternatives I considered:

- *Two tree types* (parsed, resolved). Type-safe: an unresolved name
  cannot reach the evaluator. But it doubles the node definitions and adds
  a pass that copies everything. For a language of this size the in-place
  version is easier to read; for a compiler with more passes I would take
  two types or side tables.
- *`Cell<Addr>` in the nodes*, so that the resolver needs only `&`. It
  works and hides the mutation; I prefer the signature to say `&mut`.
- *Side tables keyed by node id*. The right tool with an arena tree.

The cost of the chosen design is one invariant that the types do not
express: function bodies are `Rc<FnDecl>` (closures share them and can
outlive the tree of a REPL entry), and the resolver mutates through
`Rc::get_mut`. That is sound because nothing clones the `Rc` between the
parser and the resolver; the evaluator, the only thing that does, runs
later. The `unreachable!` in `Walk::function` states this.

## Names: declared first, then resolved

SPEC 6.2 gives a name to its whole block. `Walk::scoped` therefore
declares all names of a block (parameters or loop variable first, then
every `let`, `var` and `fn` among the block's direct statements) before it
looks at the first statement, and gives each a slot number.

A use of a name becomes `Addr { depth, slot }`: go `depth` environments
outwards, take slot `slot`. A block that declares nothing gets no
environment at run time (`Block::slots == 0`), and `Walk::find` skips such
scopes when it counts depth. The unit test
`addresses_skip_blocks_without_bindings` exists because this is the
easiest thing here to get wrong: the resolver and the evaluator must agree
exactly on which blocks have an environment.

The walk collects every name error and `resolve` returns the one that
starts first in the text. Declaring a block's names up front means a
duplicate declaration at the end of a block is *found* before an unknown
name at its start; sorting is simpler than making the walk find errors in
text order.

"Uninitialized" is `None` in a slot of type `Option<Value>`. `Value` has
spare bit patterns, so the `Option` costs no space.

## Environments

```text
 Env { slots: RefCell<Vec<Option<Value>>>, parent: Option<Rc<Env>> }

 builtins  <-  top level  <-  body of make_counter  <-  body of the closure
                                    ^
                                    +-- Closure { decl: Rc<FnDecl>, env: Rc<Env> }
```

Every block entry that has bindings allocates one `Env`. A closure holds
the `Rc<Env>` that was current where its `fn` was evaluated. That is the
whole implementation of closures, of "each entry makes new bindings" and
of "a call runs in the scope where the function was written".

A call of a function with parameters costs two allocations, the `Vec` of
slots and the `Rc<Env>`. Arguments are evaluated straight into that `Vec`
(parameters are the first slots), so there is no separate argument list.
A call of a function without parameters or declarations allocates nothing.

The alternative is the one real compilers use: find out in the resolver
which variables are captured by an inner function, give only those a
shared cell, and keep all others in one flat array per call. It removes
the environment chain and most allocations, and it is what a bytecode VM
needs. It also makes the resolver two or three times as large. I kept the
chain because it is the design a first implementation arrives at, done
carefully, and the speed is sufficient (numbers below).

Environments that contain a closure over themselves are reference cycles
and are never freed: every recursive function at a block level, for one.
SPEC section 14 allows it; the process ends and the system takes the
memory back. A long-running embedding would need a cycle collector.

The leak is visible to tools. `cargo +nightly-2026-11-20 miri test --lib` (the unit
tests run whole Tansy programs in-process; the three tests that need the
256 MiB stack are best skipped there) finds no undefined behaviour, which
is expected without `unsafe`, and reports exactly these cycles as leaked
memory. With `MIRIFLAGS=-Zmiri-ignore-leaks` it passes.

## Values

```text
 enum Value { Nil, Bool, Int(i64), Str(Rc<str>), List(Rc<RefCell<Vec<Value>>>),
              Fn(Rc<Closure>), Builtin(Builtin) }           24 bytes
```

Cloning a value is at most one reference-count increment. Strings never
change, so `Rc<str>` needs no `RefCell`; lists are shared and mutable
(SPEC 5.2), which is exactly `Rc<RefCell<Vec<_>>>`.

No `RefCell` borrow is ever held while Tansy code runs, because Tansy code
can reach the same list. The three places where that takes care:

- `for` over a list borrows for one `get(i).cloned()` per round;
- assignment to an element evaluates target, index and value first, then
  borrows, swaps the element in, ends the borrow and only then drops the
  old element;
- `Env::set` does the same for bindings.

Printing and comparing borrow for longer but run no Tansy code.
`write_value` keeps the lists it is in the middle of writing and prints
`[...]` for one it meets again. `equal` does the same with pairs of lists
and treats a pair that is already being compared as equal; the SPEC does
not ask for that (it allows the comparison of two cyclic lists not to
end), but ending is cheap.

Strings are indexed by character and stored as UTF-8, so `s[i]` and
`len(s)` walk the string. A loop that indexes a long string is quadratic.
`for c in s` is linear, and that is what the example programs use.

## One road for everything that stops early

```text
 enum Unwind { Error(Diagnostic), Return(Value), Break, Continue, Exit(u8), Output(io::Error) }
```

Every evaluating function returns `Result<_, Unwind>`. A call turns
`Return` into its value; a loop handles `Break` and `Continue`; `session`
maps the rest to a `Failure`. `exit` is a variant, not
`std::process::exit`, so the output buffer is flushed by the same code
that runs at a normal end. A failed write to standard output is a variant
too: SPEC 2.4 is three lines in `cli.rs`.

Bookkeeping is restored structurally: `Interp::in_env` is the only place
that changes the current environment, and it restores it after running its
body whatever the result; the call and depth counters are decremented on
the line after the fallible operation, not before a `return`. `Interp::run`
asserts (in debug builds) that all three are back to their starting
values, which is what makes the REPL safe to continue after an error.

## Depth: limits that are measured, not hoped for

The evaluator is recursive, so the interpreted program's nesting becomes
native stack. Three things keep that from crashing:

1. `main` runs everything on a thread with a 256 MiB stack
   (`with_large_stack`). The memory is reserved address space; pages are
   used only when the stack grows.
2. `MAX_CALL_DEPTH` (1000) is the limit the SPEC demands.
3. `MAX_EVAL_DEPTH` (50 000) bounds the nesting of `Interp::eval` itself,
   for programs that nest absurdly inside each call.

The third number is derived from measurements of an unoptimised build on
aarch64 (a temporary binary with the limits lifted, a 16 MiB stack, and a
search for the largest input that survives):

| what nests | stack per level |
|---|---|
| parser, prefix operator | 4.3 KiB |
| parser, parenthesis | 6.9 KiB |
| parser, `if` in a condition | 8.9 KiB |
| resolver, any expression | 0.34 KiB |
| dropping the tree | 0.14 KiB |
| evaluator, unary operator or block | 1.5 KiB |
| evaluator, call nested in an argument | 2.6 KiB |
| one ordinary recursive Tansy call (`down(n - 1)` in an `if`) | 5.3 KiB |

So: the parser recurses at most 100 + 1000 levels, under 10 MiB. The
deepest tree the parser can build is about 100 000 levels (100 delimiters,
each around a chain of 1000 operators); resolving it takes about 35 MiB
and dropping it 15 MiB. The evaluator at 50 000 levels takes at most
130 MiB. All of it fits twice. 1000 ordinary calls take about 5 MiB, which
is why a debug build would already be in danger on a 1 MiB main thread.

The unit test `deep_recursion_is_an_error_not_a_crash` runs the worst
shapes the parser lets through and expects `stack overflow` as a
diagnostic. If someone changes the evaluator so that frames grow, that
test is what notices.

An alternative to counting is to compare the address of a local variable
with the address recorded at thread start, and so measure the real stack
use. It needs no calibration but relies on how the platform lays out
stacks; a counter with a measured cost is easier to defend.

**Values nest too, and there nothing counts.** A value can be deep without
any deep expression: `l = [i, l]` in a loop builds a linked list, one list
inside the next. Writing such a value (`write_value`), comparing it
(`equal_inner`) and freeing it (the drop code that the compiler generates
for `Rc<RefCell<Vec<Value>>>`) are recursive Rust functions, one level per
link, and the reference has no counter in them. SPEC 13.5 asks for chains
of 1000 links; the acceptance test `lists_nested_one_thousand_deep_work`
checks that. Measured in an unoptimised build on the 256 MiB stack:
printing and comparing work for 100 000 links and fail at 300 000;
freeing works for 400 000 and fails at 600 000. "Fail" means the process
aborts with a stack overflow, which is not the diagnostic that SPEC 13.5
wishes for. The margin over the specified limit is a factor of 100, and I
left it at that. See "What I would do differently".

## The REPL

`Session` owns what survives from one run to the next: the resolver (the
top-level names and their slots) and the evaluator (the top-level
environment, the input and output handles). `cli::repl` is a loop around
`Session::run_from`.

- **One text for the whole session.** The REPL appends every source line
  to one `String` and runs each entry "from offset N of that text". All
  spans are offsets into the same text, so a diagnostic for a function
  typed ten entries ago finds its line, and the session-wide line numbers
  of SPEC 12.5 fall out of the ordinary rendering code. The alternative,
  a source id in every span plus a list of sources, is more machinery for
  the same result.
- **A failed entry leaves no names.** `Resolver::resolve` copies the map
  of top-level names before the walk and puts the copy back if there is an
  error. Copying a small map per entry is cheaper than tracking what to
  undo.
- **Declaring a top-level name again** (allowed across entries, SPEC 12.4)
  reuses the slot. That is why code from earlier entries sees the new
  value, and why the binding keeps its old value until the new declaration
  runs: nothing happens to the slot until then.
- **One reader.** The REPL reads its lines through `Session::read_line`,
  which is the same buffered reader that `input()` uses. Two readers on
  one stream would steal each other's lines.
- **Is the entry complete?** `lexer::needs_more_input` counts delimiter
  tokens. Reusing the lexer means strings and comments are handled by
  definition.

## Where a panic could come from

The crate forbids `unsafe`. Outside the tests, these are all the places
that can panic, each with the reason it does not:

| place | relies on |
|---|---|
| `Walk::function`: `unreachable!` after `Rc::get_mut` | no closure exists before the resolver has run |
| `Env::ancestor`: `expect` on the parent link | the resolver counted depth over the same scopes the evaluator materialises |
| `Env::get`, `set`, `is_initialized`: indexing by slot | slot numbers come from the resolver, and `Block::slots` sized the `Vec` |
| `Parser::peek` and friends: `tokens[pos]` | the token list always ends with `Eof`, and `advance` never moves past it |
| slicing the source by a token's span | spans are made by the lexer on character boundaries of this text |
| `Walk`: `scopes[1]` | the two outermost scopes are pushed in `resolve` and never popped |
| `Interp::builtin`: `values[index]`, `exprs[index]` | called only from patterns that matched that many arguments |
| `Parser::close`, `leave_prefix`: `-= 1` on a counter | each follows the matching increment on the same path |

The first three are the price of annotating the tree in place; with two
tree types the compiler would check them.

## Speed

Processor time on the Apple silicon laptop this was written on:

| program | debug | release |
|---|---|---|
| `fib(27)`, 832 000 calls | 0.23 s | 0.05 s |
| one million rounds of `sum = sum + i % 7; i = i + 1` | 0.35 s | 0.06 s |

Nothing was tuned beyond the decisions above: addresses instead of name
lookups, no environment for blocks without bindings, arguments evaluated
into the callee's slots, cheap value clones, block-buffered output.

## What I would do differently

- **Box the large statement variants.** `Stmt` is 224 bytes because the
  `for` variant holds two expressions and a block inline; `Expr` is 72.
  Boxing `If`, `For` and `While` would shrink every statement list.
- **Intern names.** Each occurrence of an identifier allocates its own
  `Rc<str>`. After resolution, names are needed only for error messages.
- **Capture single variables** (see "Environments") if calls ever became
  the bottleneck, or as the first step toward a bytecode VM.
- **Two tree types**, if the language grew another pass.
- **A character index for long strings**, or strings stored as
  `Rc<[char]>`, if scripts indexed strings in loops.
- **Free and print deep values without recursion.** A `Drop`
  implementation for a list type of my own could move the nested lists it
  is the last owner of onto a work list (`Rc::into_inner` says whether it
  is) and free them in a loop, and `write_value` and `equal_inner` could
  count their depth and report `stack overflow`. Then a linked list of a
  million elements would end with a diagnostic or simply work.
- The message for a source text over 4 GiB (`Span` offsets are `u32`) is
  outside the SPEC's list of messages. It is also outside its limits.
