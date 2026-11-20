# Chapter 9 hints — Structs and methods

[Chapter guide](README.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise.

## 01 · Named structs and update syntax

<details><summary>Hint 1</summary>

A struct is a named collection of fields; update syntax moves fields that are not supplied.

</details>
<details><summary>Hint 2</summary>

Use shorthand where parameter names match fields. Build a replacement struct with ..old so the untouched String moves.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A struct expression names every field, so order in the expression does not matter; field shorthand `label` stands for `label: label`. Update syntax `..old` moves (or copies, for Copy fields) every field you did not list out of `old`. Moving a String field moves only its header, so the label keeps its heap buffer and no clone happens. Afterward `old` is partially moved: the fields you supplied yourself are still owned by `old` and usable, but `old` as a whole is not. Fields are dropped in declaration order, not in the order you wrote them.

</details>

## 02 · Tuple structs, unit structs and layout

<details><summary>Hint 1</summary>

Tuple structs are distinct named types even when their representations happen to match.

</details>
<details><summary>Hint 2</summary>

Read .0 to access a tuple field and construct the unit struct by its name.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Tuple structs are nominal: `UserId(u64)` and `OrderId(u64)` share a representation but are different types, so the compiler rejects mixing them up. A unit struct has one value and occupies zero bytes, so it can mark a proof step ("validated") at no cost; a `Vec` of zero-sized values never allocates. With the default `repr(Rust)` the compiler may reorder fields to reduce padding, so neither field order nor exact size is a promise. `repr(C)` fixes declaration order and C's padding rules: it is a contract for FFI and binary layouts, not an optimization.

</details>

## 03 · Methods and associated items

<details><summary>Hint 1</summary>

Choose receivers according to observation, mutation, or consumption.

</details>
<details><summary>Hint 2</summary>

Validate the upper bound before changing any field; access the bound through Self::MAX.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Associated functions without `self` are called as `Gauge::new(..)`; methods take `self`, `&self`, or `&mut self` and are called with dot syntax. Pick `&self` to observe, `&mut self` to change, and `self` to consume and finish the value; a `self` method drops its receiver when it returns unless it moves it elsewhere. The constructor and `set` both enforce `reading <= Self::MAX`, and `set` checks before writing, so no code path can store an invalid reading. An associated constant belongs to the type, not to any value. When an inherent method and a trait method have the same name and the same receiver type, the inherent method wins (P1). The rule is not "inherent always wins": for `x.m()` the compiler first looks for a method that takes `x` by value, then for one that takes `&x`, then `&mut x`, and it prefers inherent methods only within each of these steps. So a trait method taking `self` beats an inherent method taking `&self` when you call it on a value (P3).

</details>

## 04 · Smart-pointer self receivers

<details><summary>Hint 1</summary>

The receiver consumes the smart pointer value; moving out is permitted for unique Box ownership.

</details>
<details><summary>Hint 2</summary>

Return the field from a Box receiver. For Rc or Arc, read what you need first, then return `self` itself. `Rc::clone(&self)` would also pass the tests, because the extra owner is dropped again when the method ends, but it does needless work: check this part by reading your own code.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A receiver like `self: Rc<Self>` takes ownership of ONE pointer, not of the value: the method holds one strong count while it runs and drops it at the end unless it returns it. `self: Box<Self>` owns the heap value uniquely, so it may move fields out of it. Returning `self` hands the same pointer back without another `Rc::clone`; a clone followed by dropping `self` leaves the same counts, so no test can see it, but for an `Arc` it costs two atomic updates for nothing. Autoderef lets you call `&self` methods through any of these pointers, which is why the receiver type only matters when the method needs ownership of the pointer itself.

</details>

## 05 · Generic structs and impl blocks

<details><summary>Hint 1</summary>

Ownership operations do not require Clone or Copy.

</details>
<details><summary>Hint 2</summary>

Take the Option out of its field, and return the previous value on replace.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Slot<T>` puts no bounds on `T`, because owning, moving, and replacing values never needs `Clone` or `Copy`. `Option::take` and `Option::replace` (or `mem::replace`) move the old value out and leave a valid one behind. Bounds belong on the impl block or the method that needs them, such as an `impl<T: Copy>` block for a copying `peek`; putting `Copy` on the struct would reject `Slot<String>` for no reason. `_` is a pattern that binds nothing. In `let _ = expr;` a temporary made by `expr` (such as the return value of a call) is dropped at the end of that statement, which is what P2 shows. If `expr` is an existing variable, `let _ = x;` does not move or drop `x` at all; write `drop(x)` for that. `let _name = expr;` is a real binding and keeps the value to the end of the scope.

</details>

## 06 · Lifetimes and const parameters

<details><summary>Hint 1</summary>

The window borrows an array whose length is part of its type.

</details>
<details><summary>Hint 2</summary>

Use array.get for checked access; the stored reference already carries the source lifetime.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`T` is the element type, `'a` says the window borrows an array that must outlive it, and `N` is part of the type, so `Window<T, 3>` and `Window<T, 4>` are different types. The window owns nothing; it is one thin pointer, because the length is known at compile time, while a `&[T]` slice also carries its length at run time. `get` returns `&'a T`, a borrow of the array itself rather than of the window, so the element outlives the window. Returning a window over a local array fails because the array dies at the end of the function.

</details>

## 07 · Deriving common traits

<details><summary>Hint 1</summary>

Derived Clone delegates to each field; the cloned String owns independent bytes.

</details>
<details><summary>Hint 2</summary>

Clone the struct and mutate only its label. For an all-default record call Default::default rather than duplicating defaults.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Derived traits work field by field: `Clone` clones every field (a new buffer for each String, a count bump for each Rc), `PartialEq` compares every field, `Ord` compares fields lexicographically in declaration order, and `Default` uses each field's default. Each derive requires every field to implement the trait, which is why a struct holding a String or Vec cannot be `Copy`. `Debug` is for developers and is derivable; `Display` is user-facing text and must be written by hand. A derived `Default` can produce a value your domain considers invalid, such as an empty id, so derive it only when the zero value really is valid.

</details>

## 08 · Cell and RefCell

<details><summary>Hint 1</summary>

Cell allows replacement through shared access; RefCell checks exclusive borrowing dynamically.

</details>
<details><summary>Hint 2</summary>

Use get/set for a saturating counter; use try_borrow_mut and return false on conflict before pushing.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Interior mutability moves the aliasing check from compile time to run time. `Cell<T>` never hands out references; you `get`, `set`, `replace`, or `take` whole values, which is why it suits Copy counters. `RefCell<T>` tracks guards at run time: many `borrow()`s or one `borrow_mut()`. Overlapping them panics with `borrow`/`borrow_mut`, or returns `Err` with the `try_` versions. A guard lives as long as the temporary that holds it. In a `let` statement that is until the semicolon. In a `match` scrutinee it is the whole match (P3). In an `if let` scrutinee it is the `if` block only: since edition 2024, which this course uses, the temporary is dropped before the `else` block runs (P4). Older material describes the 2021 behaviour, where it lived through the `else` block too. Neither type is `Sync`, so they cannot be shared between threads; that is what `Mutex` and atomics are for.

</details>

## 10 · Weak and parent links

Do this lesson before the capstone.

<details><summary>Hint 1</summary>

Ownership points down the tree and observation points up. The parent holds an `Rc` to each child; the child holds a `Weak` to its parent. `Weak::new()` is a link to nothing, and `upgrade()` gives `None` for it.

</details>
<details><summary>Hint 2</summary>

Fields: `parent: Weak<RefCell<Folder>>` and `children: Vec<Link>`. Use `Rc::downgrade(parent)` in `attach`, and `borrow().parent.upgrade()` in `parent_of`. In `root_of`, start with `Rc::clone(node)` in a variable, and replace it with its parent for as long as `parent_of` gives `Some`. In `detach`, finish reading the child (get the parent handle in its own `let` statement) before you call `borrow_mut` on anything, and remove with `retain(|c| !Rc::ptr_eq(c, child))`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An `Rc` value is freed when its strong count reaches zero. If a parent owns its children with `Rc` and a child also owns its parent with `Rc`, each keeps the other's count above zero, so neither is ever freed: a reference cycle, which is a leak. The fix is to decide which direction owns. In a tree the parent owns the children, and a child only observes its parent through a `Weak`. A `Weak` does not add to the strong count; it adds to the weak count, which keeps the two counters in memory but not the value. Because the value may be gone at any time, a `Weak` cannot be dereferenced: `upgrade()` returns `Option<Rc<T>>`, `Some` with a new strong pointer while the value is alive and `None` afterwards. The drop-log test shows the difference: with `Weak` parent links, dropping the last handle drops every folder, and with strong parent links the log stays empty.

</details>

## 09 · Capstone: validated inventory

<details><summary>Hint 1</summary>

List what must always be true of an inventory, then pick one field for each piece of state. Separate state changes that need `&mut self` from the read statistic, which must change through `&self`.

</details>
<details><summary>Hint 2</summary>

Three fields are enough: an owned name, the stock, and a `Cell<u8>` for the read count. Trim and validate the name before you build the value; call `checked_add` before you change the stock; update the counter with `get`, `saturating_add` and `set`. `into_name` takes `self`, so it may move the name field out.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The type needs an owned name (`String`), the stock (`u16`) and the read count. `new` establishes the invariant (a trimmed, nonempty, owned name) so every other method can rely on it, and the private fields stop code outside the module from bypassing it. `restock` computes `checked_add` first and writes only on success, so a failed call changes nothing. `inspect` only needs `&self` because the read counter is a statistic, not part of the inventory's state; a `Cell<u8>` lets a shared borrow update it without `&mut`. A `RefCell<u8>` would also work, but it stores a borrow flag (the size test notices) and adds a way to panic for no benefit. `into_name` takes `self`: the inventory is finished, so its name can move out without a copy. The counter is single-threaded by design: sharing it across threads would need an `AtomicU8`.

</details>

## R01 · Spaced review of Chapters 3, 5, 7 and 8

<details><summary>Hint 1</summary>

The tests live in a sibling module, so they can only reach `Tag` through `pub` items: validate with `bytes().all(..)` and the length check, and only then call `to_owned`. For the transfer, compute both new balances with checked arithmetic before writing either.

</details>
<details><summary>Hint 2</summary>

`checked_sub` on the source gives the `Insufficient` case, `checked_add` on the target gives `WouldOverflow`; `ok_or(..)?` turns each into an early return. For `Words`, slice `&self.text[self.pos..]`, use `trim_start()` and `find(char::is_whitespace)`, and advance `pos` by byte lengths, which always land on char boundaries. Return type `&'a str` means the result borrows the text, not `self`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one validates the input before it is cleaned up, and one makes equality depend on how often a value was *looked at*. Every `.clone()` in this file is unnecessary.

</details>
<details><summary>Hint 2</summary>

Derived `PartialEq` compares every field, including the statistic, so write `impl PartialEq` by hand for the fields that define identity. `Cell<u32>` is enough for a Copy counter. A getter can return `&str`; a builder-style method can take `self` and use `..self`. `format!` or `write!` builds the summary in one go.

</details>
<details><summary>Check your review</summary>

`new` checks `is_empty()` before trimming, so an all-whitespace id gets through; it also clones the owned `items` vector and allocates the trimmed id twice. The derived `PartialEq` includes `views`, so two equal shipments stop being equal after one is summarized; compare `id`, `items`, and `grams` only. `id()` returns a fresh String where `&str` would do, and `with_grams(&self)` clones the whole shipment; taking `self` moves everything for free. `RefCell<u32>` with two separate borrows is a heavyweight `Cell::update(|n| n.saturating_add(1))`; the size test notices the borrow flag that a `RefCell` carries. In `summary`, clippy flags `len() == 0`, the `to_string()` inside `format!`, and the repeated `out = out + ..`. Fixing only those lints still leaves a chain of temporary Strings, which the allocation test rejects: match on `items.len()` and build each form with a single `format!`. The derived `Clone` copies the view count too; decide whether that is what callers expect.

</details>

## Drills

Run `cargo drill ch09`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Update syntax moves fields</summary>E0382. `..base` moved `base.path` (a String) into `custom`, so `base.path` is gone; `base.retries` was only copied. Supply `path: base.path.clone()` in the new struct, or borrow it before the move if you only need to read it.</details>
<details><summary>d02 · Private fields outside the module</summary>E0451. Fields are private to their module unless marked `pub`, and a struct expression must name every field, so outside code cannot build `Price` at all. Add a `pub fn new(cents: u32) -> Price` to `shop` and call it; that constructor is where invariants belong.</details>
<details><summary>d03 · Newtypes are distinct types</summary>E0308. `UserId` and `OrderId` are different nominal types even though both wrap a u64; that is the point of a newtype. Pass `order`.</details>
<details><summary>d04 · Associated functions are not methods</summary>E0599. `new` has no `self` parameter, so it is not a method and cannot be called with dot syntax. Call it through the type: `Gauge::new(5)`.</details>
<details><summary>d05 · Calling a &mut self method</summary>E0596. `counter.bump()` auto-borrows `&mut counter`, which needs a `mut` binding. Write `let mut counter`.</details>
<details><summary>d06 · Derives need every field</summary>E0369. Derived `PartialEq` compares field by field, and `Celsius` has no `==`. Add `#[derive(PartialEq)]` to `Celsius` (f64 supports PartialEq but not Eq, so `Eq` could not be derived).</details>
<details><summary>d07 · RefCell borrows at run time</summary>Panics with the message "RefCell already borrowed". The `first` guard is still alive when `borrow_mut` runs, and RefCell enforces one writer or many readers at run time. Read the length with a temporary guard (`let before = log.borrow().len();`) or `drop(first)` before mutating.</details>
<details><summary>d08 · Update syntax with Copy fields</summary>Compiles and runs. `..base` copies Copy fields instead of moving them, so `base` stays fully usable when every remaining field is Copy.</details>
<details><summary>d09 · Moving a field out of &self</summary>E0507. Through `&self` you may read the String but not move it out, because the caller still owns the Badge. Return `&str` (`&self.name`), take `self` by value to consume the badge, or clone if you really need a second String.</details>
<details><summary>d10 · A struct that borrows</summary>E0106. A struct field that is a reference needs a lifetime parameter: `struct Excerpt<'a> { text: &'a str }`. It tells the compiler an Excerpt cannot outlive the text it points into.</details>
<details><summary>d11 · Upgrading a Weak pointer</summary>Panics with the message "called `Option::unwrap()` on a `None` value". The only `Rc` was dropped at the end of the inner block, and a `Weak` does not keep its value alive, so `upgrade()` returns `None`. Keep a strong owner alive for as long as you need the value: here, remove the inner block so `team` lives to the end of `main`. In real code, handle the `None` instead of unwrapping.</details>

## Mastery check: model answers

Answer each question of the [mastery check](README.md#mastery-check) in your own words first. Open this only to compare.

<details><summary>Model answers (after writing your own)</summary>

1. **Named, tuple and unit structs.** Use named fields when each field has its own meaning and you want to read `parcel.zone`. Use a tuple struct for a thin wrapper around one or two values, above all a newtype such as `Meters(u32)`. Use a unit struct when the type itself is the information and there is no data. Struct types are nominal: two structs are different types because they have different names, even when their fields are identical. So `UserId(u64)` cannot be passed where `OrderId(u64)` is expected.
2. **Invariants and receivers.** Keep the fields private and make every way of creating or changing the value go through a function that checks the rule, such as `new` returning `Option<Self>`. Choose `&self` to read, `&mut self` to change in place, and `self` to consume the value and turn it into something else. A method that can fail checks first and writes afterwards, so a failed call changes nothing.
3. **Smart-pointer receivers.** `self: Box<Self>`, `self: Rc<Self>` and `self: Arc<Self>` take ownership of one pointer, not of a copy of the value. A `Box` receiver owns the value alone and may move fields out. An `Rc` or `Arc` receiver is one of possibly many owners: return `self` or store it, and do not call `clone` on the pointee. Cloning the pointer copies an address and adds one to a count; it never copies the data.
4. **Type, lifetime and const parameters.** In `Window<'a, T, const N: usize>`, `T` says what kind of element is stored, `'a` says the window borrows data that must outlive it, and `N` is a number that is part of the type, here the array length. Different arguments give different types: `Window<u8, 3>` and `Window<u8, 4>` cannot be mixed.
5. **Derived traits.** A derive writes the obvious field-by-field impl: `Clone` clones each field, `PartialEq` compares each field, `Ord` compares fields in declaration order, `Default` uses each field's default, `Debug` prints the type name and fields. Each one needs every field to implement the same trait, and `Copy` also needs `Clone`. On a generic struct the derive adds the bound to every type parameter, for example `T: Clone`.
6. **Cell and RefCell.** Both allow change through a shared reference, in one thread only. `Cell<T>` never gives out a reference to its content: you copy a value out or put a whole value in. `RefCell<T>` gives out references through guards and counts them at run time: many readers or one writer. Breaking that rule panics (`borrow`, `borrow_mut`) or returns `Err` (`try_borrow`, `try_borrow_mut`). The rule "shared or mutable, never both" still holds; only the moment of the check moved from compile time to run time. Neither type is `Sync`, so the compiler rejects sharing them between threads.
7. **Rc and Weak.** Give the ownership direction an `Rc` and the back-link a `Weak`. A strong cycle keeps every strong count above zero, so nothing in it is dropped. A `Weak` does not keep the value alive; `upgrade()` returns `None` once the last `Rc` is gone, and the caller must handle that case.
8. **The capstone again.** There is no single answer. A good answer names the invariant, shows where it is checked, says which methods need `&mut self`, and explains the cost of the changed requirement. Example: making the read count visible to several threads replaces `Cell<u8>` with `AtomicU8` and makes the type `Sync`, at the price of atomic operations on every read.

</details>
