# Concepts for the interpreter capstone

Read this before you start, and come back to a section when you reach the
milestone that needs it.

**What you will be able to do afterwards.** Explain how a program text
becomes behaviour; cut text into tokens with exact positions; parse
operators of any precedence with one small loop; represent a program as a
tree in Rust; explain what a closure is by pointing at who owns what; let
an error, a `return` and a `break` leave a deep computation along the same
road; and decide between walking a tree and running bytecode.

**What is assumed.** Chapters 3 to 11 and 13 to 18 of the course: integers,
ownership and borrowing, `Result` and `?`, structs and enums, traits,
`Rc` and `RefCell`, closures in Rust, iterators, collections, strings as
UTF-8, standard input and output. No knowledge of compilers is assumed.

**None of the examples is Tansy.** They are small languages made up for
one section each, so that the technique is visible and the capstone is
still yours to design. Every `rust` block in this file compiles and runs
as a test: `rustdoc --edition 2024 --test CONCEPTS.md`.

**Time (estimates).** About 20 to 30 minutes per section; three hours for
the whole file if you type in and change the examples, which you should.

| section | read it before |
|---|---|
| 1. From text to behaviour | M1 |
| 2. Lexing: tokens and positions | M1 |
| 3. Pratt parsing: binding power | M1 |
| 4. Trees in Rust | M1 |
| 5. Positions in text | M2 |
| 6. Stopping early | M2, again before M3 |
| 7. Environments and closures | M3, again before M4 |
| 8. Talking to the outside | M6 |
| 9. Tree walk or bytecode | M7 |

---

## 1. From text to behaviour

A program text is a string. A computer cannot do anything useful with
`1 + 2 * 3` as a string, because the structure that a person sees in it
(the multiplication belongs together, the addition comes last) is not in
the characters. An interpreter makes that structure explicit, step by
step, and then acts on it.

```text
 "1 + 2 * 3"          the text
      |  lexer
      v
 1  +  2  *  3        tokens: the words of the language
      |  parser
      v
      +               a tree: who belongs to whom
     / \
    1   *
       / \
      2   3
      |  evaluator
      v
      7               behaviour: a value, output, an error
```

Each step takes one kind of data and gives another. That is the most
important fact about the design of an interpreter: **the stages are joined
only by the data that flows between them.** The lexer does not know what
an expression is. The parser does not know what a value is. You can write,
test and replace each one alone.

Real interpreters add stages in the middle. Two of them matter for this
capstone:

- A *check* (or *resolve*) stage walks the tree once before anything runs.
  It finds mistakes that need no running, such as a name that nobody
  declared, and it can write down facts for the evaluator, such as where a
  name's value will be stored.
- A *compile* stage turns the tree into a flat list of instructions
  (bytecode) for a small virtual machine. Section 9 compares that with
  walking the tree directly.

**When you design the stages,** design the data first. For each arrow ask:
what is in it, who owns it, how long does it live, and what does an error
look like at this point? If you can answer those four questions for every
arrow, the functions almost write themselves.

**Read more.**
[The Rust Reference: lexical structure](https://doc.rust-lang.org/reference/lexical-structure.html)
and [expressions](https://doc.rust-lang.org/reference/expressions.html)
show how a real language specification separates tokens from grammar.

---

## 2. Lexing: tokens and positions

A *lexer* (also *scanner* or *tokenizer*) cuts the text into *tokens*: the
smallest pieces that mean something, such as a number, a name or `<=`.
Spaces and comments disappear here; later stages never see them.

A lexer answers one question again and again: **at this position, what is
the longest token that starts here?** "Longest" matters when one token is
the beginning of another. In the toy language below `*` is multiplication
and `**` is a power operator. A lexer that takes `*` as soon as it sees one
would turn `**` into two multiplications.

The second job of a lexer is to remember **where each token came from**.
Every error message you will ever print needs that. The cheapest position
is a pair of byte offsets into the text, a *span*. Lines and columns can be
computed from offsets later (section 5); the reverse is not possible.

```rust
use std::ops::Range;

#[derive(Debug, PartialEq)]
enum Tok {
    Num(u64),
    Star,
    StarStar,
    Open,
    Close,
}

/// The tokens of `text`, each with the byte range it came from.
/// The error is the byte offset of a character that starts no token.
fn lex(text: &str) -> Result<Vec<(Tok, Range<usize>)>, usize> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        let tok = if let Some(digit) = c.to_digit(10) {
            let mut value = u64::from(digit);
            // Take digits for as long as there are digits: the longest match.
            while let Some(d) = chars.peek().and_then(|&(_, c)| c.to_digit(10)) {
                value = value * 10 + u64::from(d);
                chars.next();
            }
            Tok::Num(value)
        } else {
            match c {
                ' ' => continue,
                '(' => Tok::Open,
                ')' => Tok::Close,
                // Look one character ahead before deciding.
                '*' if chars.next_if(|&(_, next)| next == '*').is_some() => Tok::StarStar,
                '*' => Tok::Star,
                _ => return Err(start),
            }
        };
        // The token ends where the next character starts.
        let end = chars.peek().map_or(text.len(), |&(i, _)| i);
        tokens.push((tok, start..end));
    }
    Ok(tokens)
}

let tokens = lex("2 ** (10*3)").unwrap();
assert_eq!(tokens[1], (Tok::StarStar, 2..4));
assert_eq!(tokens[3], (Tok::Num(10), 6..8));
assert_eq!(tokens[4], (Tok::Star, 8..9));
// `×` is not a token of this language. It starts at byte 2.
assert_eq!(lex("2 × 3"), Err(2));
```

Notice `char_indices`: it gives each character together with its byte
offset, so the code works on characters and still knows byte positions.
`peek` and `next_if` are the "look ahead without taking" tools.

**Mistakes people really make.**

- *No longest match.* Checking for `*` before `**`, or for `<` before
  `<=`. Order the checks so that the longer token is tried first, or look
  ahead as above.
- *Numbers that overflow silently.* `value * 10 + d` above wraps or panics
  for a long enough number. A real lexer uses `checked_mul` and
  `checked_add`, or collects the digits and calls `str::parse`, and
  reports the literal as an error.
- *Forgetting the end of the text.* Every loop that looks ahead must cope
  with "there is no next character". `peek()` returning `None` is the
  normal case at the end, not an exception.

**When to use which.** A lexer can produce all tokens at once (a `Vec`),
or one at a time when the parser asks. All at once is simpler, lets the
parser look ahead as far as it wants, and costs memory in proportion to
the text. One at a time uses constant memory. For source files, which are
small, all at once is the usual choice.

**Read more.**
[`str::char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices),
[`Peekable`](https://doc.rust-lang.org/std/iter/struct.Peekable.html),
[`char::to_digit`](https://doc.rust-lang.org/std/primitive.char.html#method.to_digit).

---

## 3. Pratt parsing: binding power

A *parser* builds the tree. For statements this is straightforward: the
first token says what the statement is (`while`, `let`, ...), and you read
its parts in order. This style is called *recursive descent*: one function
per kind of thing, calling each other.

Expressions are harder, because of precedence. In `1 + 2 * 3` the parser
has read `1 + 2` before it sees the `*` that takes the `2` away from the
addition. The textbook answer is one function per precedence level. It
works, and with eight levels it is eight nearly identical functions.

*Pratt parsing* (named after Vaughan Pratt) replaces them with one loop
and a table. Think of each operator as pulling on its neighbours with a
certain strength, its *binding power*. An operand between two operators
goes to the one that pulls harder:

```text
   1   +   2   *   3
      1 2     3 4          left and right power of each operator

   the 2 sits between a right power of 2 and a left power of 3:
   the * wins, so the tree is 1 + (2 * 3)
```

Every infix operator gets **two** numbers, one for each side. If the right
power is a little higher than the left, `1 - 2 - 3` groups to the left:
the middle `2` is pulled by 2 from the first `-` and by 1 from the second,
so the first one wins. Swap the two numbers and the operator groups to the
right, which is what a power operator wants (`2 ^ 3 ^ 2` is `2 ^ 9`).

The algorithm is one function, `expr(min)`: parse an expression that
consists of operators with a left power of at least `min`.

1. Parse one operand (a number, something in parentheses, or a prefix
   operator followed by `expr(its power)`).
2. Look at the next token. If it is an operator whose left power is at
   least `min`, take it, parse its right operand with
   `expr(right power)`, combine, and repeat step 2. Otherwise stop.

The example language has `+ - * /`, a right-grouping `^`, a prefix `-` and
a postfix `!`. The parser returns the tree as text with full parentheses,
which is a good way to test a parser.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Num(i64),
    Op(char),
    End,
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Tok {
        self.toks.get(self.pos).copied().unwrap_or(Tok::End)
    }

    fn next(&mut self) -> Tok {
        let tok = self.peek();
        self.pos += 1;
        tok
    }

    /// An expression made of operators that bind at least as strongly as
    /// `min`, written out with every pair of parentheses.
    fn expr(&mut self, min: u8) -> String {
        let mut left = match self.next() {
            Tok::Num(n) => n.to_string(),
            // A prefix operator has only a right power: how much of what
            // follows it takes. 5 lies between `*` and `^`.
            Tok::Op('-') => format!("(neg {})", self.expr(5)),
            Tok::Op('(') => {
                let inner = self.expr(0);
                assert_eq!(self.next(), Tok::Op(')'), "missing `)`");
                inner
            }
            other => panic!("an expression cannot start with {other:?}"),
        };
        loop {
            let Tok::Op(op) = self.peek() else { break };
            let (left_power, right_power) = match op {
                '+' | '-' => (1, 2),
                '*' | '/' => (3, 4),
                '^' => (6, 5), // right power lower: groups to the right
                '!' => (9, 0), // postfix: there is no right operand
                _ => break,    // `)` is not an operator
            };
            if left_power < min {
                break;
            }
            self.next();
            left = if op == '!' {
                format!("(fact {left})")
            } else {
                format!("({op} {left} {})", self.expr(right_power))
            };
        }
        left
    }
}

fn parse(text: &str) -> String {
    let toks = text
        .chars()
        .filter(|c| *c != ' ')
        .map(|c| c.to_digit(10).map_or(Tok::Op(c), |d| Tok::Num(i64::from(d))))
        .collect();
    Parser { toks, pos: 0 }.expr(0)
}

assert_eq!(parse("1 + 2 * 3"), "(+ 1 (* 2 3))");
assert_eq!(parse("1 - 2 - 3"), "(- (- 1 2) 3)");
assert_eq!(parse("2 ^ 3 ^ 2"), "(^ 2 (^ 3 2))");
assert_eq!(parse("(1 + 2) * 3 !"), "(* (+ 1 2) (fact 3))");
// The prefix minus is stronger than `*` and weaker than `^`:
assert_eq!(parse("-2 * 3"), "(* (neg 2) 3)");
assert_eq!(parse("-2 ^ 2"), "(neg (^ 2 2))");
```

Follow `-2 ^ 2` by hand. The prefix `-` calls `expr(5)`. That reads `2`,
sees `^` with a left power of 6, which is at least 5, and so takes it.
With `-2 * 3` the inner call sees `*` with a left power of 3, which is less
than 5, and stops; the multiplication is left for the caller. One number
places a prefix operator anywhere between the infix levels.

**Mistakes people really make.**

- *The same number on both sides.* With `(1, 1)` for `-`, the inner call
  takes the second `-` too, and `1 - 2 - 3` becomes `1 - (2 - 3)`, which
  is 2 and not -4. If results of subtraction or division are wrong only
  when there are three operands, look here.
- *Stopping the loop with an error.* When the next token is not an
  operator, or is too weak, that is not a mistake: this call is finished,
  and the token belongs to a caller. Only the place that *needs* a certain
  token (the `)` after a parenthesised expression) may complain.
- *Giving a prefix operator the power of its infix twin.* Unary and binary
  `-` are different operators that share a character. They have different
  powers.

**When to use which.** Recursive descent for statements and for anything
introduced by a keyword. A Pratt loop for expressions with more than two
or three precedence levels. They mix freely: an operand in step 1 can be a
whole `if` parsed by its own function, and that function calls `expr(0)`
for its condition.

**Read more.**
[The Rust Reference: expression precedence](https://doc.rust-lang.org/reference/expressions.html#expression-precedence)
is a precedence table for a real language; note that Rust's comparison
operators "require parentheses", a rule that a table of powers alone does
not express.

---

## 4. Trees in Rust

The tree that a parser builds is called an *abstract syntax tree*, AST for
short: "abstract" because parentheses, commas and semicolons have done
their work and are gone. In Rust the natural type is an `enum` with one
variant per kind of node. A node contains its children, so the type refers
to itself, and the compiler asks a question:

```rust,compile_fail,E0072
enum Expr {
    Number(i64),
    Add(Expr, Expr),
}
```

```text
error[E0072]: recursive type `Expr` has infinite size
 --> expr.rs:1:1
  |
1 | enum Expr {
  | ^^^^^^^^^
2 |     Number(i64),
3 |     Add(Expr, Expr),
  |         ---- recursive without indirection
  |
help: insert some indirection (e.g., a `Box`, `Rc`, or `&`) to break the cycle
```

An `Expr` would have to be large enough to hold two `Expr`s. A `Box` is a
pointer of fixed size to a value on the heap, so a node that *boxes* its
children has a known size. Evaluation is then a function that mirrors the
type: one `match`, one arm per variant, calling itself for the children.

```rust
enum Expr {
    Number(i64),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn eval(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Number(n) => Some(*n),
        Expr::Neg(inner) => eval(inner)?.checked_neg(),
        Expr::Add(a, b) => eval(a)?.checked_add(eval(b)?),
        Expr::Mul(a, b) => eval(a)?.checked_mul(eval(b)?),
    }
}

// 1 + 2 * 3
let tree = Expr::Add(
    Box::new(Expr::Number(1)),
    Box::new(Expr::Mul(Box::new(Expr::Number(2)), Box::new(Expr::Number(3)))),
);
assert_eq!(eval(&tree), Some(7));

let overflow = Expr::Neg(Box::new(Expr::Number(i64::MIN)));
assert_eq!(eval(&overflow), None);
```

```text
 tree (on the stack)        heap
 +-----------+       +-----------+       +-----------+
 | Add  * *  |--+--->| Number 1  |   +-->| Number 2  |
 +-----------+  |    +-----------+   |   +-----------+
                |    +-----------+   |   +-----------+
                +--->| Mul  * *  |---+-->| Number 3  |
                     +-----------+       +-----------+

 each node owns its children: dropping `tree` frees everything
```

Ownership here is as simple as it gets: a tree. Each node has exactly one
owner, its parent, and `eval` only borrows.

A second representation keeps all nodes in one `Vec` (an *arena*) and lets
nodes refer to each other by index. Nothing is boxed, nodes sit next to
each other in memory, and an index is `Copy`, so the borrow checker has
nothing to say about passing it around.

```rust
#[derive(Clone, Copy)]
struct Id(usize);

enum Node {
    Number(i64),
    Add(Id, Id),
}

#[derive(Default)]
struct Arena {
    nodes: Vec<Node>,
}

impl Arena {
    fn add(&mut self, node: Node) -> Id {
        self.nodes.push(node);
        Id(self.nodes.len() - 1)
    }

    fn eval(&self, id: Id) -> i64 {
        match &self.nodes[id.0] {
            Node::Number(n) => *n,
            Node::Add(a, b) => self.eval(*a) + self.eval(*b),
        }
    }
}

let mut arena = Arena::default();
let one = arena.add(Node::Number(1));
let two = arena.add(Node::Number(2));
let sum = arena.add(Node::Add(one, two));
assert_eq!(arena.eval(sum), 3);
```

**Mistakes people really make.**

- *Forgetting that recursion uses the stack.* `eval` calls itself once per
  level of the tree. So does the code that the compiler generates to
  *drop* a tree of boxes. A tree one million levels deep (easy to make
  from a text like `1 + 1 + 1 + ...`) overflows the stack, and that is not
  a panic you can catch:

  ```text
  thread 'main' (126004121) has overflowed its stack
  fatal runtime error: stack overflow, aborting
  ```

  (The number is the thread's id and differs from run to run.)

  The fix is a limit somewhere: on the nesting the parser accepts, on the
  depth the evaluator goes to, or both.
- *Cloning subtrees.* If a function needs a node "again later" (the body
  of a loop, the body of a function), share it (`Rc`) or borrow it. A
  tree-walking interpreter that clones a function's body for every call
  spends most of its time copying.
- *A wildcard arm in `match`.* `_ => ...` in a `match` over your node type
  means that the compiler will not tell you about the places to change
  when you add a variant. Name the variants.

**When to use which.** Boxes are the direct translation of the grammar and
the right start. An arena pays off when the tree is large and walked many
times, when nodes need to refer to each other in more than one direction,
or when you want to attach side tables ("for node 17: its type, its
span"). An index into the wrong arena is a bug the compiler cannot see;
that is the price.

**Read more.**
[`Box`](https://doc.rust-lang.org/std/boxed/index.html),
[`rustc --explain E0072`](https://doc.rust-lang.org/error_codes/E0072.html),
[`Rc`](https://doc.rust-lang.org/std/rc/index.html).

---

## 5. Positions in text

A Rust `&str` is UTF-8: a sequence of bytes in which one character takes
one to four bytes. Three different numbers can describe "where":

| unit | used for | cost to find |
|---|---|---|
| byte offset | slicing (`&text[a..b]`), spans | free: it is what you store |
| character count | columns that a person counts | walk the line once |
| line number | messages | count the line feeds before the offset |

Store byte offsets. Convert when you print. The conversion needs only the
text and the offset:

```rust
/// The 1-based line of the byte at `offset`, and the text of that line.
fn line_of(text: &str, offset: usize) -> (usize, &str) {
    let before = &text[..offset];
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    let number = before.matches('\n').count() + 1;
    let line = text[start..].lines().next().unwrap_or("");
    (number, line)
}

let text = "first\nzażółć gęślą\nthird";
let offset = text.find("gęślą").unwrap();
assert_eq!(line_of(text, offset), (2, "zażółć gęślą"));

// The same position in three units:
let line_start = text.find("zażółć").unwrap();
assert_eq!(offset - line_start, 11); // bytes into the line
assert_eq!(text[line_start..offset].chars().count(), 7); // characters into the line
```

`gęślą` starts 11 bytes into its line and 7 characters into it. An error
message that says "column 12" would point four places too far to the
right.

**Mistakes people really make.**

- *Slicing at a position that is not a character boundary.* This compiles
  and panics when the text is not ASCII:

  ```rust,should_panic
  let text = "żółw";
  let _first = &text[0..1];
  ```

  ```text
  end byte index 1 is not a char boundary; it is inside 'ż' (bytes 0..2 of string)
  ```

  Positions that you got from `char_indices`, `find` or the end of a
  token you lexed are always boundaries. Positions that you *computed*
  (`start + 1`) are not. Add `c.len_utf8()`, never 1.
- *Counting columns with `len()`.* `"żółw".len()` is 7. Use
  `chars().count()`.
- *Forgetting what a "line" is on Windows.* A file with `\r\n` line ends
  has a `\r` at the end of every line you cut at `\n`. `str::lines` drops
  it for you; `split('\n')` does not.

(A character in this sense is a Unicode scalar value, a Rust `char`. What
a reader sees as one character can be several of them, and some characters
are two columns wide on a terminal. Handling that needs tables that the
standard library does not have; the SPEC does not ask for it.)

**Read more.**
[`str`](https://doc.rust-lang.org/std/primitive.str.html) (see
`is_char_boundary`, `char_indices`, `lines`),
[`char::len_utf8`](https://doc.rust-lang.org/std/primitive.char.html#method.len_utf8).

---

## 6. Stopping early

An evaluator is a deep recursion: a statement inside a block inside a
loop inside a function call inside an argument. Several things must be
able to leave from anywhere in that recursion and stop at a specific
place further up:

| what | stops at |
|---|---|
| an error | the top |
| "leave this loop" | the nearest loop |
| "return from this function" | the nearest call |

Rust already has a mechanism for the first: return `Result`, and let `?`
pass the `Err` upwards. The insight is that the others are **the same
mechanism**. They are not errors, but they are "this computation did not
finish in the normal way, and somebody further up knows what to do". Put
all of them in the `Err` side, as variants of one enum. Each construct
looks at what comes up and handles its own kind.

The example is a made-up language of commands with a `Repeat` loop.

```rust
#[derive(Debug, PartialEq)]
enum Stop {
    /// Something went wrong: nobody handles this before the top.
    Error(String),
    /// Leave the innermost `Repeat`.
    Break,
}

enum Cmd {
    Emit(i64),
    Break,
    Fail,
    Repeat(u32, Vec<Cmd>),
}

fn run(cmds: &[Cmd], out: &mut Vec<i64>) -> Result<(), Stop> {
    for cmd in cmds {
        match cmd {
            Cmd::Emit(n) => out.push(*n),
            Cmd::Break => return Err(Stop::Break),
            Cmd::Fail => return Err(Stop::Error("failed".into())),
            Cmd::Repeat(times, body) => {
                for _ in 0..*times {
                    match run(body, out) {
                        Ok(()) => {}
                        Err(Stop::Break) => break,       // ours: handled here
                        Err(other) => return Err(other), // not ours: pass it on
                    }
                }
            }
        }
    }
    Ok(())
}

// Three rounds. In each, the inner loop is left before it can emit 3.
let mut out = Vec::new();
let program = [
    Cmd::Repeat(3, vec![Cmd::Emit(1), Cmd::Repeat(1, vec![Cmd::Emit(2), Cmd::Break, Cmd::Emit(3)])]),
    Cmd::Emit(9),
];
assert_eq!(run(&program, &mut out), Ok(()));
assert_eq!(out, [1, 2, 1, 2, 1, 2, 9]); // the inner Break left only the inner loop

// An error passes through every loop on its way up.
let mut out = Vec::new();
let failing = [Cmd::Repeat(5, vec![Cmd::Emit(1), Cmd::Fail])];
assert_eq!(run(&failing, &mut out), Err(Stop::Error("failed".into())));
assert_eq!(out, [1]);
```

Everything between the `Break` and its loop was written without a thought
about `Break`: the `?` operator (here, the explicit `return Err(other)`)
carries it. That is why this design scales to a real language: a new
construct needs no knowledge of the ways in which its parts can end.

**Mistakes people really make.**

- *A `break` at the top.* What if `Stop::Break` reaches the very top,
  because the program has a `Break` outside of every loop? Either a check
  before the run rules it out, or the top must handle it. Decide which,
  and do not `unwrap`.
- *State that is not restored.* If a function does "depth += 1, run,
  depth -= 1" and leaves through `?` in the middle, the counter stays too
  high. For a program that then ends it does not matter. For an
  interactive session, which goes on after an error, it does. Take the
  result first, restore the state, then return the result.
- *`std::process::exit` deep inside.* It ends the process at once: no
  destructors run, and output that sits in a buffer is lost. Treat "end
  the program" as one more `Stop` variant that only `main` handles.

**When to use which.** `Result` with a "stop" enum for everything that
leaves early *as part of the language*. A panic only for "the interpreter
itself is broken". If you ever want to catch errors inside the language
(`try`), you are already prepared: it is one more construct that handles
one variant.

**Read more.**
[The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator),
[`std::process::exit`](https://doc.rust-lang.org/std/process/fn.exit.html)
(read the note about destructors),
[`std::ops::ControlFlow`](https://doc.rust-lang.org/std/ops/enum.ControlFlow.html),
a standard type for the same idea.

---

## 7. Environments and closures

An *environment* is where an evaluator keeps the values of names. Because
blocks nest, environments nest: each has a link to the one around it, and
a name is looked for in the innermost first.

```text
 let a = 1;                top level:  a = 1
 {                             ^
     let b = 2;            block:      b = 2
     {                         ^
         a + b             inner block (nothing of its own): looks up b, then a
     }
 }
```

If blocks only ever nested, a stack would do: push an environment when a
block starts, pop it when the block ends. Functions as values break this.
A function that is created inside a block can be returned from it and
called later, and it must still be able to use the names of the block
where it was written. Such a function value is called a *closure*: it
"closes over" its environment.

So: **a closure is the code plus a reference to the environment it was
created in.** The environment must live as long as any closure that refers
to it, which can be longer than the block. One owner is no longer enough;
this is shared ownership, `Rc`. And since assignments through any closure
must be visible to all, the contents are shared *and* mutable: `RefCell`.

The example models this by hand, with Rust functions standing in for
interpreted code.

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct Scope {
    vars: RefCell<HashMap<&'static str, i64>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    fn new(parent: Option<Rc<Scope>>) -> Rc<Scope> {
        Rc::new(Scope { vars: RefCell::new(HashMap::new()), parent })
    }

    fn declare(&self, name: &'static str, value: i64) {
        self.vars.borrow_mut().insert(name, value);
    }

    fn get(&self, name: &str) -> Option<i64> {
        let found = self.vars.borrow().get(name).copied();
        found.or_else(|| self.parent.as_ref()?.get(name))
    }

    /// Assigns in the nearest scope that has the name.
    fn set(&self, name: &str, value: i64) -> bool {
        if let Some(slot) = self.vars.borrow_mut().get_mut(name) {
            *slot = value;
            return true;
        }
        self.parent.as_ref().is_some_and(|parent| parent.set(name, value))
    }
}

/// A function value: code, and the scope it was created in.
struct Closure {
    code: fn(&Scope) -> i64,
    scope: Rc<Scope>,
}

impl Closure {
    fn call(&self) -> i64 {
        // A call gets a new scope whose parent is the closure's scope,
        // not the scope of whoever is calling.
        let locals = Scope::new(Some(Rc::clone(&self.scope)));
        (self.code)(&locals)
    }
}

/// Like:  make_counter() { count = 0; return a function that does count += step }
fn make_counter(globals: &Rc<Scope>) -> Closure {
    let block = Scope::new(Some(Rc::clone(globals)));
    block.declare("count", 0);
    fn bump(scope: &Scope) -> i64 {
        let next = scope.get("count").unwrap() + scope.get("step").unwrap();
        scope.set("count", next);
        next
    }
    Closure { code: bump, scope: block }
    // `block` would be gone here if the closure did not hold it.
}

let globals = Scope::new(None);
globals.declare("step", 1);

let a = make_counter(&globals);
let b = make_counter(&globals);
assert_eq!((a.call(), a.call(), b.call()), (1, 2, 1)); // each call of make_counter made its own count

globals.set("step", 10); // both closures see the change: they share `globals`
assert_eq!((a.call(), b.call()), (12, 11));
assert_eq!(Rc::strong_count(&globals), 3); // our variable, and the two blocks
```

```text
 globals: step ------------------------------+
     ^                 ^                     |  one environment,
     | parent          | parent              |  three owners
 block 1: count    block 2: count            |
     ^                 ^
     | scope           | scope
 closure a         closure b
```

This design, a chain of maps looked up by name, is the simplest correct
one. Two refinements are worth knowing; each replaces work at run time by
work done once.

- **Resolve names before running.** The nesting of blocks is visible in
  the text. A pass over the tree can compute, for every use of a name, how
  many environments outwards its declaration is and at which position
  inside that environment. The evaluator then follows that many links and
  indexes into a `Vec`: no strings, no hashing. The same pass is the
  natural place to report names that nobody declared.
- **Capture single variables.** A closure usually needs a few of the
  names around it, not all. A pass can find out which variables are
  captured by some inner function. Those get a shared cell of their own;
  all other variables live in a plain array per call, which is fast and
  freed when the call ends. A closure is then code plus a short list of
  cells:

  ```rust
  use std::cell::Cell;
  use std::rc::Rc;

  let count = Rc::new(Cell::new(0)); // a captured variable: one shared cell
  let for_closure_a = Rc::clone(&count);
  let for_closure_b = Rc::clone(&count);
  for_closure_a.set(for_closure_a.get() + 1);
  assert_eq!(for_closure_b.get(), 1); // both see one variable
  ```

**Mistakes people really make.**

- *Copying where sharing is meant.* A closure that stores a *copy* of the
  map (`vars.borrow().clone()`) works until someone assigns: then the
  closure and the rest of the program disagree about the value. With the
  code above, try it: `a.call()` would return 1 for ever.
- *Running the call in the caller's scope.* If `call` used the caller's
  environment as the parent, a function would see the caller's local
  names. That is called dynamic scope, it makes programs very hard to
  read, and it is almost never what a language wants. The parent is the
  scope where the function was *written*.
- *Holding a `RefCell` borrow while other code runs.* The borrow rules
  that the compiler checks for `&` and `&mut` are checked by `RefCell`
  while the program runs, and a violation is a panic:

  ```rust,should_panic
  use std::cell::RefCell;

  let list = RefCell::new(vec![1, 2, 3]);
  for item in list.borrow().iter() { // this borrow lives for the whole loop
      if *item == 2 {
          list.borrow_mut().push(4); // panics: the list is still borrowed
      }
  }
  ```

  ```text
  RefCell already borrowed
  ```

  In an interpreter "other code" is the user's program, which can do
  anything, including changing the list you are walking. Borrow, copy out
  what you need, end the borrow, and only then run user code. `get` above
  does this on a small scale: the borrow ends before the parent is asked.
- *Cycles.* A closure stored in a variable of its own environment makes a
  cycle of `Rc`s: the environment owns the closure, the closure owns the
  environment. Neither count reaches zero, and the memory is never freed.
  Whether that matters depends on how long the process lives. `Weak` can
  break some cycles; in general it needs a garbage collector.

**When to use which.** The chain of maps to get something working and to
understand the problem. Resolved addresses as soon as you have a pass over
the tree anyway: it is not much code and removes the largest cost of a
tree walker. Captured single variables when calls are the hot spot, or in
a bytecode design, where local variables want to live on the VM's stack.

**Read more.**
[`Rc`](https://doc.rust-lang.org/std/rc/index.html) (the module page
explains cycles and `Weak`),
[`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html),
[closures in the Rust Reference](https://doc.rust-lang.org/reference/types/closure.html)
for how Rust itself captures: per variable, decided at compile time.

---

## 8. Talking to the outside

An interpreter reads a file or lines from standard input and writes to
standard output and standard error. Four facts decide how.

**Write to a trait, not to the terminal.** If the evaluator calls
`println!`, the only way to test it is to start a process and capture its
output. If it writes to something that implements `Write`, a test can hand
it a `Vec<u8>` and look inside afterwards. The same goes for reading and
`BufRead`.

```rust
use std::io::{self, BufRead, Write};

/// Copies lines from `input` to `out`, numbered.
fn number_lines(input: &mut dyn BufRead, out: &mut dyn Write) -> io::Result<()> {
    let mut line = String::new();
    let mut number = 1;
    while input.read_line(&mut line)? != 0 {
        writeln!(out, "{number}: {}", line.trim_end_matches(['\r', '\n']))?;
        number += 1;
        line.clear();
    }
    Ok(())
}

// In a test: a byte slice is a reader, a `Vec<u8>` is a writer.
let mut out = Vec::new();
number_lines(&mut "one\r\ntwo".as_bytes(), &mut out).unwrap();
assert_eq!(out, b"1: one\n2: two\n");

// In `main` it would be:
//     number_lines(&mut io::stdin().lock(), &mut io::stdout().lock())
```

**Buffered output is fast, and somebody must flush it.** Each write to the
real standard output can be a system call. Wrapping the handle in a
`BufWriter` collects small writes and sends them in blocks. The price: the
data is in your process until the buffer is full or `flush` is called. So
flush before the process ends, before you wait for input (a prompt that
sits in a buffer is not a prompt), and before you write to the other
stream if the order matters to a reader.

**Use one reader for one stream.** A `BufReader` reads ahead: it takes
more bytes from the stream than the line you asked for and keeps the rest.
Two buffered readers on the same standard input steal each other's lines.
Create one and pass it to everything that reads.

**Writing can fail, and `println!` panics when it does.** The usual case
is a pipe whose reader has gone away, as in `yourprogram | head -1`:

```text
failed printing to stdout: Broken pipe (os error 32)
```

That line is the message of the panic. With `writeln!(out, ...)` you get
an `Err` and can end the program in an orderly way. (The text is what
macOS and Linux report; on Windows the error has another number and
wording.)

**One more thing: the size of the stack.** A recursive evaluator turns the
depth of the interpreted program into depth of the Rust call stack, and
frames of an unoptimised build are large. The main thread's stack has a
fixed size chosen by the system (often 8 MiB; 1 MiB on Windows). A thread
that you start can have any size:

```rust
use std::thread;

fn depth(n: u64) -> u64 {
    if n == 0 { 0 } else { 1 + depth(n - 1) }
}

let worker = thread::Builder::new()
    .stack_size(64 * 1024 * 1024) // reserved, not used, until the stack grows
    .spawn(|| depth(100_000))
    .expect("the system can start a thread");
assert_eq!(worker.join().expect("no panic"), 100_000);
```

A larger stack moves the limit; it does not remove it. You still need a
limit of your own that is reached first and that reports an error, because
running out of stack is not an error you can catch (section 4).

**Read more.**
[`Write`](https://doc.rust-lang.org/std/io/trait.Write.html),
[`BufRead`](https://doc.rust-lang.org/std/io/trait.BufRead.html),
[`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html),
[`thread::Builder::stack_size`](https://doc.rust-lang.org/std/thread/struct.Builder.html#method.stack_size),
[`ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html).

---

## 9. Tree walk or bytecode

There are two classic ways to run the tree.

A **tree-walking evaluator** is the `eval` of section 4, grown up: a
recursive function over the tree that returns values. The tree *is* the
program.

A **bytecode virtual machine** adds a stage. A compiler walks the tree
once and emits a flat array of small instructions for an imaginary
machine, usually a stack machine. A loop then runs the instructions.

```text
 1 + 2 * 3     compiles to        the VM's stack while it runs

 PUSH 1                           [1]
 PUSH 2                           [1, 2]
 PUSH 3                           [1, 2, 3]
 MUL                              [1, 6]
 ADD                              [7]
```

The order of the instructions is the order in which a tree walk would
finish the nodes. The structure has gone into the order, and running the
program needs no recursion.

| | tree walk | bytecode VM |
|---|---|---|
| code to write | least: one function per node kind | a compiler and a VM; control flow becomes jumps with offsets to patch |
| speed | each node costs a `match`, a function call and often a pointer to follow | a tight loop over an array; several times faster is typical |
| native stack | grows with the depth of the interpreted program | constant: the VM keeps its own stack on the heap |
| errors with positions | easy: the node is at hand | needs a table from instruction to source position |
| closures | environments (section 7) are natural | local variables live on the VM stack, so captured ones need the "single cell" treatment of section 7 |
| debugging the interpreter | you step through code that looks like the language | you read dumps of instructions; write a disassembler first |

**When to use which.** Walk the tree first. It is the shortest road to a
correct language, and correctness is what the acceptance tests check. A
tree walker that resolves names to addresses beforehand and shares instead
of copying is fast enough for a great many uses. Go to bytecode when you
have measured that the walk itself is the cost, or when you need what a VM
gives besides speed: no native recursion, a program you can save, a place
to pause and resume. If you do, keep the tree walker: two implementations
that must agree on every test are the best test of both.

**Read more.**
[`std::hint::black_box`](https://doc.rust-lang.org/std/hint/fn.black_box.html)
and the performance module of this course (Mastery M7) before you measure
anything;
[`std::mem::size_of`](https://doc.rust-lang.org/std/mem/fn.size_of.html)
to find out how large your value and instruction types really are.
