# Mastery M1 notes — The type system in depth

Your workbook. Write answers in your own words, *before* checking the hints. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · Subtyping and variance

**Predictions I got wrong, and why:**

- 

**Explain:** State the variance of `&'a T`, `&'a mut T`, `Cell<T>` and `fn(T)` in `'a` and in `T`. For one invariant case, describe the program that would be accepted, and what it would do, if the type were covariant.

> 

**Edge case I added:**

- 

## l02 · Lifetime bounds and higher-ranked trait bounds

**Predictions I got wrong, and why:**

- 

**Explain:** Write the bound of task A without the sugar, with an explicit `for<'a>`. Explain why a caller of `pick_each` could not name that lifetime in a generic parameter list, and what `T: 'a` asks of a type.

> 

**Edge case I added:**

- 

## l03 · Generic associated types and a lending iterator

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why `WindowsMut` can implement `LendingIterator` and cannot implement `Iterator`. Name the line of the program in `tests/cases/l03/two_windows_at_once.rs` that the borrow checker rejects, and say which two borrows collide.

> 

**Edge case I added:**

- 

## l04 · PhantomData: ownership, variance, auto traits and drop check

**Predictions I got wrong, and why:**

- 

**Explain:** For `Id<T>` and `RawIter<'a, T>`, name the marker you chose and what would go wrong with each marker you rejected. Then explain when the drop checker rejects a program, and in which one situation a `PhantomData` field changes its answer on stable Rust.

> 

**Edge case I added:**

- 

## l05 · Trait objects: vtables, dyn compatibility, upcasting, lifetimes

**Predictions I got wrong, and why:**

- 

**Explain:** Describe what a `Box<dyn Shape>` consists of at run time, and what the compiler does for the upcast in task B. Then give the rule for the default lifetime of `Box<dyn Trait>` and of `&'a dyn Trait`, and the three reasons why the first `Codec` was not dyn compatible.

> 

**Edge case I added:**

- 

## l06 · Zero-sized types and uninhabited types

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what "a value of this type proves something" means for `AdminToken` and for `Result<T, Infallible>`, and why neither proof costs anything at run time.

> 

**Edge case I added:**

- 

## l07 · Sealed traits, typestate and const generics

**Predictions I got wrong, and why:**

- 

**Explain:** For each program in `tests/cases/l07/` that must not compile, say which part of your design rejects it. Then say what a caller could do if `Stage` were not sealed.

> 

**Edge case I added:**

- 

## l08 · Coherence, the orphan rule and blanket impls

**Predictions I got wrong, and why:**

- 

**Explain:** State the orphan rule in one sentence, including the part about uncovered type parameters. Then explain why this library gives its users `Shown<T>` and not a blanket impl of `Describe` for every `T: Display`.

> 

**Edge case I added:**

- 

## l09 · Capstone: units of measure, checked by the compiler

**Predictions I got wrong, and why:**

- 

**Explain:** For each of the five programs that must not compile, name the trait bound or the type mismatch that stops it. Then list what a `Quantity<Metres>` costs at run time compared with a plain `f64`.

> 

**Edge case I added:**

- 

## c01 · A generic API that asks for too much and shows too much

**My review, written before running the tests:**

- 

**Explain:** For each bound or lifetime that you removed, say what a caller can do now that was rejected before. For `names`, say what you may change inside `Pipeline` now without breaking a caller.

> 

**Edge case I added:**

- 

## Design notes for lessons 07 to 09

For each contract: the types I chose, one design I rejected, and why.

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Give the variance of `&'a T`, `&'a mut T`, `Cell<T>`, `Vec<T>` and `fn(T) -> U` in each parameter. For `&'a mut T`, describe the unsound program that covariance in `T` would allow.**

> 

**Explain what `T: 'static` does and does not promise, and write the desugared form of `F: Fn(&str) -> &str`. Say who chooses the lifetime in each form.**

> 

**Explain why an iterator that lends overlapping `&mut` windows cannot implement `Iterator`, and how a generic associated type solves it. Name one thing you lose.**

> 

**For a type that holds a raw pointer, list the three things the compiler cannot work out for itself, and the `PhantomData` marker for "owns a `T`", "borrows a `T` for `'a`" and "is only labelled with `T`".**

> 

**Draw a `Box<dyn Trait>`: the two words and what the vtable holds. Give three reasons why a trait may not be dyn compatible, and the repair for each.**

> 

**State the default lifetime of the trait object in `Box<dyn Trait>` and in `&'a dyn Trait` when the type is written in a field or a signature, what changes in a `let` inside a function body, and the error you get when the default is wrong.**

> 

**Explain how a zero-sized token with a private field proves that code ran, and what `Result<T, Infallible>` tells a caller.**

> 

**Describe the three ingredients of the typestate pattern and how a trait is sealed. Say when typestate is the wrong tool.**

> 

**State the orphan rule, including uncovered type parameters. Explain what a blanket impl over `T: Display` takes away from the users of a library.**

> 

**Redo the capstone after a break without looking at your first solution. Then add a third dimension (mass) and a unit for it, and explain which impls you had to touch and why the others did not change.**

> 
