# Chapter 11 hints — Generic code with traits

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Read the lesson's section in [CONCEPTS.md](CONCEPTS.md) and the compiler diagnostics first. Open one level at a time.

## 01 · Traits, implementations, and defaults

<details><summary>Hint 1</summary>

A default trait body can call a required method on self. Implementations use exactly the declared receiver and result types.

</details>
<details><summary>Hint 2</summary>

Build the sensor description with format!, and let brief call description rather than duplicating sensor formatting.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Sensor` supplies only the required method, `description`; `brief` comes from the trait's default body, which calls `self.description()` and so picks up whatever each implementor provides. A type that overrides `brief` replaces the default entirely, and generic code calling `value.brief()` runs the override. A trait is a contract about behavior, not about layout, so two unrelated types can implement it without sharing fields or a common ancestor; implementing it adds no data to either type (P2). Blanket impls such as `impl<T: Display> Exclaim for T` give the trait to every type that meets the bound.

</details>

## 02 · Static and dynamic dispatch

<details><summary>Hint 1</summary>

A reference to dyn Trait erases the concrete implementor while retaining a callable interface.

</details>
<details><summary>Hint 2</summary>

Iterate over the trait objects and call measure. Use Write::write_all twice with ? so an I/O failure is propagated.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`total_static<T: Measure>` is monomorphized: the compiler generates one copy per concrete `T` and can inline `measure`, but the slice must hold a single type. `&dyn Measure` is a fat pointer (data pointer plus vtable pointer, two words), so one slice can mix types, and every `measure` call goes through the vtable. Default methods have vtable entries too, so an override is found at run time. Generic type parameters are implicitly `Sized`; `W: Write + ?Sized` lifts that, so `write_record` accepts `&mut dyn Write` as well as concrete writers. `write_all` loops until every byte is accepted, while a single `write` may stop early.

</details>

## 03 · Self, supertraits, and associated items

<details><summary>Hint 1</summary>

Self denotes the implementing type. A supertrait makes another trait’s methods available under one bound. A provided method can call an associated function of the same trait as `Self::function()`.

</details>
<details><summary>Hint 2</summary>

Construct Count directly. In the trait body, `reset` cannot see any field, because it does not know the implementing type; it can only replace the whole value behind `self` with a new `Self`. In `fresh`, call `T::zero()` without an instance and read `T::LIMIT`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`reset` is a provided method. Its body uses only items that every implementor must supply (`Self::zero()`), so it works for each of them, and `Level` in the tests gets it without writing it. `Resettable: Named + Sized` lists two supertraits. `Named` means every implementor also implements `Named`, so code bounded by `T: Resettable` can call `name()`. `Sized` is there for the body of `reset`: assigning a whole `Self` through `*self`, and receiving the value that `zero()` returns, both need a size known at compile time. The declaration `fn zero() -> Self;` alone would not need it; only a body that handles `Self` by value does. An associated function such as `zero` has no `self` and is called as `T::zero()`. A method takes `self`, `&self`, or `&mut self` and can be called with dot syntax. Associated constants belong to the implementation, not to each value: `Count` is still 2 bytes (P1), and a default method sees each implementor's own constant (P2).

Lab: `Resettable` cannot be used as `dyn Resettable`, for three separate reasons: the `Sized` supertrait, the associated constant `LIMIT`, and `zero`, which has no receiver. With `+ Sized` in place the compiler reports only that one; remove it and the compiler lists the other two. `where Self: Sized` on `zero` removes only the reason about `zero`.

</details>

## 04 · Extension traits and qualified calls

<details><summary>Hint 1</summary>

An extension trait belongs to your crate, so its implementation may target an external type.

</details>
<details><summary>Hint 2</summary>

Find the smallest and largest items first, then convert both to i64 before you subtract. Fully qualify both `tone` calls (`Warm::tone(value)`, `Cool::tone(value)`); ordinary `value.tone()` is ambiguous.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Coherence requires at most one implementation of a trait for a type in the whole program. The orphan rule enforces it one crate at a time: an impl is allowed if the trait is local, or if at least one type in the impl header (the implementing type or a type parameter of the trait) is local and no bare type parameter comes before the first local type. So `impl SliceStats for [i32]` is fine (your trait), and `impl From<MyType> for u8` is fine (a local type in the trait's parameters), but `impl Display for Vec<i32>` is rejected (nothing is local); wrap the type in a newtype instead. Trait methods are callable only when the trait is in scope. If two traits in scope provide the same method name for one type, `value.tone()` is ambiguous and you name the trait: `<Palette as Warm>::tone(&value)` or `Warm::tone(&value)`.

Method lookup tries the receiver type by value first, then `&`, then `&mut`, and only then dereferences and starts again. Within one of those steps an inherent method wins over a trait method (P1). It does not win across steps: a trait method that takes `self` is found before an inherent method that takes `&self` (P4). For the same reason `vec![1].kind()` finds the `Vec<i32>` impl before it derefs to the slice (P3).

</details>

## 05 · Associated types and generic traits

<details><summary>Hint 1</summary>

The associated Item is chosen once per implementation. The generic Rhs is part of which trait is implemented.

</details>
<details><summary>Hint 2</summary>

Use Iterator::next on the stored iterator; write the two `Combine<Rhs>` impls independently of each other.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An associated type is an output: the implementation fixes it once, so each type has exactly one `Item` per trait and callers never have to name it (`Iterator`, `Deref::Target`). A generic trait parameter is an input chosen by the caller, so one type can implement `Combine<u8>` and `Combine<&str>` side by side, and the argument type selects the implementation. Use an associated type when the relationship is determined by the implementing type; use a parameter when several relationships make sense. `Vec<u8>` and `&Vec<u8>` are different implementing types, so they can have different `Item`s (P1). Implementing the one required method gives every provided method that is built on it (P2).

</details>

## 06 · Opaque results, lifetimes, and precise capture

<details><summary>Hint 1</summary>

An opaque result hides a single concrete type. Its lifetime contract still describes references yielded by that type.

</details>
<details><summary>Hint 2</summary>

Use split_whitespace for borrowed words. For the closure, `prefix` is already a reference, and `move` copies that reference into the closure; do not build a String from it. For the pair, one clone is enough: what can happen to the original afterward?

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`impl Trait` in return position hides one concrete type chosen by the function body. In edition 2024 the hidden type may capture every generic and lifetime parameter in scope, including the unnamed lifetime of `_scratch`. `use<'a>` narrows that list, so the iterator from `words` does not borrow `_scratch`, which can then be dropped first. `+ 'a` is a different statement: the hidden type outlives `'a`, so the closure from `starts_with` may hold `&'a str` and nothing shorter. `T: Clone` is the only bound `duplicated` needs, because it clones once and moves the original. A returned closure is a struct of its captures: a `&str` capture makes it two words, and neither building it nor calling it allocates.

</details>

## 07 · Capstone: a generic validation pipeline

<details><summary>Hint 1</summary>

Collecting a Source does not require knowing its concrete type, only its associated Item.

</details>
<details><summary>Hint 2</summary>

Pull owned strings with a while let loop; all on an empty rule list is true. Keep accepted records in input order.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Rule` methods take `&self` and `&str` and return `bool`, with no generics and no `Self` by value, so the trait is dyn compatible and one slice can mix `MinLength` and `Forbid`. A source's `Item` is fixed by its implementation, and the bound `S: Source<Item = String>` says which sources `run` accepts; `run` is monomorphized per source type. A generic method such as `fn accepts_any<T: AsRef<str>>(&self, t: T)` cannot have a vtable entry, because there would be one instance per `T`. It would break `&dyn Rule` unless it is marked `where Self: Sized`, which leaves it uncallable through the trait object. Strings move from source to output without being cloned.

</details>

## R01 · Spaced review of Chapters 7, 9, and 10

<details><summary>Hint 1</summary>

A recursive enum is processed by a `match` whose `Crate` arm calls `weight` on each child. `PartialOrd` gives `<` and `>`, and `Copy` lets you keep the current low and high by value. `?` converts any `E: Error` into `Box<dyn Error>`.

</details>
<details><summary>Hint 2</summary>

`500 + items.iter().map(Weight::weight).sum::<u64>()`. For `bounds`, start with `let first = *items.first()?;`. For `parse_ids`, `let n: u16 = item.trim().parse()?;`, then `if n == 0 { return Err("zero is not a valid id".into()); }`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one includes a boundary value it should exclude, and one lets a later tie replace the earliest. Every `Box`, `to_string`, and `clone` here can go, and two bounds are unnecessary.

</details>
<details><summary>Hint 2</summary>

Return `impl Iterator<Item = &str>` from `above`, taking `&[Box<dyn Metric>]`; in edition 2024 the result captures the borrow of the slice without any extra syntax. `largest<T: PartialOrd>(items: &[T]) -> Option<&T>`, replacing only on `>`. In `render`, drop the `Clone` bound, iterate by reference, and `writeln!` into one `String` (import `std::fmt::Write`).

</details>
<details><summary>Check your review</summary>

`above` uses `>=` where the contract says strictly above, and it boxes an iterator that could be returned as `impl Iterator`. It also allocates a `String` per name where `&str` borrowed from the metrics would do, and takes `&Vec` where `&[_]` is enough. Clippy does not report that parameter in this function, so you have to find it by reading. `largest` uses `>=`, so the last of equal items wins. It clones on every improvement, demands `Clone` and `Display` bounds it does not need, and indexes where it could iterate (clippy: `needless_range_loop`). `render` copies the whole slice with `to_vec` (clippy: `unnecessary_to_owned`) and so forces `M: Clone`, which shuts out every metric type that cannot be cloned. It also builds a new string for each line with `out = out + &format!(...)`.

</details>

## Drills

Run `cargo drill ch11`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Trait objects need dyn-compatible traits</summary>E0038. `scaled` returns `Self`, whose size is unknown behind `dyn Shape`, so no vtable entry can exist for it. Add `where Self: Sized` to `scaled`: concrete types can still call it, and the trait becomes dyn compatible.</details>
<details><summary>d02 · Trait methods need the trait in scope</summary>E0599. `write_all` is a method of `std::io::Write`; trait methods resolve only when the trait is in scope. Add `use std::io::Write;`.</details>
<details><summary>d03 · The orphan rule</summary>E0117. `Display` is foreign, `Vec` is foreign, and no type in the impl header is local, so another crate could write the same impl. Wrap it: `struct Ids(Vec<u32>);` and implement `Display` for `Ids`.</details>
<details><summary>d04 · Generic code only gets what its bounds promise</summary>E0369. Inside a generic function, `T` supports only what its bounds declare, and `>` comes from `PartialOrd`. Declare `T: PartialOrd`.</details>
<details><summary>d05 · impl Trait is one concrete type</summary>E0308. `Rev<RangeInclusive<u32>>` and `RangeInclusive<u32>` are different types, and `impl Iterator` stands for exactly one. Return `Box<dyn Iterator<Item = u32>>` (one allocation, dynamic dispatch), or restructure so both branches have one type.</details>
<details><summary>d06 · Extension traits on foreign types</summary>Compiles. Your crate defines `Halves`, so it may implement it for the foreign type `str`. The orphan rule restricts only impls of a trait that another crate defines; an impl of a local trait is always allowed by it.</details>
<details><summary>d07 · Associated types are part of the impl</summary>E0046: the impl is missing `type Item`. The signature of `next` does not declare it; add `type Item = u32;`.</details>
<details><summary>d08 · Sorting floats</summary>E0277: `sort` needs `Ord`, and `f64` is only `PartialOrd` because NaN is unordered. Use `readings.sort_by(f64::total_cmp)`, which defines a total order over all floats.</details>
<details><summary>d09 · impl Trait captures every lifetime in scope</summary>E0597. In edition 2024 the opaque type captures `separator`'s lifetime too, so `iter` keeps `separator` borrowed after its block ends. Add `+ use<'a>` to the return type so only `text` is captured.</details>
<details><summary>d10 · Downcasting a trait object</summary>Panics. Nothing constrains the literal, so `5` is an `i32`. `downcast::<i64>` checks the exact `TypeId` at run time and returns `Err`. Write `Box::new(5_i64)`, or downcast to `i32`.</details>
<details><summary>d11 · A foreign trait, a local type, and a type parameter</summary>E0210. A local type in the trait's parameters (`Add<Celsius>`) does allow an impl of the foreign trait `Add` for a foreign type, but not for a bare `T`: `T` stands before the first local type, and it could be a type from any other crate, which might write a conflicting impl. Name the type: `impl Add<Celsius> for f64`, using `self + rhs.0`.</details>

## Mastery check: model answers

Answer each item of the [mastery check](README.md#mastery-check) in your own words first. The last two items are things to do, so they have no model answer.

<details><summary>Model answers (after writing your own)</summary>

**A trait object's data/vtable pair and a generic function's monomorphization.** `&dyn Trait` and `Box<dyn Trait>` are fat pointers of two words. The first word points to the value. The second points to a vtable that the compiler builds once for each (concrete type, trait) pair; it holds the type's size, alignment and drop function and one function pointer per method. A call loads the pointer and jumps, so it cannot be inlined, but one compiled function serves every type and a collection can mix types. A generic function `fn f<T: Trait>` is compiled once for each concrete `T` that the program uses. Every call is direct and can be inlined; the price is more machine code and one type per collection.

**A dyn-compatible interface, and what would break it.** Example: `trait Rule { fn accepts(&self, text: &str) -> bool; }`. It stays dyn compatible while every method has a receiver, mentions `Self` nowhere else, has no generic type parameters, and does not return `impl Trait`; the trait must also have no associated constants and no `Sized` supertrait. `fn copy(&self) -> Self`, `fn check<T: AsRef<str>>(&self, t: T)`, `fn new() -> Self` and `const MAX: usize;` each break it (E0038). The three methods can be kept by adding `where Self: Sized`, which hides them from trait objects.

**Associated type or generic trait parameter.** Use an associated type when the implementing type decides the other type, so there is one answer: `Iterator::Item`, the `Item` of a `Source`. Use a parameter when one type should relate to several others and the caller chooses: `From<T>`, `Combine<Rhs>`. Bounds state only what the body uses: `S: Source<Item = String>` when the code needs owned strings, `where S::Item: Clone` when it clones items, `T: PartialOrd` when it compares.

**Self, supertraits, associated constants, extension traits, coherence.** `Self` is the implementing type. A supertrait (`trait A: B`) obliges implementors of `A` to implement `B` and lets code bounded by `A` use `B`. An associated constant is one value per implementing type, read as `T::NAME`, and takes no space in values. An extension trait is your trait implemented for someone else's type, which gives that type new methods wherever the trait is imported. Coherence means at most one impl of a trait for a type in the whole program; the orphan rule keeps it by requiring a local trait, or a local type in the impl header with no bare type parameter before it.

**Lifetime capture of return-position `impl Trait` in edition 2024.** `fn numbered(start: u32, label: &str) -> impl Iterator<Item = u32>` returns a value that the caller must treat as borrowing `label`, because the opaque type captures every lifetime in scope, named or not. Mutating or dropping the `String` behind `label` while the iterator is alive fails (E0502 or E0597), although the iterator holds no reference. Adding `+ use<>` (or `+ use<'a>` to keep one lifetime) states the real captures and the caller's code compiles. Every type parameter in scope must still be listed.

</details>
