# Mastery M5 notes — Procedural macros

Your workbook. Write answers in your own words, *before* checking the hints. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · A derive with nothing but `proc_macro`

**Predictions I got wrong, and why:**

- 

**Explain:** What does a derive macro receive, what does it return, and where does the returned code go? Why did splitting at commas need extra care, and what does that tell you about tokens and syntax?

> 

**Edge case I added:**

- 

## l02 · `syn` and `quote` for structs with named fields

**Predictions I got wrong, and why:**

- 

**Explain:** Follow `struct Job { id: u32 }` from tokens to a `DeriveInput` to the output tokens. Which parts of the output are the user's tokens, which are new, and why does that matter for error messages?

> 

**Edge case I added:**

- 

## l03 · Generics, where clauses and helper attributes

**Predictions I got wrong, and why:**

- 

**Explain:** What are the three pieces that `split_for_impl` returns, and where does each go? Compare "one bound per type parameter" with "one bound per field type": give one struct for which each choice is wrong or too strict.

> 

**Edge case I added:**

- 

## l04 · Errors that point at the right place, and a compile-fail suite

**Predictions I got wrong, and why:**

- 

**Explain:** What does a `syn::Error` hold, and what does it become in the macro's output? Why is a recorded `.stderr` file a test of your spans and not only of your messages?

> 

**Edge case I added:**

- 

## l05 · An attribute macro that keeps the item intact

**Predictions I got wrong, and why:**

- 

**Explain:** Which two token streams does an attribute macro receive, and what happens to the original item? Why is "parse the function, change one field, print it again" safer than building a new function from parts? Why does the macro return the item even when it reports an error?

> 

**Edge case I added:**

- 

## l06 · A function-like macro with its own syntax

**Predictions I got wrong, and why:**

- 

**Explain:** How does a `ParseStream` differ from an iterator over tokens? What do `parse`, `peek` and `parse_terminated` do with the cursor? Which errors did syn write for you, and which did you have to write?

> 

**Edge case I added:**

- 

## l07 · Hygiene, crate paths, and how to test a macro

**Predictions I got wrong, and why:**

- 

**Explain:** Which names in generated code are resolved where the macro is *used*, and what are the two ways to protect them (one for local variables, one for paths)? Why can a unit test on tokens not prove that a macro is hygienic, and which kind of test can?

> 

**Edge case I added:**

- 

## l08 · Capstone: `#[derive(Builder)]`

**My plan, written before any code** (the data I need about each field, the checks that can fail, the generated code for the example struct):

- 

**Explain:** Which decisions does your macro make from *syntax* alone, and where can that go wrong for a user (think of a type alias for `Option`)? Which part of the generated code runs user-written expressions, and how did you keep your own local variables away from them?

> 

**The two cases I added to `tests/ui/l08/`, and what each one protects:**

- 

## c01 · A teammate's derive

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Describe the life of a derive macro: which crate it is in and why, when it runs, what it receives, what it returns, and what the compiler does with the result. Say what a macro cannot know about the types it sees.**

> 

**For `struct View<'a, T: Clone = u8> where T: Send { .. }`, write the header of the impl that a derive must generate. Compare "one bound per type parameter" with "one bound per field type", and give a struct for which each one is wrong.**

> 

**A user misspells an argument of your helper attribute. Say exactly what your macro returns, how the compiler knows where to underline, why a panic is the wrong reaction, and which two kinds of test check the message and its position.**

> 

**Explain why an attribute macro returns the item even when it reports an error. Give two ways to run code at the end of a function, and say for each what it misses or changes.**

> 

**Name the three kinds of names in generated code, and the protection for each. Explain why a name in a pattern needs more than hygiene, and why a unit test on tokens cannot show that a macro is hygienic.**

> 

**Redo the capstone after a break without looking at your first solution. Then invent a changed requirement and explain what it changes in the generated code and in your tests.**

> 
