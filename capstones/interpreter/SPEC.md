# Tansy: the specification

Tansy is a small scripting language made for this capstone. You write its
interpreter: a program called `tansy` that runs Tansy scripts and offers an
interactive session.

This document is the contract. The acceptance tests in `tests/` check what
is written here and nothing else. Where the text says "must", a test may
depend on it. Section 13 says where the contract ends.

A first taste:

```tansy
// Functions are values, and they remember where they were made.
fn make_counter(start) {
    var count = start;
    fn() { count = count + 1; count }
}

let next = make_counter(10);
next();
print("second call:", next());

let words = ["tansy", "is", "small"];
var longest = "";
for word in words {
    if len(word) > len(longest) { longest = word; }
}
print(longest, words[2][0], [len(words), nil, "text"]);
```

```text
second call: 12
tansy s [3, nil, "text"]
```

The language in one paragraph: values are `nil`, booleans, 64-bit integers,
strings, lists and functions. Bindings are made with `let` (fixed) and `var`
(assignable) and live in blocks. Blocks and `if` are expressions; the last
expression of a block, written without `;`, is its value. Functions are
first-class and close over the bindings around them. Types are checked while
the program runs, names before it starts. Every error has a position in the
source.

Contents: [2 command line](#2-the-command-line) ·
[3 lexical structure](#3-lexical-structure) · [4 grammar](#4-grammar) ·
[5 values](#5-values) · [6 bindings and scope](#6-bindings-and-scope) ·
[7 expressions](#7-expressions) · [8 statements](#8-statements) ·
[9 functions](#9-functions-and-closures) ·
[10 built-in functions](#10-built-in-functions) · [11 errors](#11-errors) ·
[12 REPL](#12-the-repl) · [13 limits](#13-limits) ·
[14 non-goals](#14-non-goals) · [15 milestones](#15-milestones) ·
[16 stretch goals](#16-stretch-goals)

---

## 2. The command line

### 2.1 Invocation

| command | what happens |
|---|---|
| `tansy` | starts the REPL (section 12) |
| `tansy SCRIPT` | runs the script in the file `SCRIPT` |
| `tansy --help`, `tansy -h` | writes a help text to standard output and exits with status 0. The first line of the text is `usage: tansy [script]`; the rest is up to you. |
| anything else | a usage error |

"Anything else" is: more than one argument (also `tansy --help extra`), or
one argument that starts with `-` and is not `-h` or `--help` (also a
single `-`). A usage error writes
`usage: tansy [script]` as the first line of standard error, writes nothing
to standard output, and exits with status 3.

The name of the script file does not matter; `.tn` is only a habit.

If the script cannot be read (it does not exist, it is a directory, it is
not valid UTF-8, ...), `tansy` writes one line to standard error that starts
with `error: cannot read ` followed by the path exactly as it was given.
The rest of that line is up to you (the system's reason is a good choice).
The exit status is 3.

### 2.2 Exit status

| status | meaning |
|---|---|
| 0 | the script ran to its end, or the REPL reached the end of its input |
| 1 | a runtime error stopped the script |
| 2 | the script has a syntax error or a name error; nothing was run |
| 3 | usage error, unreadable script, or output that cannot be written |
| *n* | the script called `exit(n)` (section 10); *n* is 0 to 125 |

### 2.3 Output

- What the program prints goes to standard output, as UTF-8, with a line
  feed (`\n`, never `\r\n`) where this document says "line".
- Diagnostics (section 11) go to standard error.
- When the process exits, for whatever reason, everything the program
  printed before must have arrived on standard output. You may buffer, but
  you must flush: before exiting, before a diagnostic is written, before
  `input()` reads, and before the REPL reads a line.

### 2.4 When output cannot be written

If writing to standard output fails (for example because the reader of a
pipe has gone away, as in `tansy big.tn | head -1`), the interpreter stops
at once, writes a line that starts with `error: cannot write output` to
standard error, and exits with status 3. It must not panic.

---

## 3. Lexical structure

### 3.1 Source text

A source text is UTF-8. A *line* ends at a line feed (U+000A) or at the end
of the text. A carriage return (U+000D) directly before a line feed belongs
to the line end, not to the line. Lines are numbered from 1.

A carriage return anywhere else does not end a line: between tokens it is
whitespace (3.2), and in a string literal it is a character of the string
(3.5). How a diagnostic shows a line that contains one is not specified.

A *column* is the position of a character in its line, counted in
characters (Unicode scalar values, what Rust calls `char`), from 1. A tab
is one character and therefore one column.

### 3.2 Whitespace and comments

Space, tab, carriage return and line feed separate tokens and mean nothing
else. A line end has no meaning of its own: an expression can run over
several lines.

A comment starts with `//` and ends at the end of the line. Comments are
whitespace. (Inside a string literal, `//` is ordinary text.)

### 3.3 Names and keywords

A name starts with an ASCII letter or `_` and continues with ASCII letters,
ASCII digits and `_`. Names are case-sensitive. These seventeen words are
keywords and cannot be names:

```text
let  var  fn  if  else  while  for  in  return  break  continue
and  or  not  true  false  nil
```

The names of the built-in functions (`print`, `len`, ...) are ordinary
names, not keywords.

### 3.4 Integer literals

One or more ASCII digits, read as a decimal number. Leading zeros are
allowed and mean nothing (`007` is 7). The value must be at most
9223372036854775807; a larger literal is the error
`integer literal is too large`.

There is no negative literal. `-5` is the operator `-` applied to `5`.

### 3.5 String literals

A string literal starts with `"`, ends with the next `"` that is not part
of an escape sequence, and must end on the line where it starts. Between
the quotes any character stands for itself, except `\`, which starts one of
these escape sequences:

| written | means |
|---|---|
| `\n` | line feed (U+000A) |
| `\t` | tab (U+0009) |
| `\r` | carriage return (U+000D) |
| `\0` | the character U+0000 |
| `\\` | one backslash |
| `\"` | one double quote |

Reading a string from left to right, the first of these problems is the
error that is reported:

- A `\` followed by any other character: `invalid escape sequence '\X'`,
  where `X` is that character. The span is the backslash and the character.
- The end of the line or of the text is reached (also directly after a
  `\`): `unterminated string`. The span is the opening quote.

### 3.6 Punctuation and the longest match

```text
(  )  [  ]  {  }  ,  ;  ..
+  -  *  /  %  =  ==  !=  <  <=  >  >=
```

At each position the longest token that fits is taken: `<=` is one token,
not `<` and `=`. A character that cannot start any token (outside strings
and comments) is the error `unexpected character 'X'`. That includes every
character that is not ASCII, and `!` and `.` when they are not followed by
`=` and `.`.

---

## 4. Grammar

### 4.1 The grammar

`{ x }` means zero or more `x`, `[ x ]` means zero or one. NAME, INT and
STRING are the tokens of section 3.

```text
program    = sequence
block      = "{" sequence "}"
sequence   = { statement } [ expression ]          (exact rule: 4.3)

statement  = ";"
           | ( "let" | "var" ) NAME "=" expression end
           | "fn" NAME function
           | expression "=" expression end         (the left side: 8.1)
           | "return" [ expression ] end
           | "break" end
           | "continue" end
           | "while" expression block
           | "for" NAME "in" expression [ ".." expression ] block
           | if
           | block
           | expression ";"
end        = ";"   or nothing, when "}" or the end of input follows

function   = "(" [ NAME { "," NAME } [ "," ] ] ")" block

expression = or
or         = and { "or" and }
and        = not { "and" not }
not        = "not" not | comparison
comparison = sum [ ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) sum ]
sum        = product { ( "+" | "-" ) product }
product    = unary { ( "*" | "/" | "%" ) unary }
unary      = "-" unary | postfix
postfix    = primary { "(" [ items ] ")" | "[" expression "]" }
items      = expression { "," expression } [ "," ]
primary    = INT | STRING | "true" | "false" | "nil" | NAME
           | "(" expression ")"
           | "[" [ items ] "]"
           | "fn" function
           | if
           | block
if         = "if" expression block [ "else" ( if | block ) ]
```

### 4.2 Operator precedence

The grammar above says the same as this table, from weakest to strongest
binding:

| level | operators | grouping |
|---|---|---|
| 1 | `or` | left to right |
| 2 | `and` | left to right |
| 3 | `not` (prefix) | |
| 4 | `==` `!=` `<` `<=` `>` `>=` | cannot be chained |
| 5 | `+` `-` | left to right |
| 6 | `*` `/` `%` | left to right |
| 7 | `-` (prefix) | |
| 8 | call `f(...)`, index `a[...]` | left to right |

Three consequences are worth spelling out.

- `not` binds weaker than comparison: `not a == b` means `not (a == b)`.
  For the same reason `not` cannot be an operand of an arithmetic or
  comparison operator: `1 + not x` and `a == not b` are syntax errors
  (`expected expression, found` `` `not` ``). Write `a == (not b)`.
- Comparisons do not chain. After `a < b`, another comparison operator is
  the syntax error `comparison operators cannot be chained`, reported at
  that second operator. Write `(a < b) == c` or `a < b and b < c`.
- Unary minus binds tighter than `*`: `-a * b` is `(-a) * b`. Call and
  index bind tighter still: `-f(x)[0]` is `-((f(x))[0])`.

```tansy
print(1 + 2 * 3, (1 + 2) * 3, 10 - 4 - 3, 2 - -1);
print(not 1 == 2, true or true and false, 1 + 1 == 2 and not false);
```

```text
7 9 3 3
true true true
```

### 4.3 Statements, `;` and the value of a sequence

A sequence (a program or the inside of a block) is read one element after
another, until the token that ends the sequence: `}` for a block, the end
of input for a program. The first token of an element decides what it is:

1. `;` is an empty statement.
2. `let`, `var`, `return`, `break`, `continue`, `while`, `for` start the
   statement of that name.
3. `fn` followed by a NAME starts a function declaration. (`fn` followed
   by anything else starts an expression: rule 5.)
4. `if` and `{` start an `if` expression or a block that is **a statement
   of its own: it ends at its closing `}`**. Nothing continues it, no
   operator and no call. If the sequence ends directly after it, it is the
   *tail* of the sequence; otherwise its value is thrown away.
5. Anything else starts an expression. After the expression:
   - `=` makes the element an assignment (8.1);
   - `;` ends an expression statement, whose value is thrown away;
   - if the sequence ends here, the expression is the *tail*;
   - `}` or the end of input also end the statement where they do not end
     the sequence (a `}` at the top level, the end of input inside a
     block); the next element then fails with the error of 11.5;
   - any other token is the error ``expected `;` ``.

Where the grammar says `end`, a `;` is consumed if it is there; nothing is
needed if `}` or the end of input follows; any other token is the error
``expected `;` ``. So the last statement of a block or of the input never
needs its `;`.

**The value of a sequence** is the value of its tail, or `nil` if it has no
tail. The value of a script's top-level sequence is ignored; the REPL
prints it (12.3).

```tansy
let a = { let x = 20; x + 1 };       // tail `x + 1`: the block is 21
let b = { let x = 20; x + 1; };      // no tail: nil
let c = if a > 5 { "big" } else { "small" };
print(a, b, c)                       // no `;` needed before the end
```

```text
21 nil big
```

Rule 4 has a consequence that surprises at first: a statement that starts
with `if` or `{` is not the left operand of what follows.

```tansy
if true { 1 } else { 2 }
-3;                 // a second statement: the expression `-3`
print("two statements");
```

```text
two statements
```

`{ 1 } + 2;` is therefore a syntax error (``expected expression, found `+` ``).
In the middle of an expression the rule does not apply:
`let v = if c { 1 } else { 2 } + 10;` adds 10 to the value of the `if`.

---

## 5. Values

### 5.1 Types

| type name | values |
|---|---|
| `nil` | the single value `nil` |
| `bool` | `true`, `false` |
| `int` | the integers from -9223372036854775808 to 9223372036854775807 |
| `str` | a sequence of Unicode characters (scalar values); cannot be changed |
| `list` | a sequence of values of any types; can be changed |
| `fn` | a function: one you wrote, or a built-in one |

The type name is what `type(v)` returns and what error messages print.

There is no conversion between types unless you ask for it with `str` or
`int`. In particular only `true` and `false` are accepted where a condition
is needed.

### 5.2 Lists are shared

A list is one object. Binding it to another name, passing it to a function,
or putting it into another list does not copy it: every way of reaching the
list reaches the same list.

```tansy
let a = [1, 2];
let b = a;
push(b, 3);
b[0] = "changed";
print(a, a == b);
```

```text
["changed", 2, 3] true
```

`+` on two lists is the one operation that makes a new list (7.2).

### 5.3 The display form and the quoted form

Every value has two textual forms. `print` and `str` use the *display
form*. The *quoted form* is used for the elements of a list and by the
REPL when it shows a value.

| value | display form | quoted form |
|---|---|---|
| `nil` | `nil` | the same |
| a `bool` | `true` or `false` | the same |
| an `int` | decimal digits, with `-` in front if negative | the same |
| a `str` | its characters, unchanged | see below |
| a `list` | `[`, the quoted forms of the elements separated by `, `, `]` | the same |
| a function declared as `fn name` | `<fn name>` | the same |
| a function literal | `<fn>` | the same |
| a built-in function | `<fn name>` | the same |

The quoted form of a string is `"`, then its characters, then `"`, where
six characters are written as escape sequences: `"` as `\"`, `\` as `\\`,
line feed as `\n`, tab as `\t`, carriage return as `\r`, and U+0000 as
`\0`. Every other character is written as it is.

A list can contain itself. While a list is being written, an occurrence of
that same list inside it is written as `[...]`.

```tansy
print("a \"quoted\" word", ["a \"quoted\" word", "tab\there"], [[], [nil]]);
let loop = [1];
push(loop, loop);
print(loop, [print, fn() { }]);
```

```text
a "quoted" word ["a \"quoted\" word", "tab\there"] [[], [nil]]
[1, [...]] [<fn print>, <fn>]
```

---

## 6. Bindings and scope

### 6.1 Blocks and bindings

A *binding* connects a name to a place that holds a value. Bindings are
created by

- `let NAME = expression` (a binding that cannot be assigned again),
- `var NAME = expression` (a binding that can be assigned),
- `fn NAME(...) { ... }` (cannot be assigned),
- the parameters of a function (cannot be assigned),
- the variable of a `for` loop (cannot be assigned).

Every binding belongs to a *block*: the innermost `{ ... }` that contains
its declaration. The whole script is a block too, the *top level*.
Parameters belong to the block that is the function's body, and the
variable of a `for` loop belongs to the block that is the loop's body.

**Each time a block is entered, all its bindings are made anew.** A loop
body that runs three times has three sets of bindings; so has a function
body that is called three times. This matters when a closure keeps a
binding alive (section 9).

"Cannot be assigned" is about the binding, not about the value: a `let`
binding that holds a list always refers to the same list, and the list can
still be changed.

### 6.2 A name belongs to its whole block

The *scope* of a binding (the part of the program where its name means that
binding) is **the whole block it belongs to, from the opening `{` to the
closing `}`**, including the part before the declaration and including
nested blocks and functions, except where a nested block declares the same
name again (*shadowing*).

When a block is entered, its bindings exist but are *uninitialized*. A
binding gets its value when its declaration runs (for a parameter: when the
function is called; for a loop variable: at the start of each round).
Reading or assigning an uninitialized binding is the runtime error
`` `NAME` is used before it is initialized ``.

This rule is what lets functions call each other regardless of the order in
which they are written:

```tansy
fn is_even(n) { if n == 0 { true } else { is_odd(n - 1) } }
fn is_odd(n) { if n == 0 { false } else { is_even(n - 1) } }
print(is_even(10));
```

```text
true
```

When `is_even` is declared, `is_odd` is already a name of the block. It has
no value yet, but nobody reads it before the call on the last line, and by
then both declarations have run.

The other side of the rule:

```tansy
let a = "outer";
{
    print(a);
    let a = "inner";
}
```

```text
main.tn:3:11: error: `a` is used before it is initialized
        print(a);
              ^
```

Inside the block, `a` means the block's own `a` on every line, also on the
line before its declaration. For the same reason `let x = x + 1;` never
reads an outer `x`: the `x` on the right is the new binding.

### 6.3 The built-in scope and shadowing

The top level is itself inside one more scope that holds the built-in
functions (section 10) as bindings that cannot be assigned. A program can
shadow them like any outer name, also at the top level:
`let len = 3;` is allowed, and from then on `len` is 3 in that block.
The rule of 6.2 holds here too: in that block `len` means the new binding
on every line, so a use of `len` that runs before the `let` is the error
`` `len` is used before it is initialized ``, not a call of the built-in
function.

A name can be declared **once per block**. Declaring it again in a nested
block is shadowing and is fine.

### 6.4 Checks before the program runs

After a program has been parsed without error and before any of it runs,
the whole program is checked, including code that would never run. These
are the *name errors*:

| situation | message | span |
|---|---|---|
| a name is used and no enclosing block (nor the built-in scope) declares it | `` unknown name `NAME` `` | the name |
| a block declares a name twice (parameters and loop variables count) | `` `NAME` is already declared in this block `` | the name in the later declaration |
| an assignment `NAME = ...` to a binding not made by `var` | `` cannot assign to `NAME`: it is not declared with `var` `` | the name |
| `return` that is not inside a function body | `` `return` outside of a function `` | the keyword |
| `break` or `continue` that is not inside a loop body of the same function | `` `break` outside of a loop ``, `` `continue` outside of a loop `` | the keyword |

"Of the same function": a loop does not reach into a function declared in
its body. `while c { fn f() { break; } }` is an error.

"A loop body" is the block of the loop. The condition of a `while` and the
header of a `for` are not part of it (8.3).

When a block declares a name twice, the first declaration is the one that
counts for the other checks. In `var n = 1; n = 2; let n = 3;` the
assignment is correct, and the only error is the second declaration.

If a program has several name errors, the one that starts first in the
text is reported (11.3).

---

## 7. Expressions

Everything is evaluated **from left to right**: the operands of an
operator, the elements of a list, the parts of a call. The checks that an
operation makes happen after all its operands have been evaluated, unless
this section says otherwise (`and`, `or`, `if`).

### 7.1 Literals and names

A literal evaluates to its value. `true`, `false` and `nil` are literals.
A name evaluates to the value of its binding (6.2).

### 7.2 Arithmetic: `+` `-` `*` `/` `%` and unary `-`

On two `int` operands:

- `+`, `-`, `*` are the mathematical operations. If the result is not an
  `int` (5.1), the runtime error is `integer overflow`.
- `/` divides and drops the fractional part (rounds toward zero):
  `-7 / 2` is `-3`. `%` is the remainder of that division, so it has the
  sign of the left operand: `-7 % 2` is `-1`, `7 % -2` is `1`. For all
  `a` and `b != 0`: `a / b * b + a % b == a`.
- A right operand of 0 for `/` or `%` is the error `division by zero`.
- `-9223372036854775808 / -1` is `integer overflow`.
  `-9223372036854775808 % -1` is 0.

Unary `-` on an `int` is its negation; negating -9223372036854775808 is
`integer overflow`.

`+` has two more meanings:

- `str + str` is a new string: the left one followed by the right one.
- `list + list` is a **new** list with the elements of the left one
  followed by those of the right one. The operands are not changed.

Any other combination of operand types is the runtime error
`` cannot apply `OP` to T1 and T2 `` (for unary minus:
`` cannot apply `-` to T ``), with the type names of 5.1.

### 7.3 Ordering: `<` `<=` `>` `>=`

Two `int`s compare as numbers. Two `str`s compare character by character
by code point; a string that is a proper prefix of another is smaller.
(`"Z" < "a"` is true, `"ab" < "abc"` is true.) Any other combination is
`` cannot apply `OP` to T1 and T2 ``. The result is a `bool`.

### 7.4 Equality: `==` `!=`

`==` works on any two values and never fails. Values of different types
are not equal. Within a type:

- `nil` equals `nil`; `bool`s, `int`s and `str`s are equal if they are the
  same value;
- two lists are equal if they are the same list, or if they have the same
  length and their elements are pairwise equal by this same rule;
- two functions are equal only if they are the same function value. Every
  evaluation of a function declaration or function literal makes a new
  function value (section 9). A built-in function equals only itself.

`a != b` is `not (a == b)`.

### 7.5 Logic: `and` `or` `not`

`not x` evaluates `x`, which must be a `bool`, and gives the other `bool`.

`a and b` evaluates `a`, which must be a `bool`. If it is `false`, the
result is `false` and **`b` is not evaluated**. Otherwise `b` is evaluated,
must be a `bool`, and is the result. `a or b` is the same with `true` in
the place of `false`.

An operand that is evaluated and is not a `bool` is the runtime error
`expected bool, found T`. Its span is that operand.

### 7.6 Calls

`callee(arg1, ..., argN)` is evaluated in this order:

1. `callee` is evaluated.
2. The arguments are evaluated from left to right.
3. If the value of `callee` is not a function: `cannot call T`
   (span: the callee expression).
4. If the function does not take N arguments:
   `expected M arguments, found N` (span: the whole call), where M is the
   number it takes. The word is `argument` when M is 1.
5. The function is called (section 9 for your functions, section 10 for
   built-in ones). The value of the call is what the function returns.

Steps 3 and 4 come after step 2: the arguments of a call that is going to
fail are still evaluated, with all their effects and errors.

### 7.7 `if`

`if cond { A } else { B }` evaluates `cond`, which must be a `bool`
(otherwise `expected bool, found T`; span: the condition). If it is `true`
the value is the value of the block `A`; if it is `false`, the value of
the `else` part, or `nil` if there is no `else` part. The part that is not
chosen is not evaluated. `else if` is an `else` part that is another `if`.

### 7.8 Blocks

A block `{ ... }` used as an expression is entered (6.1), its sequence is
run, and its value is the value of the sequence (4.3).

### 7.9 Indexing

`target[index]` evaluates `target`, then `index`. Then, in this order:

1. `target` must be a `list` or a `str`, else `cannot index T`
   (span: the target expression).
2. `index` must be an `int`, else `expected int, found T`
   (span: the index expression).
3. `index` must be at least 0 and less than the length, else
   `index I is out of range for length N` (span: the whole indexing
   expression).

The result is the element of the list at that position (the first has
index 0), or, for a string, a new string of one character: the character
at that position, counted in characters, not bytes. The length of a string
is its number of characters.

### 7.10 List and function literals

`[e1, ..., eN]` evaluates the elements from left to right and makes a new
list of them, each time it is evaluated. `fn(params) { body }` makes a new
function value each time it is evaluated (section 9).

---

## 8. Statements

Declarations (`let`, `var`, `fn NAME`) are described in sections 6 and 9.
`let` and `var` evaluate the expression and then initialize the binding.
An expression statement evaluates the expression and drops the value.

### 8.1 Assignment

The left side of `=` is parsed as an expression and must then turn out to
be a name or an indexing expression (parentheses around it do not matter).
Anything else is the syntax error `invalid assignment target`. Its span is
the left side as an expression (11.4): in `(1 + 2) = 3;` it is `1 + 2`.
Assignment is a statement, not an expression:
`a = b = c;` and `print(a = 1);` are syntax errors.

- `NAME = expression`: the expression is evaluated; then the binding
  takes the value. The binding must be a `var` (6.4). If it is still
  uninitialized (6.2) at that moment, after the expression has been
  evaluated, that is the runtime error of 6.2, with the name as its span.
- `target[index] = expression`: `target`, `index` and the expression are
  evaluated, in that order. Then `target` must be a `list`
  (`cannot assign into T`, span: the target expression; a string cannot be
  changed), `index` must be an `int` and in range (messages and spans as
  in 7.9). Then the element is replaced.

### 8.2 `while`

`while cond { body }` evaluates `cond`, which must be a `bool`
(`expected bool, found T`, span: the condition). If it is `false` the loop
ends. Otherwise the body block is entered and run, and the loop starts
again. The value of the body is ignored.

### 8.3 `break` and `continue`

`break` ends the innermost loop (`while` or `for`) whose body contains it.
`continue` ends the current round of that loop: a `while` evaluates its
condition next, a `for` moves to the next item. Both may appear anywhere
inside the loop body, also inside a block that is part of an expression.
They do not reach out of a function (6.4).

The condition of a `while` and the header of a `for` are not part of the
loop's body. A `break` in a block there belongs to the loop around that
loop, if there is one, and is a name error otherwise (6.4).

### 8.4 `for`

There are two forms. In both, the loop variable is a fresh binding in
every round (6.1), it is in scope in the body only (not in the header),
and it cannot be assigned.

**`for NAME in first..limit { body }`** evaluates `first`, then `limit`,
once. After both have been evaluated, both must be `int`s
(`expected int, found T`, span: that expression); `first` is checked
before `limit`. The body runs once for each integer from `first` up to but
not including `limit`, in increasing order. If `first >= limit`, it does
not run. `..` exists only here; it is not an operator.

**`for NAME in items { body }`** evaluates `items` once.

- For a `str`: the body runs once per character, with a one-character
  string each time.
- For a `list`: the loop keeps a position that starts at 0. Before each
  round it looks at the list as it is at that moment: if the position is
  less than the length, the body runs with the element at the position,
  and afterwards the position goes up by one; otherwise the loop ends. So
  a body that changes the list is well defined.
- Anything else: `cannot iterate over T` (span: the expression).

```tansy
let xs = [1, 2, 3];
for x in xs { if x < 3 { push(xs, x + 10); } }
print(xs);
for i in 0..3 { for c in "ab" { if c == "b" { continue; } print(i, c); } }
```

```text
[1, 2, 3, 11, 12]
0 a
1 a
2 a
```

### 8.5 `return`

`return expression` evaluates the expression and ends the innermost call
of one of your functions; the call's value is the expression's value.
`return` without an expression returns `nil`. (A `return` is followed by
an expression unless the next token is `;`, `}` or the end of input.)

---

## 9. Functions and closures

`fn NAME(p1, ..., pN) { body }` is a declaration statement: it creates a
binding `NAME` in the enclosing block (6.1, 6.2) and, when the statement
runs, initializes it with a new function value. `fn(p1, ..., pN) { body }`
is an expression with a new function value as its value. The two are the
same except for the binding and for the name in the display form (5.3).

A function value is a *closure*: the code, together with the bindings that
are in scope where the `fn` is written. It keeps those bindings (not copies
of their values) for as long as the function value exists, also after the
block that created them has ended.

A call of a function value with N arguments (7.6) does this:

1. If 1000 calls of your functions are already active, the call fails with
   the runtime error `stack overflow` (span: the call expression). See
   13.3. Calls of built-in functions are not counted.
2. The body block is entered: fresh bindings for the parameters and for
   everything the body declares. The enclosing scope of the body is **the
   scope where the function was written**, not the scope of the caller.
3. The parameters are initialized with the argument values, in order.
4. The body's sequence runs. The value of the call is the value of the
   sequence (4.3), or the value of the `return` that ended it.

```tansy
fn make_counter() {
    var count = 0;
    fn() { count = count + 1; count }
}
let a = make_counter();
let b = make_counter();
a(); a();
print(a(), b());            // each call of make_counter made its own `count`

let word = "top";
fn speak() { print(word); }
fn other() { let word = "local"; speak(); }
other();                    // `speak` sees the `word` of the place it was written
```

```text
3 1
top
```

A function can call itself through its own name, because the name belongs
to the whole block (6.2).

---

## 10. Built-in functions

The built-in scope (6.3) holds these nine functions and nothing else. They
are values like your functions: they can be stored, passed and compared.

The number of arguments is checked first (7.6, step 4), then the types.
A type error has the form `NAME: expected WHAT, found T`, and its span is
the argument expression concerned.

| call | result |
|---|---|
| `print(v1, ..., vN)` | Writes the display forms (5.3) of the arguments, separated by one space, and then a line feed, to standard output. Any number of arguments, also none. Returns `nil`. |
| `input()` | Reads the next line of standard input and returns it as a `str` without its line end (`\n` or `\r\n`). A last line without a line end counts. At the end of input: `nil`. If the line is not valid UTF-8: the runtime error `input: invalid UTF-8` (span: the call). |
| `len(v)` | The number of characters of a `str` or the number of elements of a `list`. Type error: `len: expected str or list, found T`. |
| `push(list, v)` | Adds `v` at the end of the list. Returns `nil`. Type error: `push: expected list, found T`. |
| `pop(list)` | Removes the last element and returns it. Type error: `pop: expected list, found T`. On an empty list: the runtime error `pop: the list is empty` (span: the call). |
| `str(v)` | The display form of `v` as a `str`. |
| `int(v)` | For an `int`: `v` itself. For a `str`: if it consists of an optional `-` followed by one or more ASCII digits and nothing else, and the number is an `int` (5.1), that number; otherwise `nil`. (No spaces, no `+`, no other digits.) Type error: `int: expected str or int, found T`. |
| `type(v)` | The type name of `v` (5.1) as a `str`. |
| `exit(code)` | Ends the program at once with exit status `code`, after everything printed so far has been written. Type error: `exit: expected int, found T`. If `code` is not from 0 to 125: the runtime error `exit: expected a code from 0 to 125, found N` (span: the argument). |

```tansy
print(len("żółw"), len([1, [2, 3]]), type(len), str([1, "a"]) + "!");
print(int("42") + 1, int("-7"), int("4 2"), int("12abc"), int(""));
let stack = [1, 2];
push(stack, 3);
print(pop(stack), stack, type(nil), type("x"[0]));
```

```text
4 2 fn [1, "a"]!
43 -7 nil nil nil
3 [1, 2] nil str
```

---

## 11. Errors

### 11.1 The three kinds

| kind | found | effect |
|---|---|---|
| **syntax error** | when the source text is turned into tokens and a tree (sections 3, 4) | nothing runs; exit status 2 |
| **name error** | when a program without syntax errors is checked (6.4) | nothing runs; exit status 2 |
| **runtime error** | while the program runs | the program stops; what it printed before stays printed; exit status 1 |

Each error is reported as one *diagnostic* on standard error. Tansy has no
way to catch an error.

### 11.2 The format of a diagnostic

A diagnostic is exactly three lines:

```text
FILE:LINE:COLUMN: error: MESSAGE
    SOURCE LINE
    MARKER
```

- FILE is the script path as given on the command line, or `<repl>`.
- LINE and COLUMN (3.1) are where the *span* of the error starts (11.4).
- SOURCE LINE is four spaces, then the text of that line without its line
  end, with every tab replaced by one space.
- MARKER is four spaces, then COLUMN - 1 spaces, then one `^` for each
  character of the span that is on that line. If the span goes on past the
  end of the line, the carets stop at the end of the line. There is always
  at least one caret.

```tansy
print("total:", 10 / (5 - 5));
```

```text
main.tn:1:17: error: division by zero
    print("total:", 10 / (5 - 5));
                    ^^^^^^^^^^^^
```

The *end of input* is a position too: the place just after the last
character of the text. If the text ends with a line end, that is column 1
of the line after it, the source line is empty (so the second line of the
diagnostic is four spaces), and the marker is one caret.

### 11.3 Which error is reported

Exactly one diagnostic is written per program (in the REPL: per entry).

- **Syntax errors.** The interpreter reads the tokens in order and stops
  at the first token that cannot continue a correct program. That token is
  the error. A lexical error (3.4 to 3.6) is a token that can never
  continue a correct program; when it is the first such token, its own
  message is the message. So of two errors, a lexical one and a
  grammatical one, the one that comes first in the text is reported.
- **Name errors** are looked for only in a program that has no syntax
  error. Of several, the one whose span starts first in the text is
  reported.
- **Runtime errors**: the first one that happens stops the program.

### 11.4 Spans

A *span* is a stretch of the source text. The span of a token is the token.
The span of an expression runs from the first character of its first token
to the last character of its last token. For example the span of
`f(a, b)` is all of `f(a, b)`, and the span of `x + (y * 2)` is all of it,
including the parentheses, because `)` is the last token of the sum.

Parentheses are not part of the expression they surround: when
`(1 / 0)` fails, the span is `1 / 0`.

### 11.5 The messages

`T`, `T1`, `T2` stand for type names (5.1), `NAME` for a name, `OP` for an
operator as written.

**Lexical errors** (sections 3.4 to 3.6):

| message | span |
|---|---|
| `unexpected character 'X'` | the character |
| `unterminated string` | the opening `"` |
| `invalid escape sequence '\X'` | the `\` and the character after it |
| `integer literal is too large` | the literal |

**Grammatical errors.** Most have the form `expected WHAT, found TOKEN`,
and their span is the token found. TOKEN is the text of the token between
backquotes, exactly as written in the source (a string literal with its
quotes and escapes), or the words `end of input` without backquotes. WHAT
depends on the place:

| the parser is at | WHAT |
|---|---|
| a place where an expression must start (an operand, an argument or element after `,`, the right side of `=`, a condition, the start of a statement by rule 5 of 4.3, ...) | `expression` |
| after `let`, `var` or `for`; in a parameter list where a parameter must come | `a name` |
| after the name in `let` or `var` | `` `=` `` |
| after `fn` when no name follows (4.3, rule 3), or after the name of a function declaration | `` `(` `` |
| after a parameter or an argument | `` `,` or `)` `` |
| after an element of a list literal | `` `,` or `]` `` |
| after the expression inside parentheses | `` `)` `` |
| after the index expression | `` `]` `` |
| where a block must start: after the condition of `if` or `while`, after the header of `for`, after the parameter list | `` `{` `` |
| after `else` | `` `if` or `{` `` |
| after the name in `for` | `` `in` `` |
| at the end of a statement (4.3) | `` `;` `` |
| at the end of input, where the next statement of a block or its `}` must come | `` `}` `` |

In an empty argument or parameter list, or after a trailing comma, the
closing delimiter is accepted; any other token is an error of the first or
second row (`expression` for arguments and list elements, `a name` for
parameters).

Three more grammatical errors:

| message | span |
|---|---|
| `invalid assignment target` (8.1) | the expression left of `=` |
| `comparison operators cannot be chained` (4.2) | the second comparison operator |
| `nesting too deep` (13.1, 13.2) | see section 13 |

**Name errors:** the table in 6.4.

**Runtime errors:**

| message | span | defined in |
|---|---|---|
| `` `NAME` is used before it is initialized `` | the name | 6.2 |
| `` cannot apply `OP` to T1 and T2 `` | the whole operation | 7.2, 7.3 |
| `` cannot apply `-` to T `` | the whole operation | 7.2 |
| `integer overflow` | the whole operation | 7.2 |
| `division by zero` | the whole operation | 7.2 |
| `expected bool, found T` | the operand or condition | 7.5, 7.7, 8.2 |
| `expected int, found T` | the index, or the bound of a range | 7.9, 8.1, 8.4 |
| `cannot call T` | the callee expression | 7.6 |
| `expected M arguments, found N` | the whole call | 7.6 |
| `stack overflow` | the whole call | 9, 13.3 |
| `cannot index T` | the target expression | 7.9 |
| `index I is out of range for length N` | the whole indexing expression | 7.9, 8.1 |
| `cannot assign into T` | the target expression | 8.1 |
| `cannot iterate over T` | the expression after `in` | 8.4 |
| the messages of the built-in functions | | 10 |

---

## 12. The REPL

`tansy` without arguments reads Tansy from standard input, one *entry* at
a time, and evaluates each entry in one lasting top-level scope.

### 12.1 Prompts

Before it reads a line that starts a new entry, the REPL writes the prompt
`> ` (greater-than, space) to standard output. Before it reads a line that
continues an unfinished entry, it writes `. ` (full stop, space). There is
no banner and no other decoration.

Prompts are written whether or not standard input is a terminal. Before
each read, the prompt and all earlier output must have been flushed (2.3):
somebody who types a line sees the answer before typing the next one.

### 12.2 Entries

The REPL reads a line and adds it, with a line feed, to the entry. The
entry is *complete* unless both of these hold:

- the text of the entry so far has no lexical error (3.4 to 3.6), and
- it contains more opening delimiters (`(`, `[`, `{`) than closing ones
  (`)`, `]`, `}`), counted as tokens. (A `{` inside a string or a comment
  is not a token.)

A complete entry is evaluated at once. An entry that is not complete gets
the next line. Note that only delimiters keep an entry open: the line
`1 +` is a complete entry, with a syntax error.

Bytes on a line that are not valid UTF-8 are replaced by U+FFFD, the way
Rust's `String::from_utf8_lossy` does it.

### 12.3 Evaluating an entry

An entry is a program (4.1), evaluated in the lasting top-level scope.

- If it has a syntax or name error, the diagnostic is written to standard
  error and **the entry has no effect at all**: none of its declarations
  exist afterwards.
- If a runtime error stops it, the diagnostic is written; everything the
  entry did up to that point stays done.
- If it runs to its end and the value of its sequence (4.3) is not `nil`,
  the REPL writes the *quoted form* (5.3) of that value and a line feed to
  standard output.

Then the REPL goes on with the next prompt. An empty entry (blank, or only
comments) does nothing.

```text
> let greeting = "hello"
> greeting + ", world"
"hello, world"
> fn twice(f, x) {
.     f(f(x))
. }
> twice(fn(n) { n * n }, 3)
81
> print(greeting); 1 / 0
hello
<repl>:7:18: error: division by zero
    print(greeting); 1 / 0
                     ^^^^^
> nil
>
```

(The diagnostic goes to standard error; on a terminal the two streams
appear together as shown. After the last prompt the input ended.)

### 12.4 State between entries

All entries share one top level. A binding declared by one entry is in
scope in all later entries.

The rules of section 6 apply to each entry as a program of its own, with
two additions:

- The names declared by earlier entries are known. The names an entry
  declares itself belong to the whole entry (6.2), so the functions of one
  entry can call each other in any order. A name that only a *later* entry
  will declare is unknown.
- An entry may declare a top-level name that an **earlier** entry
  declared. (Declaring a name twice within one entry is still an error.)
  The new declaration takes the place of the old binding: code from
  earlier entries that uses the name sees the new value. From this entry
  on, the name can or cannot be assigned according to the new declaration.
  (Code from earlier entries was checked when it was entered and is not
  checked again: a function that assigns the name goes on doing so.)
  The binding keeps the value it had until the new declaration runs.

If a runtime error stops an entry before one of its declarations has run,
that name is declared and uninitialized (6.2) until a later entry declares
it again.

### 12.5 Diagnostics in the REPL

FILE is `<repl>`. The lines of all entries are numbered in one sequence
for the whole session: the first line the REPL reads is line 1. (Lines
that `input()` reads are data and are not counted.) So an error inside a
function reports the line where that part of the function was typed, and
shows it, even when a later entry called the function.

For an entry that ends with its line feed, the end of input (11.2) is
column 1 of the line that the next entry will be.

### 12.6 The end, `exit` and `input`

When there is no more input, the REPL first evaluates an entry that is
still unfinished, as it is (it normally has a syntax error, which is
reported). Then it writes a line feed to standard output and exits with
status 0. Errors in entries do not change the exit status.

`exit(n)` ends the session at once with status `n`; no line feed is
written.

`input()` reads the next line of standard input, the same stream the REPL
reads its entries from. A line read by `input()` is not an entry.

---

## 13. Limits

### 13.1 Open delimiters

At every point of a source text, at most **100** delimiters may be open
(`(`, `[` and `{` that have not been closed yet). An opening delimiter
that would be the 101st is the syntax error `nesting too deep`; its span is
that delimiter. This limit is exact and tested. (If the grammar allows no
such delimiter at that place anyway, either `nesting too deep` or the
grammatical error may be reported for it.)

### 13.2 Nesting without delimiters

Three constructs nest without any delimiter being open:

- a run of prefix operators (`- - - x`, `not not x`);
- `if` directly inside the condition of an `if`, and `else if` chains;
- a run of infix or postfix operators at one level
  (`a + b + c + ...`, `f()()()...`, `a[0][0][0]...`).

Your interpreter must accept such runs up to a length of 250. For longer
runs it may either work or report the syntax error `nesting too deep`
anywhere in the run. It should not crash; the tests do not try.

### 13.3 Calls

At most **1000** calls of your functions may be active at the same time
(section 9). The call that would be the 1001st fails with `stack overflow`.
This limit is exact and tested: a recursion 1000 calls deep must work.

Apart from the number of calls, your interpreter may give up with the
runtime error `stack overflow` when the expressions and blocks that are
being evaluated at one moment, in all active calls together, are nested
more than 10 000 levels deep. (1000 active calls of functions written in a
normal style need a few thousand levels.) It must not give up earlier.
The span of that error is the expression at which your interpreter stops.

A recursive evaluator needs a deep native stack for this. Running out of
it must not happen within these limits; how you make sure is part of the
project.

### 13.4 Speed

There is no precise requirement. The tests in `tests/m7_programs.rs` run a
few hundred thousand function calls, a million loop rounds, a hundred
thousand list operations and a script of 1 MiB, and allow two minutes for
each. The reference solution, a tree-walking evaluator, needs well under a
second for each of them without optimisation.

Reading an element of a list by its index and `push` must not take time in
proportion to the length of the list.

### 13.5 Sizes and other limits

- A source text of up to 1 MiB must be accepted.
- Strings and lists are limited only by memory.
- Values that hold each other in a chain more than 1000 links long need
  not work: a list inside a list inside a list, or a function that
  captures a binding that holds a function, and so on. (Building, printing,
  comparing and freeing such a chain all walk it.)
- `==` on two different lists that both contain themselves need not end.
- Memory that the program can no longer reach need not be given back
  while the program runs (see 14).

What happens outside the limits of this section is not specified, with one
general wish: an error is better than a crash.

---

## 14. Non-goals

Tansy is deliberately small. These are **not** part of the language, and
you should not build them before the tests pass:

- numbers other than 64-bit integers (no floats);
- maps, sets, tuples, records, classes, methods;
- catching errors (`try`), and errors as values beyond what a program
  builds itself out of lists;
- modules, imports, a standard library beyond section 10;
- string interpolation, string slicing, negative indices;
- compound assignment (`+=`), increment operators, a ternary operator;
- more than one diagnostic per program, warnings, error recovery;
- call stack traces in runtime errors;
- tail-call elimination;
- reclaiming unreachable memory, in particular reference cycles (a closure
  stored in a binding it captures; a list that contains itself);
- line editing, history or colours in the REPL;
- threads, timers, files, command-line arguments for scripts.

Several of them make good stretch goals (section 16).

---

## 15. Milestones

Work in this order. Each milestone has one test file; run it with
`cargo test --test NAME`. A milestone is done when its file and all
earlier files pass.

Each milestone needs only part of this document. Later milestones change
code you wrote for earlier ones; that is expected, and the earlier tests
tell you whether you broke something.

**M1 · Expressions** — `tests/m1_expressions.rs` (36 tests)

Run a script file. Lex the whole of section 3. Parse and evaluate
expression statements with literals, parentheses, all operators of 4.2,
and calls of `print`. Sections 2.1 to 2.3 (script mode), 3, 4.2, 5.1, 5.3
(without lists), 7.1 to 7.5.
You do not need bindings yet; `print` can be the one name you know.

**M2 · Diagnostics** — `tests/m2_diagnostics.rs` (42 tests)

Every token and every expression knows its span. Lexical, grammatical and
runtime errors are reported in the format of 11.2 with the right exit
status, and nothing runs when the script does not parse. The nesting
limits of 13.1 and 13.2. Sections 2.2, 11, 13.1, 13.2.

**M3 · Bindings and control flow** — `tests/m3_bindings.rs` (52 tests)

`let`, `var`, assignment to names, blocks as expressions, `if`, `while`,
`break`, `continue`, the rules of 4.3, and the checks of 6.4. This is
where the rule of 6.2 (a name belongs to its whole block) has to be
designed in. Sections 4.3, 6, 7.7, 7.8, 8.1 to 8.3.

**M4 · Functions and closures** — `tests/m4_functions.rs` (41 tests)

Declarations, literals, calls, `return`, recursion, closures that share
bindings, the call depth limit. Sections 7.6, 8.5, 9, 13.3.

**M5 · Lists, strings and `for`** — `tests/m5_collections.rs` (49 tests)

List literals, indexing, assignment to elements, the display and quoted
forms for lists, `for` in all three forms, and the built-in functions
`len`, `push`, `pop`, `str`, `int`, `type`. Sections 5.2, 5.3, 7.2 (`+` on
lists), 7.4, 7.9, 7.10, 8.1, 8.4, 10, 13.5.

**M6 · The command line and the REPL** — `tests/m6_repl.rs` (46 tests)

Usage errors, unreadable scripts, `exit`, `input`, output that cannot be
written, and the REPL. Sections 2, 10 (`input`, `exit`), 12.

**M7 · Programs** — `tests/m7_programs.rs` (12 tests)

The six programs in `programs/` produce their expected output, and the
speed tests finish in time. If the earlier milestones pass, most of these
pass too; what is left are mistakes that only larger programs show, and
choices that make the interpreter very slow. Sections 13.4, 13.5.

---

## 16. Stretch goals

None of these is tested. Each one is a real project; pick what interests
you. Keep the acceptance tests passing while you work.

1. **A bytecode compiler and a virtual machine** in place of the tree
   walk: a flat instruction array, a value stack, call frames, and
   closures that capture single variables. Measure both designs on the
   same programs before you decide which to keep.
2. **Error recovery in the parser**: report every syntax error of a file,
   not only the first, without drowning the user in follow-up errors.
   (Behind a command-line flag, since section 11.3 asks for exactly one.)
3. **Stack traces**: a runtime error lists the calls that were active,
   innermost first, each with its position.
4. **Maps**: `{"key": value}` literals, indexing, `for` over keys in
   insertion order, the built-in functions that go with them. Decide which
   values can be keys and why.
5. **A cycle collector**: find and free closures and lists that only
   reference each other, and prove with a test that memory stays bounded.
6. **Tail calls**: a call in tail position does not count toward the limit
   of 13.3 and uses no stack.
7. **A formatter**: `tansy --format SCRIPT` prints the script in a
   standard layout and keeps the comments.
8. **Fuzzing**: outside this package (it needs a third-party tool), feed
   random bytes and random token sequences to your lexer and parser and
   fix every crash and every hang it finds.
9. **A better REPL**: line editing and history without a library, by
   switching the terminal to raw mode yourself. This one needs `unsafe`
   and platform-specific code; read the unsafe chapter first.
