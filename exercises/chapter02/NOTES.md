# Chapter 2 notes — A first tour

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Variables, mutability and basic types

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the difference between changing a `mut` variable and shadowing a name with a second `let`, and say why Rust makes you write `mut`.

> 

**Edge case I added:**

- 

## l02 · Functions, expressions and statements

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the difference between an expression and a statement, and what the final line of a function body without a semicolon means.

> 

**Edge case I added:**

- 

## l03 · if, loops and ranges

**Predictions I got wrong, and why:**

- 

**Explain:** Say when you choose `for`, when `while` and when `loop`, and what the difference between `a..b` and `a..=b` is.

> 

**Edge case I added:**

- 

## l04 · Text: &str, String and formatting

**Predictions I got wrong, and why:**

- 

**Explain:** Explain when a function should take `&str` and when it must return `String`, and what `format!` gives you that `println!` does not.

> 

**Edge case I added:**

- 

## l05 · Vectors: a list that can grow

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what `for x in &values` gives you in each round, and why a function that only reads a list takes `&[i32]`.

> 

**Edge case I added:**

- 

## l06 · Structs and methods

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what `&self` and `&mut self` mean in a method, and why `Novel::new` has no `self` at all.

> 

**Edge case I added:**

- 

## l07 · Enums and match

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what "a match must be exhaustive" means, and what you gain when you list every variant and do not write `_`.

> 

**Edge case I added:**

- 

## l08 · Option and Result at first contact

**Predictions I got wrong, and why:**

- 

**Explain:** Say when a function should return `Option` and when `Result`, and why Rust has no "null" value that every variable may hold.

> 

**Edge case I added:**

- 

## l09 · Writing your own tests

**Predictions I got wrong, and why:**

- 

**Explain:** Explain how you chose your test cases, and why four tests of ordinary years would not have been enough.

> 

**Edge case I added:**

- 

## l10 · Capstone: a text statistics tool

**Predictions I got wrong, and why:**

- 

**Explain:** Describe your design: which types and functions you made, what each one is responsible for, and one thing you would change if the tool had to grow to ten commands.

> 

**Edge case I added:**

- 

## c01 · A teammate's first pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Explain `let`, `let mut` and shadowing with one example each, and say when you use which.**

> 

**Say what a function returns, what a semicolon on the last line changes, and what `()` means in a compiler message.**

> 

**Write from memory a `for` loop over `1..=n`, a `while` loop, and a `loop` with `break`. Say how many rounds `0..5` and `0..=5` make.**

> 

**Explain the difference between `&str` and `String`, and which of them a function should take and which it should return.**

> 

**Define a struct with one method that reads and one that changes, and an enum with a variant that carries data. Write a `match` over the enum.**

> 

**Explain what `Option` and `Result` are for, and show three ways to get the value out.**

> 

**Write a test module from memory, with one ordinary test and one `#[should_panic]` test. Name the kinds of test case that you look for.**

> 

**Redo the capstone after a break without looking at your first solution. Then add a fifth command of your own.**

>
