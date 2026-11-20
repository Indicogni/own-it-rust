# Chapter 17 notes — Working with text

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Unicode scalar operations and encoding

**Predictions I got wrong, and why:**

- 

**Explain:** Why can case conversion yield multiple scalars? Why is reversing logical scalar order unsuitable as a text-direction algorithm?

> 

**Edge case I added:**

- 

## l02 · Constructing and editing owned text

**Predictions I got wrong, and why:**

- 

**Explain:** Why must insertion boundaries be checked? Which operations keep the original allocation versus return a new String?

> 

**Edge case I added:**

- 

## l03 · Search patterns, byte positions, and replacement

**Predictions I got wrong, and why:**

- 

**Explain:** How does a literal string pattern differ from a char or character predicate? Why does replacen need an explicit replacement count?

> 

**Edge case I added:**

- 

## l04 · Text iteration, trimming, and casing

**Predictions I got wrong, and why:**

- 

**Explain:** Why is lowercasing not a universal Unicode case-insensitive comparison? Why do chars and bytes answer different questions?

> 

**Edge case I added:**

- 

## l05 · FromStr and textual conversion

**Predictions I got wrong, and why:**

- 

**Explain:** Why should a parser return an error rather than quietly truncate? How do parse and to_string use traits?

> 

**Edge case I added:**

- 

## l06 · The formatting language

**Predictions I got wrong, and why:**

- 

**Explain:** How do named/indexed arguments and dynamic parameters make a formatter reusable? Why should pointer addresses never be hard-coded in tests?

> 

**Edge case I added:**

- 

## l07 · Display, fmt::Write, and format_args!

**Predictions I got wrong, and why:**

- 

**Explain:** Why is fmt::Error for formatting machinery failures rather than input validation? How does format_args! pass formatting without an intermediate String?

> 

**Edge case I added:**

- 

## l08 · UTF-8, borrowed text views, and delayed allocation

**Predictions I got wrong, and why:**

- 

**Explain:** Why should OS strings and paths not be forced through to_str().unwrap()? Which conversions guarantee UTF-8 and which preserve arbitrary bytes?

> 

**Edge case I added:**

- 

## l09 · Capstone: a Unicode-aware report parser

**Predictions I got wrong, and why:**

- 

**Explain:** Why are byte offsets safe at separator boundaries? What errors should a display-oriented parser preserve rather than ignore?

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: earlier chapters inside Chapter 17

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Explain the difference between bytes, Unicode scalars, grapheme clusters and terminal columns, and say which of them `len()`, `chars().count()` and a format width count.**

> 

**Say which `String` and `str` operations take or return byte offsets, what happens at an offset inside a character, and how you get offsets that are always safe.**

> 

**Explain why changing case can change the length of a text, why lowercasing a whole string can differ from lowercasing each char, and what `String::from_utf8` does with the buffer when it succeeds and when it fails.**

> 

**Implement `FromStr` and `Display` for a small type from memory. Say what `fmt::Error` means, and how a function writes formatted text into any destination without a temporary `String`.**

> 

**Explain when a `Cow<str>` allocates, and why file paths are not `String`s.**

> 

**Explain why a regex is compiled once in a `LazyLock`, and choose between `is_match`, `find` and `captures` for a given job.**

> 

**Redo the capstone after a break without looking at your first solution. Then invent a changed requirement and explain what it changes in the design.**

>
