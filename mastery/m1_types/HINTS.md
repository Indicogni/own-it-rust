# Mastery M1 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each task's doc comment and in each contract are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own. Model answers to the mastery check are at the end.

## 01 · Subtyping and variance

<details><summary>Hint 1</summary>

For A, B and C, first write the parameter itself as the whole body and let the compiler answer. Where it accepts, you are done: the conversion is a coercion and needs no code. Where it refuses, read the two `note:` lines; they name the invariant type. For D, write the body first (`push`, then `len`); it compiles with the given signature. Then look at the case `record_twice.rs` and ask how long `&'a mut Vec<&'a str>` keeps the log borrowed.

</details>
<details><summary>Hint 2</summary>

B: `Slot<'long>` cannot become `Slot<'short>`, but the `&'long str` *inside* it can become a `&'short str`. Take the value out (`slot.current.into_inner()` or `.get()`) and build a new `Slot` with `Cell::new`. D: the lifetime of the `&mut` borrow and the lifetime of the entries must be two different lifetimes. Remove `'a` from the outer reference: `log: &mut Vec<&'a str>`. `entry` keeps `'a`, because it is stored in the vector.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`&'a T` is covariant in `'a` and in `T`. `&'a mut T` is covariant in `'a` and invariant in `T`. `Cell<T>` is invariant in `T`. `fn(T)` is contravariant in `T`. Suppose `&mut T` were covariant in `T`. Then a `&mut &'static str` could be passed to a function that takes `&mut &'a str` and a `&'a str` for a short `'a`, and that function could store the short-lived reference in the slot. The owner of the slot still believes it holds a `&'static str`, and reads freed memory after the short-lived value is dropped. Reading alone is harmless under covariance; it is the ability to write through the reference that forces invariance. A function argument is write-only from the caller's side, which is why its direction turns around.

</details>

## 02 · Lifetime bounds and higher-ranked trait bounds

<details><summary>Hint 1</summary>

B: write the loop first (`total = &total + item;`) and read the error. The bound must be on the *reference* type `&T`, and `&total` borrows a local variable, so no lifetime that the caller could name is short enough. Which kind of bound says "for every lifetime"? C: implement the body with `trim_start`, `find(char::is_whitespace)` and `split_at`; then read the case `words_outlive_the_borrow.rs` and ask where the words really point. D: write `Box::new(value)` and follow the compiler's suggestion, then ask whether `'a` or `'static` is the right lifetime. E: the returned closure must own `prefix` (a `&str` is `Copy`, so `move` copies the reference).

</details>
<details><summary>Hint 2</summary>

B: `where T: Default, for<'a> &'a T: Add<&'a T, Output = T>`, then `let mut total = T::default();` and `total = &total + item;` in a loop over `items`. C: the result type is `Option<&'src str>`; `self.rest` is a `&'src str`, so slices of it have that lifetime and do not depend on `&mut self`. D: `T: Display + 'a`. E: `move |line| line.strip_prefix(prefix)`. The return type `impl for<'a> Fn(&'a str) -> Option<&'a str>` gives the closure its signature, so no annotation is needed on `line`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Without the sugar, the bound is `F: for<'a> Fn(&'a str) -> &'a str`: for every lifetime `'a`, `F` can be called with a `&'a str` and returns a `&'a str`. A lifetime in the generic parameter list of `pick_each` is chosen by the caller, once per call, and it outlives the whole function body. The lifetimes that `pick` needs are those of the borrows of each line, and the caller cannot name them: they exist only inside `pick_each`. With `for<'a>`, the code that calls `pick` chooses the lifetime at each call. `T: 'a` asks that every reference contained in a `T` is valid for at least `'a`. It says nothing about how long a value of type `T` lives; an owned type such as `String` satisfies `T: 'a` for every `'a`, also for `'static`.

</details>

## 03 · Generic associated types and a lending iterator

<details><summary>Hint 1</summary>

A: `slice::get_mut` with a range returns `Option<&mut [T]>`, and `?` gives you the "no window left" case. Compute the end of the range without overflow. B is a `while` loop around `self.next()`. C: the impl needs a `type Item<'a> = ... where Self: 'a;` line. Which type of the inner lender has the same shape? D: keep a `String` in the struct; `clear` keeps its capacity.

</details>
<details><summary>Hint 2</summary>

A: `let end = self.start.checked_add(self.size)?; let window = self.slice.get_mut(self.start..end)?; self.start += 1; Some(window)`. The reborrow through `&mut self` has exactly the lifetime of `Self::Item<'_>`. C: `type Item<'a> = I::Item<'a> where Self: 'a;` and in `next`: `self.left = self.left.checked_sub(1)?; self.inner.next()`. D: in `new`, `String::with_capacity(text.len())` and `text.lines()`; in `next`, take the next line with `?`, then `clear`, `push_str`, `make_ascii_uppercase`, and return `Some(&self.buffer)`. The item type is `&'a str`, not `&'t str`: it borrows the buffer, not the text.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Iterator::Item` is one type for the whole iterator, so it cannot mention the lifetime of a single `next` call. `WindowsMut` could only promise `&'s mut [T]`, with the lifetime of the whole slice. Two such windows could be alive together, and they overlap, which would give two `&mut` to the same elements. Safe code cannot produce that type from `&mut self`, and the compiler refuses the attempt. With `type Item<'a> = &'a mut [T]`, `next` returns a window that keeps the `&mut self` of its own call alive. In `two_windows_at_once.rs`, the line `let second = windows.next().unwrap();` is rejected with E0499: it needs a second mutable borrow of `windows` while `first` still holds the first one, and `first` is used afterwards.

</details>

## 04 · PhantomData

<details><summary>Hint 1</summary>

A: ask what an `Id<T>` has in common with a `T`. It stores none, drops none, and should not care whether `T` is `Clone` or `Send`. Which row of the marker table in CONCEPTS says that? Then look at what `#[derive]` did to the bounds. B: the invariant in the doc comment is your tool: each SAFETY comment should start from it. Decide which standard type `RawIter<'a, T>` behaves like, and copy its marker and its `Send`/`Sync` bounds. C: you need a field type that is neither `Send` nor `Sync` and has size zero.

</details>
<details><summary>Hint 2</summary>

A: `_marker: PhantomData<fn() -> T>`, and hand-written `impl<T> Clone`, `Copy`, `PartialEq`, `Eq` and `Debug` with no bound on `T` (`clone` returns `*self`; `eq` compares `raw`). B: `_marker: PhantomData<&'a T>`; `new` stores `slice.as_ptr()` and `slice.len()`; `next` returns `None` when `left == 0`, else reads `unsafe { &*self.ptr }`, advances with `unsafe { self.ptr.add(1) }` and decrements `left`; `as_slice` is `unsafe { std::slice::from_raw_parts(self.ptr, self.left) }`. A `RawIter` is a shared view, like `&[T]`, so both impls are `unsafe impl<T: Sync> ... for RawIter<'_, T>`. C: `PhantomData<*const ()>`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Id<T>` uses `PhantomData<fn() -> T>`. `PhantomData<T>` would say "I own a `T`": `Id<Rc<u8>>` would not be `Send`, although it holds a `u32`. `PhantomData<fn(T)>` would be contravariant, and `PhantomData<*const T>` would never be `Send` or `Sync`. `RawIter<'a, T>` uses `PhantomData<&'a T>`, because it behaves like a shared borrow of the slice. `&'a mut T` would make it invariant in `T`, and `fn(&'a T)` would be contravariant in `'a`: the lifetime could grow, and the iterator could hand out references that outlive the slice. The drop checker rejects a program when a value is dropped while data that its destructor might read is no longer valid. A type with a `Drop` impl is treated as using every lifetime and type parameter it has, so all of them must still be valid, and a marker cannot add anything to that. A type for which dropping runs no code at all (only references, raw pointers, numbers, markers) is never checked. The marker counts in the case between the two: a type without a `Drop` impl of its own, with a field that has code to run (a `String`, a `Box`). The compiler then goes through the fields, and it treats a `PhantomData<X>` field like a field of type `X`. If dropping an `X` could read borrowed data, the borrow must outlive the value. `PhantomData<&'a T>` and `PhantomData<fn() -> T>` own nothing and ask for nothing. The standard collections reach the same field walk through the unstable `#[may_dangle]` attribute, which is why `Vec<T>` carries a `PhantomData<T>` inside.

</details>

## 05 · Trait objects

<details><summary>Hint 1</summary>

A and B: `Shape` has `Any` as a supertrait, so a `&dyn Shape` can be *assigned* to a `&dyn Any` variable, and a `Box<dyn Shape>` to a `Box<dyn Any>`. `dyn Any` has `is`, `downcast_ref` and (on a box) `downcast`. In B, check before you convert: after a failed `downcast` you hold a `Box<dyn Any>` and cannot go back. C: `Box<dyn Label>` in a struct field has a lifetime that you did not write; CONCEPTS has the table. D: the impl is for `Box<L>`; think about which `L` must be allowed. E: go through the three reasons the compiler gives when you write `dyn Codec`, and decide for each method whether it must be callable on a trait object.

</details>
<details><summary>Hint 2</summary>

A: `let any: &dyn Any = shape; any.downcast_ref()`. B: `let any: &dyn Any = &*shape;` (not `&shape`, and not `shape.type_id()`: both ask the box), `if !any.is::<Rect>() { return Err(shape); }`, then `let any: Box<dyn Any> = shape;` and `Ok(any.downcast().expect(..))`. C: `struct Legend<'a> { entries: Vec<Box<dyn Label + 'a>> }`, `impl<'a> Legend<'a>`, and `add(&mut self, entry: impl Label + 'a)`. D: `impl<L: Label + ?Sized> Label for Box<L> { fn label(&self) -> String { (**self).label() } }`. E: `encode_into(&self, text: &str, out: &mut dyn Write)`; add `where Self: Sized` to `twin` and to `extension`; change the two impls to match.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `Box<dyn Shape>` is two words: a pointer to the value on the heap, and a pointer to the vtable for that value's concrete type and the trait `Shape`. The vtable holds the destructor, the size and the alignment of the concrete type, and one function pointer for each method. For the upcast to `Box<dyn Any>`, the data pointer stays the same and the vtable pointer is replaced by the vtable of the same concrete type for `Any`; nothing is allocated or copied. In `Box<dyn Trait>` the hidden lifetime defaults to `'static` wherever the type is written in an item (a field, a signature), so the boxed value may hold no short-lived borrow; in `&'a dyn Trait` it defaults to `'a`. In the type of a `let` inside a function body the lifetime is inferred instead. The first `Codec` was not dyn compatible because `encode_into` had a type parameter (one vtable slot cannot hold a function for every writer type), because `twin` returned `Self` by value (the caller of a trait object does not know its size), and because `extension` had no `self` receiver (there is no value whose vtable could be used).

</details>

## 06 · Zero-sized and uninhabited types

<details><summary>Hint 1</summary>

A: a pattern is allowed in a plain `let` when it covers every value that can exist. Which values of `Result<T, Infallible>` can exist? B: choose the error type that has no values; then `parse` can only return `Ok`. For `Even`, `str::parse::<u32>` after `trim` does the first check. C: the token must have no data and must not be constructible outside the module. Look at the two failure modes in "Mistakes people really make" in CONCEPTS.

</details>
<details><summary>Hint 2</summary>

A: `let Ok(value) = result; value`. A `match` with the single arm `Ok(value) => value` works too. B: `type Error = Infallible;` and `Ok(Shout(text.to_ascii_uppercase()))`; for `Even`, `text.trim().parse().map_err(|_| EvenError::NotANumber)?` and then a test of the remainder. C: `pub struct AdminToken { _private: () }` (or `pub struct AdminToken(());`), no derives; `log_in` returns `Some(AdminToken { _private: () })` only for the right password; `wipe(_proof: &AdminToken, data: &mut Vec<u8>)` calls `data.clear()`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An `AdminToken` can be created only by `log_in`, because its field is private, and `log_in` creates one only after the password matched. So a function that receives a `&AdminToken` knows that the check ran, without running it again; the type system carried the fact from one place to the other. A `Result<T, Infallible>` proves the opposite kind of fact: the `Err` variant would need a value of a type that has none, so the result is always `Ok`, and the compiler accepts a pattern that names only `Ok`. Neither proof costs anything at run time. The token is zero-sized: passing it copies no bytes. The result with an uninhabited error has the same layout as `T` alone, with no tag and no branch.

</details>

## 07 · Sealed traits, typestate and const generics

<details><summary>Hint 1</summary>

Start with the types and no methods: three unit structs, the two traits, and `struct Envelope<S>` with a `PhantomData<S>` field. Then give each method its own home: which `impl` block decides in which stage a method exists? `new` is the method people misplace. For `cc`, the bound of the `impl` block does the work. For the matrix, write the type first; `[[i64; C]; R]` is "R rows of C values". `mul` needs one extra const parameter of its own.

</details>
<details><summary>Hint 2</summary>

Sealing: `mod sealed { pub trait Sealed {} }`, `pub trait Stage: sealed::Sealed { const NAME: &'static str; }`, `pub trait HasRecipient: Stage {}`, and impls for the stage types. Blocks: `impl Envelope<NeedsTo> { new, to }`, `impl<S: HasRecipient> Envelope<S> { cc }`, `impl Envelope<NeedsBody> { body }`, `impl Envelope<Ready> { text, send }`, `impl<S: Stage> Envelope<S> { stage }`. Every transition takes `self` and builds the next type from the old fields; a private helper `fn advance<Next: Stage>(self) -> Envelope<Next>` avoids writing that three times. Matrix: `impl<const R: usize, const C: usize> Matrix<R, C>` holds `from_rows`, `rows`, `transpose` and `fn mul<const K: usize>(&self, rhs: &Matrix<C, K>) -> Matrix<R, K>`; `impl<const N: usize> Matrix<N, N>` holds `identity`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`send_needs_a_body` and `new_starts_at_needs_to` fail because `send` and `new` are defined in `impl Envelope<Ready>` and `impl Envelope<NeedsTo>` only; the other types have no such method (E0599). `cc_needs_a_recipient` fails because the `impl` block of `cc` requires `S: HasRecipient`, and `NeedsTo` does not implement it. `stage_is_consumed` fails because `to` takes `self` by value, so the second use is a use after move (E0382). `stages_are_sealed` fails because `Stage` has the supertrait `sealed::Sealed`, which another crate cannot name and so cannot implement (E0277). `matrix_mul_shape_mismatch` fails because `mul` expects `&Matrix<C, K>` with the same `C` as the left side, and `identity_is_square_only` fails because `identity` exists only in `impl Matrix<N, N>`. If `Stage` were not sealed, a caller could define a stage of their own and name the type `Envelope<Theirs>`. No constructor returns it, so the damage is small today. The larger cost is yours: you could never add a required item to `Stage`, or rely on "these three are all the stages", without breaking that caller.

</details>

## 08 · Coherence and the orphan rule

<details><summary>Hint 1</summary>

Write the impls for `i64`, `bool`, `str` and `String` first, then the containers, each with a `T: Describe` bound. For `&T` and `Box<T>`, ask which `T` the case `describe_through_pointers.rs` needs (`str`, `[i64]`, `dyn Describe`). If you are tempted to cover `i64`, `bool` and friends with one `impl<T: Display> Describe for T`, try it, and read each error. For contract B, write each impl in the general shape `impl Trait<T1> for T0` and find the local type in it.

</details>
<details><summary>Hint 2</summary>

`impl<T: Describe> Describe for [T]` builds the text; `Vec<T>` and `[T; N]` forward to it with `self.as_slice().describe()`. `String` forwards to `str`. Pointers: `impl<T: Describe + ?Sized> Describe for &T { fn describe(&self) -> String { (**self).describe() } }`, and the same for `Box<T>`. `pub struct Shown<T>(pub T);` with `impl<T: Display> Describe for Shown<T>`. Contract B: `impl From<Level> for u8` (`level as u8`), `impl TryFrom<u8> for Level` with `type Error = UnknownLevel`, `impl PartialEq<u8> for Level` and `impl PartialEq<Level> for u8`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The orphan rule: an impl of a foreign trait is allowed only if at least one of its types (the `Self` type, then the trait's type arguments, in that order) is local, and no type parameter appears uncovered, that is, on its own and not inside another type, before the first local one. A blanket `impl<T: Display> Describe for T` would claim every type that implements `Display` now or later. In this crate it would collide with the impls for `Vec<T>`, `Option<T>`, `&T` and the others, because the standard library may give those types a `Display` impl in the future. In a user's crate it would forbid a hand-written `Describe` for any type that also implements `Display`. `Shown<T>` is a local type, so its impl overlaps with nothing, and the user chooses the `Display` text explicitly, one value at a time.

</details>

## 09 · Capstone: units of measure

<details><summary>Hint 1</summary>

Three kinds of types: the value type `Quantity<U>` (one `f64` and a marker), the unit types (no values), and the trait `Unit` that maps a unit to its dimension, its factor and its symbol. Answer the three questions of lesson 04 for `U` before you choose the marker. "Same dimension" is an equality between associated types; CONCEPTS, lesson 09, shows the shape of such a bound. `Per<A, B>` is a generic type that implements `Unit` when `A` and `B` do.

</details>
<details><summary>Hint 2</summary>

`pub trait Unit: sealed::Sealed { type Dim; const IN_BASE: f64; fn symbol() -> String; }`. Units are empty enums (`pub enum Metres {}`); a `macro_rules!` saves repetition. `pub struct Per<A, B> { _never: Infallible, _marker: PhantomData<fn() -> (A, B)> }` with `type Dim = Per<A::Dim, B::Dim>` and `IN_BASE = A::IN_BASE / B::IN_BASE`. `pub struct Quantity<U: Unit> { value: f64, _unit: PhantomData<fn() -> U> }`, with `Clone`, `Copy`, `PartialEq`, `PartialOrd`, `Debug` and `Display` written by hand. `fn to<V: Unit<Dim = U::Dim>>(self) -> Quantity<V>` multiplies by `U::IN_BASE / V::IN_BASE`. Operators: `Add` and `Sub` for `Quantity<U>` with itself only; `Mul<f64> for Quantity<U>`; `Mul<Quantity<U>> for f64`; `Div<Quantity<B>> for Quantity<A>` with `Output = Quantity<Per<A, B>>`; `Mul<Quantity<B>> for Quantity<Per<A, B>>` with `Output = Quantity<A>`; and two `Sum` impls.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`add_needs_same_unit`: the only `Add` impl is `Add<Quantity<U>> for Quantity<U>`, so the right side must have the same type; `Quantity<Seconds>` is a type mismatch (E0308). `add_does_not_convert`: for the same reason `Quantity<Kilometres>` on the right is a mismatch; the caller must write `.to::<Metres>()` first. `add_rejects_bare_number`: an `f64` on the right is a mismatch too; there is no `Add<f64>` impl. `convert_needs_same_dimension`: `to` requires `V: Unit<Dim = U::Dim>`; for `Seconds` the associated type is `Time`, not `Length`, so the equality fails (E0271). `units_are_sealed`: `Unit` has the private supertrait `sealed::Sealed` (E0277). At run time a `Quantity<Metres>` is exactly an `f64`: 8 bytes, `Copy`, passed in a register. The marker is zero-sized, the unit types are never instantiated, the factors are constants that the optimizer folds, and every check happened in the type checker.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

Run the tests and the three cases, and sort what you find into three groups. Wrong results: in which order do the stages run, and which item wins a tie? Too much demanded: for each `'static`, `Send` and `Sync`, ask whether the body uses that guarantee. Too much shown: read the return type of `names` aloud. Then let clippy point at the rest.

</details>
<details><summary>Hint 2</summary>

`stage` uses `insert(0, ..)` where `push` is meant. `longest` uses `>=` where `>` keeps the first of equals. To accept borrowing closures, give the struct a lifetime: `Pipeline<'f>` with `Vec<(String, Box<dyn Fn(&str) -> String + 'f>)>`, and bound `F: Fn(&str) -> String + 'f`; drop `Send`, `Sync` and the `'static` on `name`. `run` takes `&str`, and each stage can borrow the previous output: `text = stage(&text)` needs no clone. `names` returns `impl Iterator<Item = &str>`. `longest<T: AsRef<str>>(items: &[T]) -> Option<&str>`. Add `impl Default`.

</details>
<details><summary>Check your review (after writing your own)</summary>

Two bugs: stages were inserted at the front and so ran in reverse order, and `longest` kept the last of several longest items, not the first. Bounds that asked too much: `F: 'static` came from the hidden `'static` in `Box<dyn Fn(&str) -> String>`; with `Pipeline<'f>` a caller can now pass a closure that borrows local data. `Send + Sync` protected nothing, because the pipeline never crosses a thread and a `Box<dyn Fn>` without `+ Send` is not `Send` anyway; a caller can now capture a `Cell` or an `Rc`. `name: &'static str` was copied into a `String` at once, so any `&str` will do. `longest` demanded `'static` data and returned `&'static str`; now it works on local vectors and the result borrows from the slice. `run(&String)` forced callers to own a `String`, and it cloned the text once at the start and once per stage. `names` returned the full `Map<slice::Iter<..>, fn(..)>` type: every caller that wrote that type down, and every change to the fields of `Pipeline`, would break together. With `impl Iterator<Item = &str>` you may change the vector into a map, store the names separately, or use a closure that captures, and no caller notices.

</details>

## Drills

Run `cargo drill m1`. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill. Each title names the lesson that the drill belongs to; do a drill when you have finished that lesson.

<details><summary>d01 · A longer-lived element type behind <code>&mut</code> (lesson 01)</summary>

`E0597`. `&mut Vec<T>` is invariant in `T`, so `&mut notes` can only be a `&mut Vec<&'static str>`, and `add_note` then needs a `&'static str`; `typed` does not live that long. Remove the `'static` annotation: the vector's element lifetime is then inferred to be short enough for both entries.

</details>
<details><summary>d02 · One lifetime for the value and for the borrow (lesson 01)</summary>

`E0499`. `&'a mut self` in an `impl<'a> Counter<'a>` is `&'a mut Counter<'a>`. Invariance stops `'a` from shrinking, so the first call borrows the counter for as long as it exists, and the second call is a second mutable borrow. Write `&mut self`.

</details>
<details><summary>d03 · A function that accepts more than is asked (lesson 01)</summary>

Compiles and runs. `count` accepts every `&str`: its type is `for<'a> fn(&'a str) -> usize`. A place that needs a function for `&'static str` only asks for less, and "every lifetime" includes `'static`. The same direction holds for one named lifetime: function pointers are contravariant in their argument types, so an `fn(&'a str)` may be used as an `fn(&'static str)`.

</details>
<details><summary>d04 · A function that accepts less than is asked (lesson 01)</summary>

`E0308`, with the label "one type is more general than the other". `apply` needs `for<'a> fn(&'a str) -> &'a str`; `keep` works for `'static` only. Give `keep` the general signature `fn keep(text: &str) -> &str`.

</details>
<details><summary>d05 · A closure that returns part of its argument (lesson 02)</summary>

A compile error without a code: "lifetime may not live long enough". A closure in a `let` gets no signature from outside, and the compiler gives its argument and its result two unrelated lifetimes. Use a function item, or pass the closure through a function with the bound `F: for<'a> Fn(&'a str) -> &'a str`.

</details>
<details><summary>d06 · A callback bound to one named lifetime (lesson 02)</summary>

`E0597`. `F: Fn(&'a str) -> &'a str` binds the callback to the caller's lifetime `'a`, which outlives the function body, so `clean(&upper)` cannot borrow the local `upper`. Write `F: Fn(&str) -> &str`, which means "for every lifetime".

</details>
<details><summary>d07 · Two items from a lending iterator (lesson 03)</summary>

`E0499`. Each item borrows the buffer inside `lines` and keeps the `&mut lines` of its `next` call alive. To hold two lines at once, copy each one: `.to_owned()`.

</details>
<details><summary>d08 · A generic associated type without its bound (lesson 03)</summary>

A compile error without a code: "missing required bound on `Item`". `next` returns `Self::Item<'_>` for a borrow of `Self`, so `Self: 'a` must hold; the compiler requires the `where Self: 'a` clause on the associated type.

</details>
<details><summary>d09 · A destructor that may read a dead borrow (lesson 04)</summary>

`E0597`, with "borrow might be used here, when `report` is dropped and runs the `Drop` code for type `Report`". Locals are dropped in reverse order of declaration, so `title` dies before `report`, and `Drop for Report` reads the title. Declare `title` first.

</details>
<details><summary>d10 · A type parameter that no field uses (lesson 04)</summary>

`E0392`. The compiler reads variance, auto traits and drop behavior from the fields, and no field mentions `T`. Add `_marker: PhantomData<fn() -> T>`; the size stays 4 bytes.

</details>
<details><summary>d11 · A trait that cannot be a trait object (lesson 05)</summary>

`E0038`. `scaled` returns `Self` by value, which a caller of a `dyn Shape` cannot receive. Add `where Self: Sized` to `scaled`: it leaves the vtable and stays available on concrete types.

</details>
<details><summary>d12 · The lifetime nobody wrote (lesson 05)</summary>

`E0597`, with the note "due to object lifetime defaults, `Box<dyn Greeter>` actually means `Box<(dyn Greeter + 'static)>`". Give `Desk` a lifetime parameter and write `Box<dyn Greeter + 'a>`.

</details>
<details><summary>d13 · Which value is behind <code>&dyn Any</code>? (lesson 05)</summary>

Compiles, then panics at the assertion. `&boxed` is a `&Box<dyn Any>`, and `Box<dyn Any>` implements `Any` itself, so the coercion makes a trait object whose concrete type is the box. `downcast_ref::<u32>()` then returns `None`. Write `&*boxed`.

</details>
<details><summary>d14 · A pattern that leaves a variant out (lesson 06)</summary>

Compiles and runs. `Infallible` has no values, so `Err(_)` cannot exist and `Ok(name)` covers everything. With an error type that has values, the same `let` is `E0005`.

</details>
<details><summary>d15 · A method that this state does not have (lesson 07)</summary>

`E0599`, with the note "the method was found for `Valve<Open>`". After `close()` the value is a `Valve<Closed>`. Keep the valve open: remove the `.close()`.

</details>
<details><summary>d16 · A foreign trait for a foreign type (lesson 08)</summary>

`E0117`. `Display` and `Vec` are both foreign. The local type `Celsius` inside `Vec<..>` does not count; only `&`, `&mut`, `Box` and `Pin` pass locality through. Wrap the vector in a local newtype and implement `Display` for that.

</details>
<details><summary>d17 · A blanket impl and one more (lesson 08)</summary>

`E0119`. `bool` implements `Display`, so the blanket impl already covers it. Stable Rust has no "more specific impl wins". Replace the blanket impl with the impls that are needed (`i32`, `bool`).

</details>
<details><summary>d18 · A bare type parameter as the <code>Self</code> type (lesson 08)</summary>

`E0210`. In `impl<T> From<Meters> for T`, the uncovered parameter `T` stands before the local type `Meters`. `T` could be a type of any crate, and that crate may write its own `From<Meters>`. Name the target: `impl From<Meters> for f64`.

</details>

<details><summary>d19 · A marker that says "I own one" (lesson 04)</summary>

`E0597`, with "borrow might be used here, when `ticket` is dropped and runs the destructor for type `Ticket<'_>`". `Ticket` has no `Drop` impl, but its `String` has code to run, so the drop checker goes through the fields, and it treats `PhantomData<Guard<'a>>` like a `Guard<'a>` field. `Guard` has a `Drop` impl that may read through `'a`, and `event` is already gone. A ticket owns no guard: use `PhantomData<&'a str>`. (Without the `seat` field the program would compile: a struct of markers alone has nothing to run.)

</details>
<details><summary>d20 · The lifetime nobody wrote, inside a function (lesson 05)</summary>

Compiles and runs. The default `'static` for `Box<dyn Greeter>` applies where a type is written in an item: a field (drill d12), a signature, a type alias. In the type of a `let` inside a function body the missing lifetime is inferred, so the box may hold a borrow of `name`.

</details>

## Mastery check: model answers

<details><summary>Variance of the common types</summary>

`&'a T`: covariant in `'a` and `T`. `&'a mut T`: covariant in `'a`, invariant in `T`. `Cell<T>`: invariant. `Vec<T>`: covariant. `fn(T) -> U`: contravariant in `T`, covariant in `U`. If `&mut T` were covariant in `T`, a `&mut &'static str` could be passed as a `&mut &'a str`; the callee could store a short-lived `&'a str` through it, and the owner, who still believes in `'static`, would read freed memory later.

</details>
<details><summary><code>T: 'static</code> and the desugared <code>Fn</code> bound</summary>

`T: 'static` promises that a `T` contains no reference that can expire. It does not promise that a value lives long; an owned `String` qualifies and may be dropped at once. `F: Fn(&str) -> &str` is `F: for<'a> Fn(&'a str) -> &'a str`. With `for<'a>`, the code that calls `F` chooses the lifetime at each call. With a lifetime parameter on the enclosing function, the caller of that function chooses it once, and it outlives the function body.

</details>
<details><summary>Lending iterators</summary>

`Iterator::Item` is one type and cannot name the lifetime of one `next` call, so items cannot borrow from the iterator, and nothing stops a caller from holding two of them; overlapping `&mut` windows would alias. With `type Item<'a> where Self: 'a` and `fn next(&mut self) -> Option<Self::Item<'_>>`, an item keeps the iterator mutably borrowed, and the borrow checker allows one item at a time. You lose `for` loops, `collect` and every adapter that needs two items; closure-taking methods also hit the `'static` limitation of higher-ranked bounds over GATs.

</details>
<details><summary>Raw pointers and markers</summary>

The compiler cannot work out the variance that is correct for the type, whether it may be `Send` and `Sync`, and what it owns or borrows (for lifetimes and for drop check). Owns a `T`: `PhantomData<T>` (with a `Drop` impl). Borrows a `T` for `'a`: `PhantomData<&'a T>`, or `&'a mut T` for an exclusive borrow. Only labelled with `T`: `PhantomData<fn() -> T>`.

</details>
<details><summary><code>Box&lt;dyn Trait&gt;</code> and dyn compatibility</summary>

Two words: a data pointer and a vtable pointer. The vtable holds the destructor, size and alignment of the concrete type and one function pointer per method. Not dyn compatible: a method with a type parameter (take a `&dyn` or `&mut dyn` argument, or add `where Self: Sized`); a method that returns or takes `Self` by value (add `where Self: Sized`, or return `Box<dyn Trait>`); a function without a `self` receiver or an associated constant (add `where Self: Sized` to the function, or move the item to another trait); a `Sized` supertrait such as `Clone` (remove it).

</details>
<details><summary>Object lifetime defaults</summary>

In a field, a signature or a type alias, `Box<dyn Trait>` is `Box<dyn Trait + 'static>`. `&'a dyn Trait` is `&'a (dyn Trait + 'a)`. Inside a function body, a missing lifetime in the type of a `let` is inferred, not `'static`. When a boxed value borrows a local, the error is E0597 with the note "due to object lifetime defaults ...", or "lifetime may not live long enough" in a function that returns the box; the fix is `Box<dyn Trait + 'a>` or `+ '_`.

</details>
<details><summary>Proof tokens and <code>Infallible</code></summary>

A zero-sized type with a private field can be created only by the functions of its module. If the only such function runs a check first, a value of the type shows that the check passed, and a function that takes the token needs no check of its own. `Result<T, Infallible>` tells the caller that `Err` cannot be built: `let Ok(value) = result;` compiles, no error handling is needed, and the result is as small as `T`.

</details>
<details><summary>Typestate and sealing</summary>

Zero-sized state types; a type parameter with a `PhantomData` field on the main type; transitions that take `self` by value and return the next type, with each method in the `impl` block of the states in which it is valid. A trait is sealed by a supertrait that is public but lives in a private module, so other crates can use the trait and cannot implement it. Typestate is the wrong tool when the state depends on run-time input, when values in different states must be stored together, or when the wrong call is a normal event that the caller should handle with a `Result`.

</details>
<details><summary>The orphan rule and blanket impls</summary>

An impl of a foreign trait needs a local type among the `Self` type and the trait's type arguments, and no uncovered type parameter before the first local type. `&T`, `&mut T`, `Box<T>` and `Pin<T>` count as their contents. A blanket `impl<T: Display> Tr for T` forbids the library and all its users to write any other impl of `Tr` that could overlap: for every type that implements `Display`, and for every foreign type that might implement it in the future. Users can no longer give their `Display` types a `Tr` impl of their own.

</details>
<details><summary>The capstone, changed</summary>

A new dimension needs one empty enum, and a new unit needs one empty enum with a `Sealed` and a `Unit` impl. No impl on `Quantity` changes: `Add`, `Sub`, `to`, `Display` and `Sum` are generic over `U: Unit`, and `Per<A, B>` works for the new unit at once (kilograms per second). That is the benefit of putting the facts (dimension, factor, symbol) in associated items of one trait: operations are written once against the trait.

</details>
