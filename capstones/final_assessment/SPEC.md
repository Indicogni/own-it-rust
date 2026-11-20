# rgx: specification

This document says exactly what your library and your program must do. It
does not say how. There are no modules, no data structures and no
algorithms here, because choosing them is the assessment. What is fixed is
everything that somebody else can observe: which patterns are valid, what a
pattern matches, the text and position of every error, how long a search
may take, how much it may allocate, the command line, the bytes on standard
output and standard error, and the exit status.

Words used with a fixed meaning:

- **must**: required. Where a test checks it, section 10 names the test
  file.
- **should**: expected of a good solution, but no test can see it. The
  rubric asks about these.
- **may**: your decision.

Notation used everywhere:

- A **character** is a Unicode scalar value: what Rust calls a `char`. It is
  not a byte, and it is not what a reader sees as one letter. `é` (U+00E9)
  is one character of two bytes. `e` followed by U+0301 (a combining
  accent) is two characters.
- An **offset** is a number of bytes from the start of a `&str`. Offsets
  count from 0.
- `LF` is the line feed, U+000A. `CR` is the carriage return, U+000D.
- `U+0041` names a character by its number in hexadecimal.

Contents:

1. [What you build](#1-what-you-build)
2. [The library](#2-the-library)
3. [The pattern language](#3-the-pattern-language)
4. [What a pattern matches](#4-what-a-pattern-matches)
5. [Pattern errors](#5-pattern-errors)
6. [Limits](#6-limits)
7. [The program](#7-the-program)
8. [Memory](#8-memory)
9. [Build rules and non-goals](#9-build-rules-and-non-goals)
10. [Milestones and their tests](#10-milestones-and-their-tests)
11. [Stretch goals](#11-stretch-goals)

---

## 1. What you build

`rgx` is two things in one package.

**A library** that compiles a regular expression and searches text with it.

- It reads a pattern in the language of section 3 and reports a precise
  error, with a position, for every pattern that is not in that language.
- It compiles a valid pattern into a finite automaton.
- It runs the automaton in such a way that the time of a search is
  proportional to the length of the text, **for every pattern and every
  text**. There is no pattern that makes it slow. This is the property that
  separates this engine from the backtracking engines of many languages,
  and section 4.5 states it exactly.
- It works on UTF-8 text: a pattern talks about characters, and every
  position the library reports can be used to slice the text.

**A program**, also named `rgx`, that prints the lines of files that match
a pattern.

```text
rgx [OPTIONS] PATTERN [PATH...]
```

- It searches standard input, files, and directories with everything below
  them.
- It searches many files at the same time on several threads, and its
  output is the same, byte for byte, on every run and for every number of
  threads.
- It reports what it could not do on standard error, and its exit status
  tells a script what happened.

The acceptance tests look at the library through the five functions of
section 2 and at the program through its command line. Everything behind
those two surfaces is yours.

## 2. The library

The package has a library crate named `rgx` (the starter's `Cargo.toml`
sets this up). The tests use two types from its root, `rgx::Regex` and
`rgx::Error`, and the following items. This is the **facade**: keep these
names, what they take, and what they return.

```text
impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, Error>;
    pub fn is_match(&self, haystack: &str) -> bool;
    pub fn find(&self, haystack: &str) -> Option<Range<usize>>;
    pub fn find_iter(&self, haystack: &str)        -> an iterator of Range<usize>;
    pub fn matching_lines<'h>(&self, text: &'h str) -> an iterator of (usize, &'h str);
}

impl Error {
    pub fn offset(&self) -> usize;
}
```

`Range` is `std::ops::Range`. The starter writes the two iterators as
`impl Iterator<Item = ...>`. You may return named types instead. You may
add methods, types, traits, functions and modules as you like, and the
program of section 7 will need more than the facade offers (section 3.9 is
one example). The tests never name anything else.

### 2.1 `Regex`

A `Regex` is a compiled pattern.

- `Regex::new(pattern)` returns the compiled pattern if `pattern` is valid
  by sections 3 and 6, and otherwise the error that section 5 defines for
  it. It must never panic, whatever the pattern is, and section 6.2 limits
  the work it may do.
- `is_match(haystack)` is `true` exactly when `find(haystack)` is `Some`.
- `find(haystack)` returns the match of section 4.3 as the range
  `start..end` of byte offsets, or `None` if there is no match.
- `find_iter(haystack)` returns an iterator over the successive matches of
  section 4.4.
- `matching_lines(text)` returns an iterator over the lines of `text` that
  contain a match. Lines are defined in section 7.2. Each line is searched
  as a haystack of its own. An item is the pair of the line's number (the
  first line of the text is line 1) and the line itself, without its line
  feed. Section 8 says what this iterator may cost.

Further requirements:

- `Regex` must implement `Debug`, `Send` and `Sync`.
- Every method takes `&self` and leaves the value unchanged as far as a
  caller can see. Any number of threads may search with one `Regex` at the
  same time, and each gets the result it would get alone.
- The items of `matching_lines` borrow from `text` only, never from the
  `Regex`: the lines can be kept after the `Regex` is dropped. (The
  iterator itself may borrow from both.)
- The searching methods must never panic, whatever the haystack is.

### 2.2 `Error`

An `Error` says why `Regex::new` refused a pattern.

- `offset()` returns the byte offset in the pattern that section 5.2 gives
  for the error. It always lies on a character boundary of the pattern.
- `Display` prints the message of section 5.2 and nothing else: no
  position, no prefix, no final full stop. For the pattern `a(b` it prints
  exactly `missing closing parenthesis`.
- `Error` must implement `Debug` and `std::error::Error`, and it must be
  `Send + Sync + 'static`, so that `?` can turn it into a
  `Box<dyn std::error::Error + Send + Sync>`.

You may make `Error` a struct or an enum and give it any further methods.

## 3. The pattern language

A pattern is a `&str`. It is read as a sequence of characters, never as
bytes. The language is a deliberately small subset of what the common
regular-expression libraries accept. Where it has a construct, the
construct means what you expect from those libraries, with the exceptions
that this section states. Section 9 lists what is left out.

### 3.1 Grammar

```text
pattern        = alternation
alternation    = concatenation { "|" concatenation }
concatenation  = { repetition }
repetition     = atom [ quantifier ]
quantifier     = "*" | "+" | "?" | count
count          = "{" number "}" | "{" number ",}" | "{" number "," number "}"
atom           = group | class | "." | "^" | "$" | "\b" | "\B"
               | char-escape | set-escape | literal
group          = "(" alternation ")"
class          = "[" [ "^" ] class-item { class-item } "]"
class-item     = class-char "-" class-char | class-char | set-escape
class-char     = char-escape | any character except  \  ]  [
char-escape    = "\" punctuation | "\n" | "\t" | "\r" | "\u{" hex-digits "}"
set-escape     = "\d" | "\D" | "\w" | "\W" | "\s" | "\S"
literal        = any character except  \  .  ^  $  |  (  )  [  {  *  +  ?
number         = one or more of the digits 0 to 9
hex-digits     = one to six of the digits 0 to 9, a to f, A to F
punctuation    = one of the 32 characters  ! " # $ % & ' ( ) * + , - . /
                 : ; < = > ? @ [ \ ] ^ _ ` { | } ~
```

`{ x }` means zero or more times `x`, and `[ x ]` means `x` or nothing. A
`concatenation` can be empty, so the empty pattern, an empty group `()` and
an empty branch as in `a|` are all valid. The following sections give the
meaning of each rule, the two places where the grammar alone does not
decide how to read a pattern (the `-` in a class, section 3.5, and the `{`,
section 3.7), and the one rule that refuses patterns which the grammar
allows (a loop around something that can be empty, section 3.7).

Operators bind in the order of the grammar: a quantifier applies to the one
atom before it, concatenation comes next, and `|` binds least. `ab|cd*`
is "`ab`, or `c` followed by any number of `d`".

### 3.2 Literal characters

Every character other than the twelve **metacharacters**

```text
\  .  ^  $  |  (  )  [  {  *  +  ?
```

is a literal and matches itself. That includes the space, every other white
space character, every character outside ASCII, and the two closing
characters `]` and `}`, which are special only inside the construct they
close. There are no comments and no free-spacing mode: `a b` matches the
three characters `a`, space, `b`.

A literal matches exactly its own character. `é` (U+00E9) does not match
`e` followed by U+0301, although the two look the same.

### 3.3 Escapes

A `\` gives the next character a meaning, as follows.

| escape | meaning |
|---|---|
| `\` and one of the 32 punctuation characters of the grammar | that character, as a literal. `\.` is a full stop, `\\` is a backslash, `\-` is a hyphen. A punctuation character that needs no escape may still have one: `\@` is `@`. |
| `\n`, `\t`, `\r` | the line feed U+000A, the tab U+0009, the carriage return U+000D |
| `\u{H}` | the character with the hexadecimal number `H`. `H` is one to six hexadecimal digits in either case; leading zeros are allowed. `\u{e9}`, `\u{E9}` and `\u{00E9}` are all `é`. The number must be a Unicode scalar value: at most `10FFFF`, and not from `D800` to `DFFF`. |
| `\d`, `\D`, `\w`, `\W`, `\s`, `\S` | a predefined set of characters (section 3.5) |
| `\b`, `\B` | an assertion (section 3.8) |

Every other character after a `\` is an error (`unknown escape`): all
letters and digits not listed above, the space, control characters, and
every character outside ASCII. So there are no back-references (`\1`), and
no `\x41`, `\A`, `\z`, `\p{...}`.

### 3.4 The dot

`.` matches any one character except the line feed. It matches a carriage
return. It matches one character whatever its length in bytes: `a.c`
matches `a😀c`, which is six bytes long.

### 3.5 Classes

A class `[...]` matches one character that is in its set. `[^...]` matches
one character that is **not** in the set written after the `^`. A negated
class matches a line feed unless the line feed is listed; in this it
differs from the dot.

The set is the union of the items:

- a single character, written literally or as a `char-escape`: `[abc]`,
  `[\n\u{e9}]`;
- a range `X-Y`: every character whose number is from the number of `X` to
  the number of `Y`, both included. `X` and `Y` are single characters,
  literal or escaped, and `X` must not come after `Y`. `[a-a]` is valid;
  `[z-a]` is an error;
- a `set-escape`: all the characters of that predefined set. `[\d.]` is
  the digits and the full stop, and `[\D]` is the same as `\D`.

Inside a class only four characters are special:

- `\` starts an escape. The assertions `\b` and `\B` are not allowed in a
  class (`unknown escape`).
- `]` ends the class, wherever it stands. To put a `]` into a class, write
  `\]`. As a consequence `[]` and `[^]` are empty classes, which are
  errors, and `[]a]` is an error too.
- `[` must be escaped as `\[`. An unescaped `[` inside a class is an error
  (`unsupported class syntax`); this keeps `[[:alpha:]]` and nested classes
  free for later.
- `^` negates the class if it is the first character after the `[`.
  Anywhere else it is the character `^`.

All other characters, including `.`, `*`, `(`, `|`, `$` and `{`, are
literals in a class.

**The hyphen.** Read the items from left to right. After a single character
`X` (not a set escape) has been read: if the next character is `-`, and the
character after that `-` exists and is not `]`, then `X`, the `-` and the
next class character form a range. In every other place a `-` is the
character `-` (and, like any single character, it can be the `X` of a
range). So:

```text
[a-c]      the range a to c
[-a]       the characters - and a       (a - at the start)
[a-]       the characters a and -       (a - before the closing bracket)
[^-a]      everything except - and a
[a-c-e]    the range a to c, then -, then e   (a - directly after a range)
[\d-z]     the digits, then -, then z   (a - after a set escape)
[+--]      the range + to -, which is the three characters + , -
[a-c--0]   the range a to c, then the range - to 0 (a - directly after
           a range is a single character, so it can begin a range)
[a\-z]     the three characters a, - and z
[a--]      an error: the range a to - runs backwards (`invalid class range`)
[a-\d]     an error: a range cannot end in a set (`invalid class range`)
```

A set that is empty **after** negation is valid and matches nothing:
`[^\u{0}-\u{10FFFF}]` and `[^\d\D]` never match.

**The predefined sets** are defined on ASCII:

| escape | set |
|---|---|
| `\d` | the ten digits `0` to `9` |
| `\w` | the 63 *word characters*: `0` to `9`, `A` to `Z`, `a` to `z` and `_` |
| `\s` | the six characters U+0009 (tab), U+000A (line feed), U+000B, U+000C, U+000D (carriage return) and U+0020 (space) |
| `\D`, `\W`, `\S` | every character that is **not** in `\d`, `\w`, `\s` |

`\d` does not match `٣` (an Arabic-Indic digit), `\w` does not match `é`,
and `\s` does not match the no-break space U+00A0. Consequently `\D`, `\W`
and `\S` do match them.

Classes hold characters, never bytes. `[é]` matches the character `é` and
nothing else; in particular it does not match `ã`, although the two share
their first byte in UTF-8.

### 3.6 Groups and alternation

`(e)` matches what `e` matches. Groups only group: they exist so that a
quantifier or an alternation can apply to more than one atom. Nothing is
captured, and a group has no number.

A `(` that is directly followed by `?` is an error (`unsupported group
syntax`). This keeps `(?:...)`, `(?i)`, `(?<name>...)` and the look-around
forms free for later.

`e1|e2` matches what `e1` matches or what `e2` matches. When both can
match, `e1` is preferred; section 4.2 says exactly what that means. A
branch may be empty, and an empty branch matches the empty string: `a|`
matches `a` or nothing.

### 3.7 Quantifiers

| quantifier | the atom before it is matched |
|---|---|
| `?` | zero times or once |
| `*` | zero or more times |
| `+` | once or more |
| `{n}` | exactly `n` times |
| `{n,}` | `n` times or more |
| `{n,m}` | at least `n` and at most `m` times |

- All quantifiers are **greedy**: more repetitions are preferred to fewer
  (section 4.2).
- `n` and `m` are written with the ASCII digits `0` to `9` only. Leading
  zeros are allowed: `a{007}` is `a{7}`. There are no spaces and no signs
  inside the braces.
- `n` must not be greater than `m`, and no count may be greater than 1000
  (section 6). `a{0}` is valid and matches the empty string.
- A quantifier may follow any atom, including an assertion and a group,
  with the one restriction on loops that is described below.
- An atom takes **one** quantifier. A quantifier character directly after
  a quantifier is an error (`nothing to repeat`). So there are no lazy
  quantifiers (`a*?`, `a+?`) and no possessive ones (`a*+`), and `a{2}{3}`
  must be written `(a{2}){3}`.
- A quantifier character where an atom is expected (at the start of the
  pattern, after `(`, after `|`) is an error (`nothing to repeat`).

**Loops need something to consume.** `*`, `+` and `{n,}` are the three
**loops**: they have no upper limit. `?`, `{n}` and `{n,m}` are not loops;
they stand for a fixed number of copies, some of them optional. A loop
after an expression that **can match the empty string** is an error
(`repeated expression can be empty`). So `(a*)*`, `(a|)+`, `()*`, `^*` and
`(a?){2,}` are errors, and `(a*){3}`, `(a|)?`, `(){2}` and `^?` are valid.

Whether an expression can match the empty string is decided from the
pattern alone, by these rules:

| expression | can match the empty string |
|---|---|
| the empty expression (an empty pattern, group or branch) | yes |
| an atom that matches one character: a literal, an escaped character, `.`, a class, a predefined set | no |
| an assertion `^`, `$`, `\b`, `\B` | yes |
| a group `(e)` | if `e` can |
| a concatenation | if every part can |
| an alternation | if at least one branch can |
| `e?`, `e*` | yes |
| `e+` | no (`e` cannot, or the `+` would be an error) |
| `e{n}`, `e{n,m}` | if `n` is 0, or if `e` can |
| `e{n,}` | if `n` is 0 (`e` cannot, or the loop would be an error) |

The rules do not look at what an assertion means: `(^$)*` is an error
although `^$` matches only in an empty haystack, and `(a^)*` is valid
although it can never match more than the empty string.

Why this rule exists: a loop that can go round without consuming anything
has no agreed meaning. Widely used engines give different answers, and
none of the answers is easy to state exactly:

```text
pattern    haystack   Perl 5, Python   JavaScript   the regex crate of Rust
(a*|b)*    abab       a                abab         abab
(|a)+      aa         (empty)          aa           (empty)
```

`rgx` refuses such a pattern instead of choosing among them. Nothing that
a loop can match is lost by this: `(a*)*` matches the same texts as `a*`,
and every such loop can be rewritten so that each round consumes a
character. Where each round must consume, the engines above agree with
each other and with section 4: all of them report `a` for `(a|ab)*` in
`abab`.

**The opening brace.** Outside a class, `{` is always a metacharacter.
Where an atom is expected it is an error (`nothing to repeat`). After an
atom it must begin one of the three count forms, complete and well formed;
if it does not, that is an error (`invalid repetition`). To match the
character `{`, write `\{`. The closing brace `}` on its own is a literal.

### 3.8 Assertions

An assertion matches the empty string at a position where a condition
holds. It consumes nothing.

| assertion | holds at a position when |
|---|---|
| `^` | the position is offset 0 of the haystack |
| `$` | the position is the end of the haystack |
| `\b` | exactly one of the two characters around the position is a word character |
| `\B` | `\b` does not hold |

- "The two characters around the position" are the character that ends
  there and the character that starts there. At offset 0 there is no
  character before, and at the end there is none after; a missing
  character is not a word character.
- The word characters are those of `\w` (section 3.5): ASCII only. `é` is
  not a word character, so `\b` holds between `a` and `é`, and does not
  hold at either end of the haystack `é`.
- `^` and `$` are the two ends of the **haystack**. A line feed inside the
  haystack does not make a new start or end; there is no multi-line mode.
  (The program of section 7 searches each line as a haystack of its own,
  so there `^` and `$` are the ends of a line.)
- `$` holds only at the very end. In the haystack `a` + `LF` it holds
  after the line feed and not before it.
- Assertions are ordinary atoms. They may stand anywhere (`a^b` is valid
  and never matches). They may take `?` or a count (`^?`, `\b{2}`), but
  not a loop: an assertion can match the empty string (section 3.7).

In the empty haystack `^`, `$` and `\B` hold at offset 0, and `\b` does
not.

### 3.9 Ignoring case

A pattern can be compiled in a mode that **ignores case**. The program
uses it for its option `-i`. The facade of section 2 does not offer it:
`Regex::new` always compiles case-sensitively, and how your library offers
the mode to your program is your design.

Only the 52 ASCII letters have a case in this mode:

- A literal character that is an ASCII letter matches that letter in both
  cases. It does not matter whether the pattern wrote it directly or as an
  escape: with the mode on, `\u{41}` matches `A` and `a`.
- For a class, take the set as written (the union of its items). Add, for
  every ASCII letter in it, the same letter in the other case. Only then
  apply the `^`, if the class is negated. So `[^a]` refuses both `a` and
  `A`, and `[X-c]`, which holds `X Y Z [ \ ] ^ _` `` ` `` `a b c`, also
  matches `x y z` and `A B C`.
- Everything else is unchanged: the dot, the predefined sets, the
  assertions, and all characters that are not ASCII letters. `é` does not
  match `É`. `k` does not match U+212A (the Kelvin sign), and `s` does not
  match U+017F (the long s).

The haystack is never changed, and every reported offset refers to the
original text. The cost of a pattern (section 6.1) is the same in both
modes.

## 4. What a pattern matches

### 4.1 Haystacks, positions and matches

A **haystack** is the `&str` that is searched. A **position** is an offset
into the haystack that lies on a character boundary; offset 0 and the
length of the haystack are positions. A haystack of `n` characters has
`n + 1` positions.

A **match** is a pair of positions, `start` and `end`, with
`start <= end`. A match with `start == end` is an **empty match**. The
library reports a match as the range `start..end`, so
`&haystack[start..end]` is always valid and is the text that was matched.

### 4.2 Where a match can end, and which end is preferred

Many patterns can match in several ways at the same position: `a|ab` can
match one or two characters of `ab`. This section defines, for an
expression `e` and a position `i`, the **list** `M(e, i)` of the positions
where a match of `e` that starts at `i` can end, in order of preference:
the first entry is the most preferred. A list may be empty (no match starts
at `i`), and it may hold the same position more than once; only the order
matters.

| expression `e` | `M(e, i)` |
|---|---|
| the empty expression (an empty pattern, group or branch) | the one position `i` |
| an atom that matches one character: a literal, an escaped character, `.`, a class, a predefined set | if a character starts at `i` and the atom accepts it: the one position after that character. Otherwise: empty |
| an assertion | the one position `i` if the assertion holds at `i`. Otherwise: empty |
| a group `(e1)` | `M(e1, i)` |
| a concatenation `e1 e2` | for each `j` in `M(e1, i)`, in order: all of `M(e2, j)` |
| an alternation `e1\|e2` | all of `M(e1, i)`, then all of `M(e2, i)` |
| `e1?` | all of `M(e1, i)`, then `i` |
| `e1+` | for each `j` in `M(e1, i)`, in order: all of `M(e1+, j)`, then `j` itself |
| `e1*` | all of `M(e1+, i)`, then `i` |
| `e1{n}` | as `n` copies of `e1` in a concatenation; `e1{0}` is the empty expression |
| `e1{n,}` | for `n >= 1`: as `n - 1` copies of `e1` followed by `e1+`. `e1{0,}` is `e1*` |
| `e1{n,m}` | as `n` copies of `e1` followed by `e1{0,m-n}`, where `e1{0,k}` is `(e1 e1{0,k-1})?` for `k >= 1` and `e1{0,0}` is the empty expression |

A concatenation or alternation of more than two parts is read from left to
right in the same way.

In words:

- **Alternation prefers the earlier branch.** Every way to match with the
  first branch comes before any way to match with the second.
- **Quantifiers are greedy.** One more repetition comes before stopping.
- **Earlier choices weigh more than later ones.** In `e1 e2`, the most
  preferred end of `e1` is tried with every end of `e2` before the second
  end of `e1` is considered.
- **A loop always ends.** Section 3.7 refuses a loop whose body can match
  the empty string, so in the row for `e1+` every `j` is greater than `i`:
  each round consumes at least one character, and the lists are finite.
- **Copies are not rounds.** `e1{n}` and `e1{n,m}` write out copies of
  `e1`, and a copy may match the empty string. `(|a){2}` in `aa` prefers
  two empty copies.

Examples. Each line gives a pattern, a haystack, the position `i`, and the
list.

```text
a|ab         ab     0    1, 2
ab|a         ab     0    2, 1
a*           aaa    0    3, 2, 1, 0
a+           aaa    0    3, 2, 1
a?           b      0    0
a{2,3}       aaaa   0    3, 2
(a|ab)(c|bcd) abcd  0    4, 3        (a then bcd; ab then c)
(a|b)*b      abab   0    4, 2        (the loop gives back until a b follows)
(a|ab)*      abab   0    1, 3, 4, 2, 0
(|a){2}      aa     0    0, 1, 1, 2
(a|){2}      aa     0    2, 1, 1, 0
a$           ab     0    (empty)
```

In the line for `(a|ab)*` the first round prefers the branch `a` and ends
at 1. No round can start at 1, so the loop stops there, and 1 is the most
preferred end: it comes before every way that begins with the branch `ab`.

### 4.3 The match

The match of a pattern `p` in a haystack is defined in two steps.

1. **The leftmost start.** `start` is the smallest position `s` for which
   `M(p, s)` is not empty. If there is no such position, there is no match.
2. **The preferred end.** `end` is the first entry of `M(p, start)`.

`find` returns this match. The rule is often called *leftmost-first*. It is
what a backtracking matcher reports that tries the positions from left to
right and, at each one, the alternatives in order. It is **not** the
*leftmost-longest* rule of POSIX tools: for `a|ab` in `ab`, the match here
is `0..1`.

```text
pattern         haystack   find
a|ab            ab         0..1    the earlier branch wins
ab|a            ab         0..2
b|ab            ab         0..2    an earlier start beats a preferred branch
b|^a            ab         0..1
a*              baaa       0..0    the empty match at 0 is the leftmost
a+              baaa       1..4
a{2,3}          aaaa       0..3
(a|ab)(c|bcd)   abcd       0..4
(a|b)*b         abab       0..4
(a|ab)*         abab       0..1    after a, no round can follow
(a|ab)*c        ababc      0..5    ...but no c follows at 1 or at 3
(|a){2}         aa         0..0    two copies, both empty
(|a){2}b        ab         0..2
(a?){2}a        aa         0..2    the second copy gives its a back
x*              (empty)    0..0
$               ab         2..2
\b              "  ab"     2..2
^$              a          none
```

The definition says which match is the right one. It is not a method: a
program that computes the lists of section 4.2 one entry after the other
needs exponential time on some patterns, and section 4.5 forbids that.

### 4.4 All the matches

`find_iter` reports the matches of a haystack from left to right. No two
of them overlap. The sequence is defined by this procedure, where `from`
is a position and `last` is a position or nothing:

```text
from = 0, last = nothing
repeat:
    Find the match (start, end) by the rule of 4.3, among the matches
    whose start is at or after `from`. If there is none: finish.
    If start < end:
        report the match; last = end; from = end
    If start == end:
        (rule 2) report the match unless last == end; last = end
        (rule 1) if end is the end of the haystack: finish
                 otherwise: from = the position after the character
                 that starts at end
```

- **Rule 1** makes the search move on after an empty match. It steps over
  one whole character, never over one byte.
- **Rule 2** drops an empty match that lies directly at the end of the
  match before it. The matches of `a*` in `baaa` are `0..0` and `1..4`;
  the empty match at 4 is not reported.
- A search that starts at `from` still **sees the whole haystack**. The
  assertions `^`, `\b` and `\B` look at the text before `from`. `^a`
  therefore matches `aaa` once, not three times, and it is wrong to search
  the slice `&haystack[from..]` as if it were a haystack.

```text
pattern   haystack   find_iter
a         banana     1..2, 3..4, 5..6
aa        aaaaa      0..2, 2..4
a|ab      abab       0..1, 2..3
(empty)   ab         0..0, 1..1, 2..2
(empty)   é          0..0, 2..2
x*        ab         0..0, 1..1, 2..2
a*        baaa       0..0, 1..4
a*        aab        0..2, 3..3
a|        ba         0..0, 1..2
\b        ab cd      0..0, 2..2, 3..3, 5..5
$         ab         2..2
^a        aaa        0..1
\Ba       aaa        1..2, 2..3
```

Two further requirements:

- The iterator is **lazy**. Making it searches nothing, and each call of
  `next` does the searching for the one match it returns and no more.
  Taking the first three matches of a long haystack must not cost a search
  for all of them.
- After `next` has returned `None`, it keeps returning `None`.

The first item of `find_iter` is always the match of `find`.

### 4.5 Time and memory

Let `c` be the cost of the pattern (section 6.1) and `n` the length of the
haystack in bytes.

- One call of `is_match` or `find`, and one call of `next` on the iterator
  of `find_iter`, must take time at most proportional to
  `(c + 1) * (n + 1)`. Deciding whether one character is in a class may
  take a number of steps that grows with the logarithm of the size of the
  class; nothing else may be added.
- `matching_lines` must take time at most proportional to `(c + 1)` times
  the length of the text.
- The bound holds for **every** pattern and **every** haystack. It covers
  the whole call: a `find` that tries the start positions one after the
  other, and reads to the end of the haystack from each of them, is
  quadratic in `n` and does not meet it.
- The memory that a search uses, beyond the compiled pattern itself, must
  be at most proportional to `c + 1`. It must not grow with `n`.
- Neither compiling nor searching may use call stack that grows with `n`,
  with the length of the pattern, or with the number of branches or atoms.
  Only the nesting depth of groups, which section 6 limits to 200, may
  deepen a recursion. A stack overflow is a crash, and a crash is a failed
  test.

The tests do not measure speed. They run searches that are small under
these bounds and hopeless otherwise, each with a limit of 30 seconds. In
every search of the tests, `(c + 1) * (n + 1)` is at most about 10^8, so
an engine that does a little work for every state at every position has
time to spare, also in a debug build. The reference solution needs less
than two seconds for each test in a debug build.

## 5. Pattern errors

`Regex::new` must refuse every pattern that sections 3 and 6 do not allow,
with exactly one error. An error has a **message**, which is one of the
sixteen fixed texts below, and an **offset** into the pattern.

### 5.1 Which error is reported

A pattern can be wrong in several ways at once. The error that is reported
is the one a reader meets first when reading the pattern once, from left to
right:

1. Each error of section 5.2 is **noticed** at a certain character of the
   pattern, or at its end. The table says where. Among several errors, the
   one that is noticed first is reported.
2. Two errors can be noticed at the same place only at the end of the
   pattern, when several constructs are still open. Then the error of the
   construct that was opened **last** is reported.
3. `pattern too large` is reported only for a pattern that has no other
   error.

The offset of an error is usually not the place where it is noticed. It
points at the character a user has to look at: the `(` that was never
closed, not the end of the pattern.

Examples of the rule:

```text
pattern      message                          offset
a)(?         unmatched closing parenthesis    1    the ) comes before the (?
(a[b         missing closing bracket          2    both open at the end; [ was opened last
*\q          nothing to repeat                0
\q*(         unknown escape                   0
[z-a](       invalid class range              1
((a          missing closing parenthesis      1    the inner group
(a{2         invalid repetition               2
[a\          incomplete escape                2
```

### 5.2 The errors

| message | the pattern has | noticed at | offset |
|---|---|---|---|
| `missing closing parenthesis` | a group that is still open at the end of the pattern | the end of the pattern | the `(` of the open group that was opened last |
| `unmatched closing parenthesis` | a `)` while no group is open | that `)` | that `)` |
| `nesting too deep` | a `(` while 200 groups are already open (section 6) | that `(` | that `(` |
| `unsupported group syntax` | a `(` directly followed by `?` | the `?` | the `(` |
| `missing closing bracket` | a class that is still open at the end of the pattern | the end of the pattern | the `[` that opened the class |
| `empty character class` | `[]` or `[^]` | the `]` | the `[` |
| `invalid class range` | a range `X-Y` in a class where `Y` is a set escape, or where `Y` comes before `X` | the end of `Y` | the start of `X` |
| `unsupported class syntax` | an unescaped `[` inside a class | that `[` | that `[` |
| `nothing to repeat` | one of `*`, `+`, `?`, `{` where an atom is expected, or directly after a quantifier | that character | that character |
| `invalid repetition` | a `{` after an atom that does not begin a well-formed count, or a well-formed count `{n,m}` with `n > m` | see below | the `{` |
| `repetition count too large` | a well-formed count in which `n` or `m` is greater than 1000 | the `}` | the `{` |
| `repeated expression can be empty` | a loop `*`, `+` or `{n,}` after an expression that can match the empty string (section 3.7) | the `*` or `+`, or the `}` of the count | the `*`, the `+` or the `{` |
| `unknown escape` | a `\` followed by a character that section 3.3 gives no meaning, or `\b` or `\B` inside a class | the character after the `\` | the `\` |
| `incomplete escape` | a `\` as the last character of the pattern | the end of the pattern | the `\` |
| `invalid Unicode escape` | `\u` that is not followed by `{`, one to six hexadecimal digits and `}`; or whose number is not a Unicode scalar value | the first character that breaks the form, the `}` for a bad number, or the end of the pattern | the `\` |
| `pattern too large` | no other error, and a cost above 100 000 (section 6.1) | after everything else | 0 |

Details:

- "Where an atom is expected" means: at the start of the pattern, directly
  after a `(`, and directly after a `|`.
- An offset is a byte offset. In `é(a` the `(` is at offset 2. Every
  offset in the table is the offset of a character of the pattern, so it
  lies on a character boundary.
- If an escape inside a class is wrong, the error is the error of the
  escape (`unknown escape`, `incomplete escape`, `invalid Unicode
  escape`), with the offset of its `\`.
- If the second end of a class range cannot be read at all, the error is
  the one for that character: `[a-\q]` is `unknown escape` at 3, and
  `[a-[]` is `unsupported class syntax` at 3.

**A count is judged in four steps**, and the first step that fails gives
the error. All of these errors have the offset of the `{`.

1. *Form.* After the `{` come one or more digits; then either `}`, or `,}`,
   or `,` with one or more digits and `}`. Anything else is `invalid
   repetition`. It is noticed at the first character that does not fit, or
   at the end of the pattern. `a{`, `a{}`, `a{,3}`, `a{2`, `a{2,x}`,
   `a{ 2}`, `a{-1}` and `a{1,2,3}` all fail here. So does `a{2000`, however
   large its number is.
2. *Size.* If `n` or `m` is greater than 1000: `repetition count too
   large`. A count may have any number of digits; a number that is too
   large for the integer type you use is still just "greater than 1000".
3. *Order.* If `n > m`: `invalid repetition`. So `a{3,2}` is `invalid
   repetition`, and `a{2000,1}` is `repetition count too large`.
4. *Loop.* If the count is `{n,}` and the expression before it can match
   the empty string: `repeated expression can be empty`. So `(a*){2,}` is
   this error, and `(a*){2000,}` is `repetition count too large`.

A quantifier is judged where it stands, before anything after it is read:
`()**` is `repeated expression can be empty` at offset 2, not `nothing to
repeat` at offset 3.

More examples:

```text
pattern          message                          offset
a(b              missing closing parenthesis      1
(a(b)(c|(d       missing closing parenthesis      8
ab)              unmatched closing parenthesis    2
(?:a)            unsupported group syntax         0
x[^ab            missing closing bracket          1
ab[]c            empty character class            2
[ab-a]           invalid class range              2
[a-\d]           invalid class range              1
[[:alpha:]]      unsupported class syntax         1
a|*              nothing to repeat                2
a+?              nothing to repeat                2
a{2}{3}          nothing to repeat                4
{2}              nothing to repeat                0
ab{1,2,3}        invalid repetition               2
(a){3,2}         invalid repetition               3
a{1001}          repetition count too large       1
(a*)*            repeated expression can be empty 4
(a|)+b           repeated expression can be empty 4
x(y|(z?))+       repeated expression can be empty 9
^*               repeated expression can be empty 1
(a?){2,}         repeated expression can be empty 4
ab\z             unknown escape                   2
[\b]             unknown escape                   1
abc\             incomplete escape                3
ab\u{110000}     invalid Unicode escape           2
\u41             invalid Unicode escape           0
日本語(          missing closing parenthesis      9
```

## 6. Limits

Three limits keep every pattern cheap to compile and every search cheap to
run. A pattern that breaks one of them is refused with an error of section
5.

| limit | value | error |
|---|---|---|
| a count `n` or `m` in `{n}`, `{n,}`, `{n,m}` | at most 1000 | `repetition count too large` |
| groups that are open at the same place | at most 200 | `nesting too deep` |
| the cost of the whole pattern (section 6.1) | at most 100 000 | `pattern too large` |

The nesting limit counts groups that are open at the same time, not groups
in total. `(a)` written a thousand times is fine. In a pattern that begins
with 201 opening parentheses, the error is at offset 200.

There is no limit on the length of a pattern or of a haystack, and none on
the number of characters in a class.

### 6.1 The cost of a pattern

The cost is a number that is computed from the pattern alone. It is the
`c` in the time bound of section 4.5.

| expression | cost |
|---|---|
| the empty expression (an empty pattern, group or branch) | 0 |
| a literal or an escaped character | 1 |
| `.`, a class (with any number of items), a predefined set | 1 |
| an assertion `^`, `$`, `\b`, `\B` | 1 |
| a group `(e)` | the cost of `e` |
| a concatenation | the sum of the costs of its parts |
| an alternation with `k` branches | the sum of the costs of the branches, plus `k - 1` |
| `e?` | cost of `e`, plus 1 |
| `e*`, `e+` | cost of `e`, plus 1 |
| `e{n}` | `n` times the cost of `e` |
| `e{n,}` with `n >= 1` | `n` times the cost of `e`, plus 1 |
| `e{0,}` | as `e*` |
| `e{n,m}` | `m` times the cost of `e`, plus `m - n` |

Examples:

```text
pattern                        cost
abc                            3
a|b|c                          5        3 + 2
(a|b)*c                        5        (1 + 1 + 1) + 1 + 1
\d{2,4}                        6        4 * 1 + (4 - 2)
[a-z0-9._]+@                   3
^\s*(#.*)?$                    8        1 + 2 + (1 + 2) + 1 + 1
a{1000}                        1000
(a{100}){1000}                 100 000  the largest cost that is allowed
(a{100}b){1000}                101 000  pattern too large
((a{1000}){1000}){1000}        10^9     pattern too large
(){1000}                       0
```

The cost is a mathematical number. Seven groups with `{1000}` inside each
other cost 10^21, which no 64-bit integer holds. Such a pattern is `pattern
too large` like any other; a cost that wraps around to a small number is a
bug that the tests look for.

### 6.2 What compiling may cost

- `Regex::new` must take time and memory at most proportional to the
  length of the pattern plus its cost, where sorting the items of a class
  may add a logarithmic factor. With the limit of section 6.1, compiling
  never takes more than a moment.
- A pattern whose cost is above the limit must be refused **before**
  anything is built whose size depends on that cost. A count of 1000
  around a count of 1000 around a count of 1000 must be an error after a
  few microseconds, not after a gigabyte.
- A pattern of cost 0, such as `((){1000}){1000}` nested twenty deep, is
  valid. It matches the empty string, and compiling it must not take
  1000^20 steps of anything.
- Section 4.5 forbids deep recursion while compiling.

## 7. The program

### 7.1 The command line

```text
rgx [OPTIONS] PATTERN [PATH...]
```

| option | long form | effect |
|---|---|---|
| `-i` | `--ignore-case` | compile the pattern in the mode of section 3.9 |
| `-v` | `--invert-match` | select the lines that do **not** contain a match |
| `-n` | `--line-number` | print the line number before each line |
| `-c` | `--count` | print only the number of selected lines of each input |
| `-l` | `--files-with-matches` | print only the names of the inputs that have a selected line |
| `-o` | `--only-matching` | print each match on a line of its own |
| `-j N` | `--threads N` | search with `N` threads |
| `-h` | `--help` | print a help text |
| `--` | | every argument after this one is an operand |

How the arguments are read:

- Before a `--`, an argument that begins with `-` and is longer than one
  character is an **option**. Every other argument is an **operand**. A
  lone `-` is an operand like any other; it does not mean standard input.
- An option must be spelled exactly as in the table, as an argument of its
  own. There is no bundling (`-in`), no attached value (`-j4`,
  `--threads=4`) and no abbreviation (`--ignore`). Anything else that
  looks like an option is a usage error.
- `-j` and `--threads` take the **next argument** as their value, whatever
  it looks like. The value must consist of ASCII digits only and must be a
  number from 1 to 256. Leading zeros are allowed (`-j 007`); a sign is
  not (`-j +4`).
- Options and operands may be mixed in any order: `rgx fog -n` is `rgx -n
  fog`.
- An option may be given more than once. Giving a flag twice is the same
  as giving it once. If `-j` is given several times, the last value
  counts.
- The first operand is the pattern. All further operands are paths
  (section 7.3). The pattern is taken exactly as the argument: `rgx -- -n`
  searches for the two characters `-n`.

**Usage errors** (section 7.5 gives the message, section 7.6 the status):

- there is no pattern;
- an unknown option;
- `-j` or `--threads` without a value, or with a value that is not a
  number from 1 to 256 written in digits;
- two different ones of `-c`, `-l` and `-o`;
- `-o` together with `-v`;
- a pattern that is not valid Unicode.

`-n` together with `-c` or `-l` is not an error: it has no effect there.

**Help.** If any argument before a `--` is exactly `-h` or `--help`, the
program prints a help text on standard output and exits with status 0. It
checks nothing else, reads nothing, and prints nothing on standard error,
even if other arguments are wrong. The place of the argument does not
matter: `rgx x -j --help` prints the help text, although `--help` stands
where `-j` expects its value. "Before a `--`" means, for this rule: before
the first argument that is exactly `--`. After it, `-h` is an operand:
`rgx -- -h` searches for the two characters `-h`. The text must begin with
`Usage: rgx`. The rest of it is yours; it should name every option.

**Order of the checks.** First the usage errors, then the pattern, then
the search. A usage error is reported even if the pattern is invalid too. A
pattern error is reported before any path is looked at.

### 7.2 Lines, selected lines, and what is printed

The text of every input (section 7.3) must be valid UTF-8 (section 7.4).
It is divided into **lines**:

- A line ends at a line feed. The line feed is not part of the line.
- Text after the last line feed is a line too, if there is any. So `a` +
  `LF` + `b` has two lines, and so has `a` + `LF` + `b` + `LF`. There is
  no empty line after a final line feed.
- The empty text has no lines. The text that is one line feed has one
  line, which is empty.
- A carriage return is an ordinary character. A line of a file with
  Windows line ends has a `CR` as its last character, and `b$` does not
  match `ab` + `CR`.
- Lines are numbered from 1.

`Regex::matching_lines` uses the same definition.

Each line is searched as a haystack of its own, so no match ever spans two
lines, and `^` and `$` are the two ends of a line.

A line is **selected** if it contains a match (`is_match`). With `-v` a
line is selected if it contains **no** match.

What is printed for each input, by mode:

| mode | output |
|---|---|
| (default) | each selected line: `LINE`, then a line feed |
| `-o` | for each selected line, each match of section 4.4 that is **not empty**: the matched text, then a line feed |
| `-c` | one line: the number of selected lines, in decimal. It is printed also when the number is 0 |
| `-l` | if at least one line is selected: one line with the name of the input. Otherwise nothing |

Details:

- In the default mode and with `-o`, each printed line gets up to two
  **prefixes**: first `NAME:` if names are shown (section 7.3), then
  `NUMBER:` if `-n` was given, where `NUMBER` is the number of the line in
  its input. With `-c`, the count gets the prefix `NAME:` if names are
  shown. `-l` prints the bare name.
- Every line of output ends with a line feed, also when the last line of
  the input did not.
- With `-o`, a line is selected when it has at least one match, empty or
  not. Empty matches are never printed. `rgx -o 'x*'` therefore prints
  nothing for the input `abc`, and still exits with status 0, because a
  line was selected.
- The name of standard input, for `-l` and for error messages, is
  `(standard input)`.

The examples use this file `inventory.txt` (the fifth line is empty) and a
file `orders.txt` with the two lines `order 66: 40 bolts` and `order 67: 2
nuts`:

```text
12 bolts
7 nuts
bolt cutter
3 washers

120 Bolts
```

```text
$ rgx bolt inventory.txt
12 bolts
bolt cutter
$ rgx -n -i bolt inventory.txt
1:12 bolts
3:bolt cutter
6:120 Bolts
$ rgx -c '^\d+ ' inventory.txt
4
$ rgx -o '\d+' inventory.txt
12
7
3
120
$ rgx -v -n '\d' inventory.txt
3:bolt cutter
5:
$ rgx -n 'bolts$' inventory.txt orders.txt
inventory.txt:1:12 bolts
orders.txt:1:order 66: 40 bolts
$ rgx -c nuts orders.txt inventory.txt
orders.txt:1
inventory.txt:1
$ rgx -l -i '^bolt' inventory.txt orders.txt
inventory.txt
```

(The quotes belong to the shell, not to the pattern.)

### 7.3 Inputs, their names and their order

An **input** is one text that is searched. Each input has a **name**.

**Without a path operand** there is one input: standard input, read to its
end. **With at least one path operand**, standard input is not read at all.

Each path operand is looked at, in the order of the command line:

- **A file** is one input. Its name is the operand, exactly as it was
  given: `./a.txt` stays `./a.txt`, and an absolute path stays absolute.
- **A directory** stands for all the regular files in it and below it.
  The entries of a directory are taken in the order of their names,
  compared **byte by byte** as UTF-8 (so `B` comes before `_`, which comes
  before `a`, and `10` comes before `9`). A file among them is an input. A
  directory among them is walked completely, in the same way, at the place
  of its name, before the next entry is taken. The name of an entry is the
  name of its directory, then `/`, then the entry's own name. The
  separator is `/` on every platform. If the operand itself already ends
  in `/`, no second `/` is added after it.
- **Anything that cannot be examined** (it does not exist, or the
  operating system refuses) is an input that fails; see section 7.4.

Further rules:

- The same file given twice is two inputs.
- An operand that is a symbolic link is treated as the thing it points to.
  A symbolic link that is **found inside** a directory is passed over: it
  is not searched and not followed. So is everything else that is neither
  a regular file nor a directory.
- An empty directory contributes no inputs. That is not an error.

For the directory

```text
store/Zinc.txt
store/a.d/x.txt
store/a.txt
store/spare/bin.txt
```

the command `rgx -c bolts store inventory.txt` has five inputs, in exactly
this order: the four files as listed (`Z` is the byte 0x5A, before `a`;
`a.d` comes before `a.txt` because `d` comes before `t`), then
`inventory.txt`.

**Names are shown** (the prefix `NAME:` of section 7.2) if there are two or
more path operands, or if at least one path operand is a directory.
Otherwise they are not: not for standard input, and not for one single
file.

**The order of the output is the order of the inputs.** Everything that
one input contributes to standard output comes before everything of the
next input, and within one input the lines come in the order of the text.
The messages on standard error (section 7.4) are in the order of the
inputs too. How the two streams interleave with each other is not
specified.

**Threads.** `-j N` sets the number of threads that search. With more than
one thread the program should search several inputs at the same time.
Without `-j` it should use as many threads as the machine offers
(`std::thread::available_parallelism`). The number of
threads must have **no effect on the output**: standard output, standard
error and the exit status are the same for every `N`, and the same on
every run. A program that prints each input as soon as some thread has
finished it does not meet this.

### 7.4 Inputs that cannot be searched

The program reports each of the following on standard error with one line
of the form of section 7.5, goes on with the remaining inputs, and exits
with status 2 at the end.

- **An input that cannot be read**: an operand that does not exist, and
  any file, named or found in a directory, that may not be opened or whose
  reading fails. Also a directory, at any depth, whose entries cannot be
  listed; its name is formed as in section 7.3.
- **An operand, or standard input, that is not valid UTF-8.** The message
  is `invalid UTF-8 at byte N`, where `N` is the length in bytes of the
  longest prefix of the input that is valid UTF-8. In other words, `N` is
  the offset of the first byte that cannot be part of a valid text. A
  sequence that is cut off by the end of the file counts from its first
  byte.

An input that fails contributes **nothing** to standard output: no line,
no count, no name. This holds also for the lines before a bad byte.

A file that is not valid UTF-8 and was **found by walking a directory** is
different: it is passed over without a message. It contributes nothing, it
is not an error, and it does not change the exit status. (A directory tree
usually holds some files that are not text. A file that was named on the
command line is one the user asked for.)

There is no special treatment of a byte order mark. U+FEFF at the start of
a file is the first character of its first line.

### 7.5 Messages

Everything on standard error is a line that begins with `rgx: `. Nothing
else is ever written there.

**A usage error** is one line:

```text
rgx: usage error: TEXT
```

`TEXT` is yours; it should say what is wrong. Nothing is printed on
standard output and nothing is searched.

**A pattern error** is three lines:

```text
rgx: pattern error at byte OFFSET: MESSAGE
    PATTERN
    <spaces>^
```

- `OFFSET` and `MESSAGE` are the offset and the message of section 5.
- The second line is four spaces and then the pattern as it was given.
- The third line is four spaces, then one more space for every
  **character** of the pattern before the offset, then `^`. The message
  counts bytes and the caret counts characters, so that the caret stands
  under the right character after `é` or `日`.

```text
$ rgx 'a(b|c' inventory.txt
rgx: pattern error at byte 1: missing closing parenthesis
    a(b|c
     ^
$ rgx 'é(a'
rgx: pattern error at byte 2: missing closing parenthesis
    é(a
     ^
```

Nothing is printed on standard output, and no input is read.

**An input that fails** is one line:

```text
rgx: NAME: REASON
```

`NAME` is the name of the input by section 7.3. For invalid UTF-8,
`REASON` is exactly `invalid UTF-8 at byte N`. For every other failure it
is a short text of yours, usually the one the operating system gives; the
tests look only at the line up to the second `: `.

```text
$ rgx -l bolts missing.txt store/b.dat store/a.txt
rgx: missing.txt: No such file or directory (os error 2)
rgx: store/b.dat: invalid UTF-8 at byte 6
```

### 7.6 Exit status

| status | when |
|---|---|
| 0 | at least one line was selected in some input, and there was no error |
| 1 | no line was selected, and there was no error |
| 2 | a usage error, a pattern error, or at least one input that failed |

Status 2 wins: if one input failed, the status is 2 even when lines of
other inputs were selected and printed. `--help` exits with 0.

### 7.7 Writing the output

- Results go to standard output and to nowhere else. The output must be
  complete and in order however large it is.
- If standard output is **closed by its reader** before the program has
  finished (as in `rgx x big.txt | head -1`), the program must stop soon,
  must not print a message about it, and must not panic. The exit status
  is not specified in this case. Every other part of this section still
  holds for what was written before.
- Any other error while writing should be reported as `rgx: TEXT` with
  status 2.
- The program should not hold more of its output in memory than it has to
  in order to keep it in order.

## 8. Memory

### 8.1 Lines are slices

Every line that `matching_lines` returns must be a **slice of the `text`
that was passed in**: the same memory, not a copy with the same content.
The tests compare addresses.

The program should work the same way: read an input once, and search and
print lines as slices of that one buffer, without making a `String` per
line.

### 8.2 The allocation budget

For a pattern whose cost (section 6.1) is 50 or less:

- one complete pass of `find_iter` over a haystack, and
- one complete pass of `matching_lines` over a text,

may each make **at most 100 heap allocations** on the calling thread,
however long the haystack is and however many matches or lines there are.
A pass is: making the iterator, calling `next` until it returns `None`,
and dropping the iterator. An allocation is one call of the allocator that
hands out or resizes memory (`alloc`, `alloc_zeroed` or `realloc`).
Compiling the pattern is not part of the pass and is not counted.

The limit is generous for work that is done once per pass and impossible
for work that is done once per line, per match or per character.

## 9. Build rules and non-goals

Build rules:

- **The standard library only.** `[dependencies]` in `Cargo.toml` stays
  empty. This is part of the assessment: every piece is yours.
- The package keeps its layout: a library crate `rgx` in `src/lib.rs` and
  a binary `rgx` in `src/main.rs`. The program should be a thin layer over
  the library.
- The code must build without warnings in the debug and in the release
  profile and behave the same in both.
- It must work on Linux, macOS and Windows. Do not start other programs
  and do not depend on signals.
- You do not need `unsafe`. If you use it, the rubric applies its
  unsafe-hygiene rows, and the library tests must pass under Miri (README,
  "How to assess your work").
- Do not change the files in `tests/`. Add tests of your own next to your
  code, or in new files.

Not part of this project, and not tested:

- capturing groups, back-references, replacement;
- look-ahead and look-behind;
- lazy and possessive quantifiers;
- a meaning for loops whose body can match the empty string (they are
  errors, section 3.7);
- flags inside the pattern such as `(?i)`; multi-line and dot-matches-all
  modes;
- POSIX classes (`[[:alpha:]]`), Unicode properties (`\p{L}`), class
  intersection and subtraction;
- Unicode-aware `\d`, `\w`, `\s`, `\b`; case folding beyond ASCII;
  normalisation; grapheme clusters;
- the leftmost-longest rule;
- haystacks that are not UTF-8, other encodings, byte order marks;
- special treatment of `CR` + `LF` line ends;
- several patterns in one run, patterns from a file;
- context lines, colours, column numbers, a `-` operand for standard
  input;
- ignore files, hidden files, file-name filters, following symbolic links
  inside directories;
- searching an input before it has been read completely, memory-mapped
  files, inputs larger than memory;
- file names that are not valid Unicode (the program should still search
  such files; how it prints their names is yours).

## 10. Milestones and their tests

Work in this order. Each milestone has one test file in `tests/`; run it
with `cargo test --test <name>`. A later milestone never breaks an earlier
test.

| milestone | you build | sections | test file | tests |
|---|---|---|---|---|
| M1 | reading a pattern: every valid form is accepted, every error has its message and offset, the count and nesting limits | 2.2, 3, 5, 6 | `m1_syntax` | 13 |
| M2 | matching: `is_match` and `find` with the leftmost-first rule, UTF-8 throughout | 2.1, 3, 4.1 to 4.3 | `m2_matching` | 21 |
| M3 | the guarantee: linear time, bounded stack, the cost limit, cheap compiling | 4.5, 6.1, 6.2 | `m3_linear_time` | 10 |
| M4 | `find_iter` and `matching_lines`, lines as slices, the allocation budget, one `Regex` on many threads | 2.1, 4.4, 7.2 (lines), 8 | `m4_lines_alloc` | 15 |
| M5 | the program on standard input and on named files: options, output modes, `-i`, messages, exit status | 3.9, 7.1, 7.2, 7.4 to 7.6 | `m5_cli` | 34 |
| M6 | directories, many files on many threads with one deterministic output, large outputs, a reader that goes away | 7.3, 7.4, 7.7 | `m6_parallel` | 13, and one measurement |

What turns green when:

- **M1**: every test in `m1_syntax`. Nothing is matched yet;
  `valid_patterns_compile` and the four tests named `*_errors` come first,
  then `a_loop_needs_something_to_consume`, then the tests of the limits.
- **M2**: every test in `m2_matching`. A matcher that is correct and slow
  passes this file; it is the right place to get the meaning of section 4
  exact before making it fast. The tests
  `loops_give_back_what_the_rest_needs` and `counted_copies_may_be_empty`
  are the ones to watch.
- **M3**: every test in `m3_linear_time`. If your M2 matcher tries the
  possibilities one after the other, this is where it has to be replaced.
  `the_cost_limit_is_exact` needs only the cost of section 6.1 and can be
  made green first.
- **M4**: every test in `m4_lines_alloc`. The three budget tests at the
  end of the file are the last to turn green.
- **M5**: every test in `m5_cli`. The tests of standard input come first
  in the file, then the options, then named files, then the errors.
- **M6**: every test in `m6_parallel`.
  `a_directory_is_searched_with_everything_below_it` passes with one
  thread; `the_output_is_the_same_for_every_thread_count` is the summary
  of the milestone.

About the tests:

- They are black-box tests. The library tests call the facade of section
  2; the program tests start your binary in an empty temporary directory
  and compare standard output, standard error and the exit status with
  this document.
- **No test hangs.** The library calls of one test have 30 seconds
  together, and so has each run of the binary. A search that does not end
  fails one test with a message that says `TIMEOUT`. A program that prints
  without end is stopped after 32 MiB of output.
- No test measures speed, and none depends on the number of processors.
- Two tests feed generated patterns to the library
  (`no_pattern_makes_regex_new_panic` in M1,
  `no_search_panics_and_every_match_is_a_slice` in M2). Their generator
  has a fixed seed: they do the same on every run and on every machine.
- Two tests need what only Unix offers a test in a portable way: an
  argument that is not valid Unicode (M5) and symbolic links (M6). On
  other systems they pass without checking anything.
- `m3_linear_time` and `m4_lines_alloc` install an allocator that counts
  allocations and refuses to let the test program hold more than one
  gibibyte. A pattern that makes your compiler build something enormous
  ends that test program with `memory allocation of N bytes failed`.
- Temporary directories are removed and child processes are ended when a
  test finishes, also when it fails.
- `m6_parallel` contains one test that is marked `#[ignore]`:
  `speed_report` prints how long your program needs for a fixed job with
  1, 2, 4 and 8 threads. It cannot fail because of a number. The rubric
  uses it.

## 11. Stretch goals

None of these is tested. Each one is a real technique from real engines
and tools. Do the ones that interest you, and measure before and after.

1. **Capturing groups.** Report where each group matched, by the same
   preference rules. Decide what a group inside a loop reports, write it
   down as precisely as section 4.2, and keep the time bound.
2. **A second engine.** Build a deterministic automaton from yours, lazily
   and with a bounded cache, for the patterns and the searches that allow
   it. Keep the first engine as the reference and test the two against
   each other.
3. **Literal prefilters.** Many patterns can only match where a certain
   literal text occurs. Find such a text at compile time and skip to its
   occurrences before the automaton runs. Measure with `speed_report`.
4. **Bytes.** Compile to an automaton over bytes that decodes UTF-8 as it
   goes, so that the matcher never decodes a character. Then decide what a
   search of a file that is not UTF-8 could mean.
5. **Unicode.** Give `\w`, `\d`, `\s`, `\b` and `-i` a Unicode-aware mode.
   The data has to come from somewhere: generate tables from the Unicode
   Character Database with a build script.
6. **Streaming.** Search standard input and very large files with bounded
   memory, and print matches as they are found. State what changes in
   section 7.4 when a bad byte is found after output has been written.
7. **More of grep.** Context lines (`-A`, `-B`, `-C`), several patterns
   (`-e`), a maximum count (`-m`), colour when standard output is a
   terminal, `--files` to list what would be searched.
8. **Ignore rules.** Skip hidden files and what a `.gitignore` excludes;
   that is a small pattern language of its own.
9. **Loops that can go round empty.** Give `(a*|b)*` and `(|a)+` a
   meaning instead of an error. Read what Perl, JavaScript and one
   automaton-based engine do with them, choose one rule, write it down as
   exactly as section 4.2, and implement it without losing the time bound.
   This is harder than it looks.
10. **A proof by testing.** Write the definition of section 4 as a second,
   slow matcher, generate patterns and haystacks at random with a fixed
   seed, and compare. Then do the same against another engine and explain
   every difference you find.
11. **Fuzzing.** Fuzz `Regex::new` and the search with `cargo fuzz`, with
    the assertion that no input panics and that every reported range is a
    valid slice.
