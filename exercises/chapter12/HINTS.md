# Chapter 12 hints — Operators for your own types

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Read the lesson's section in [CONCEPTS.md](CONCEPTS.md) and the compiler's diagnostics first. Open one level at a time.

<details><summary>A test run stops with "has overflowed its stack", or says "was still running after 10 s"</summary>

Both messages mean that a method calls itself without end. In this chapter the cause is nearly always an operator method that uses its own operator: `self + rhs` inside `add`, `self == other` inside `eq`, `&mut self[(row, col)]` inside `index_mut`, or `cmp` and `partial_cmp` that each call the other. A debug build runs out of stack and the whole test run aborts. A release build can turn the same code into an endless loop, and then the ten-second limit in the tests reports it. For the simplest cases the compiler warns `function cannot return without recursing`. Fix it by working on the fields: `self.x + rhs.x`, `self.id == other.id`, `&mut self.0[...]`.

</details>

## 01 · Unary, binary, and assignment operators

<details><summary>Hint 1</summary>

Each operator maps to a method in std::ops. Compound assignment receives &mut self.

</details>
<details><summary>Hint 2</summary>

Add the matching coordinates, negate both coordinates, and update the receiver in AddAssign. Use the fields (`self.x`, `rhs.x`), not the operator you are defining. Inputs in this lesson stay in -1000..=1000.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Each operator is a trait method: `a + b` is `Add::add(a, b)`, `-a` is `Neg::neg(a)`, and `a += b` is `AddAssign::add_assign(&mut a, b)`. `Add` and `Neg` take their operands by value and return a new `Output`, which need not be `Self`: `Mul<i32> for Delta` shows that the right-hand side can be another type too. For `Copy` types like `Delta`, moving the operands is just copying them. For `String`, `s + "x"` consumes `s` and reuses its buffer (P1). Assignment operators are separate traits because they mutate the left operand in place through `&mut self`; they have no `Output`, and implementing `Add` does not give you `+=`.

</details>

## 02 · Equivalence and NaN

<details><summary>Hint 1</summary>

Choose the identity fields explicitly. PartialEq may return false even when both operands are the same float value.

</details>
<details><summary>Hint 2</summary>

Compare ids only for Record. For Reading, apply `==` to the two wrapped f64 values, so that every float rule (NaN, the two zeros) comes along unchanged.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`PartialEq` must be symmetric (`a == b` implies `b == a`) and transitive (`a == b` and `b == c` imply `a == c`). `Eq` adds reflexivity: `a == a` for every value. `Record` compares only `id`, which still satisfies all three, so it can be `Eq` even though `note` is ignored. `f64` is not reflexive, because NaN != NaN, so `Reading`, which delegates to `f64`'s `==`, must not claim `Eq`. Code that requires `Eq` (keys of a `HashMap` or `HashSet`, and keys of a `BTreeMap` through `Ord`) would be given a promise that is false. Code that only assumes reflexivity, such as `contains` or `dedup`, compiles for floats and quietly never matches a NaN. `==` on floats is not bitwise either: `0.0 == -0.0` is true, though their bits differ.

</details>

## 03 · Partial and total comparisons

<details><summary>Hint 1</summary>

Compare primary keys first, then break ties with the secondary key.

</details>
<details><summary>Hint 2</summary>

Use cmp(...).then_with(...). PartialOrd should return Some(self.cmp(other)); partial float comparison may return None.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Ord::cmp` defines the order: priority first, then name as the tie-breaker. That tie-breaker is part of the contract, because `sort` and `BTreeMap` rely on it and callers see the resulting order. `PartialOrd` powers `<`, `>`, and friends, and for a totally ordered type it must agree with `Ord`, so `partial_cmp` returns `Some(self.cmp(other))`. Writing it separately invites the two to disagree. `cmp` returning `Equal` must coincide with `==`, which is why both fields take part in the derived `PartialEq`. Floats have only a partial order: `partial_cmp` returns `None` when either side is NaN. `total_cmp` invents a total order, in which -0.0 < 0.0.

</details>

## 04 · Index and IndexMut

<details><summary>Hint 1</summary>

Both traits should refer into the same underlying data and use exactly the same row-major index.

</details>
<details><summary>Hint 2</summary>

Check row<2 and column<3 before indexing row*3+column. Use assert! with the exact documented message. Index the inner array (`self.0[...]`), not `self`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Index::index(&self, idx) -> &Output` enables reads such as `grid[(r, c)]`; the expression means `*grid.index((r, c))`, so it is a place you can borrow (P3). `IndexMut::index_mut` returns `&mut Output` and is chosen when the place is assigned or mutably borrowed. Both must apply the same bounds check and mapping, so reads and writes see the same element. Each coordinate is checked separately: `(0, 3)` maps to flat index 3, which is in range but wrong. The traits return references into `self`; a reference to a temporary computed inside `index` would dangle, so an `Index` impl cannot build a new value. That is also why `Index` panics: a checked `get` returning `Option<&T>` is the non-panicking API.

</details>

## 05 · Capstone: a small bit-set API

<details><summary>Hint 1</summary>

The newtype gives a semantic meaning to bitwise operators without changing the underlying u8 representation.

</details>
<details><summary>Hint 2</summary>

Delegate to the corresponding u8 operators on the field. Check bit<8 before shifting so debug and release agree.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Flags` derives `PartialEq`, `Eq`, and `Hash` from the same single field, so equal values always hash equally, which is the contract `HashSet` and `HashMap` depend on. Union, intersection, and complement map directly onto `|`, `&`, and `!` of the `u8`. Complement needs no length because the width is fixed at 8 bits. `|=` mutates in place through `BitOrAssign`. `contains(bit)` must check `bit < 8` before shifting. Shifting a `u8` by 8 or more panics in a debug build. In a release build the shift amount is reduced to its low three bits, so a shift by 8 becomes a shift by 0, and an unchecked `contains(8)` would test bit 0 and wrongly answer `true` for `Flags(255)`. The check keeps both profiles in agreement. Indexing would have to return `&bool` into storage that does not exist, so a method is the honest API.

</details>

## R01 · Spaced review of Chapters 5, 10, and 11

<details><summary>Hint 1</summary>

The bounds on `dot` already list everything you may do with `T`: copy it, start from `T::default()`, add, and multiply. Compare `Money` by converting both sides to cents in a type wider than `u32`. `Index` returns a reference, so `get` must return a reference into `self.rows`.

</details>
<details><summary>Hint 2</summary>

`a.iter().zip(b).fold(T::default(), |acc, (&x, &y)| acc + x * y)`. A helper `fn cents(m: Money) -> u64` with a `match` makes `eq` one line. `self.rows.iter().find(|(n, _)| n == name).map(|(_, v)| v)`. In `index`, `self.get(name).unwrap_or_else(|| panic!("no row named {name}"))`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one breaks the rule that equal values must hash equally, and one silently drops coefficients. Every `clone`, every `to_ascii_lowercase`, and the fresh `Vec` in `add` can go.

</details>
<details><summary>Hint 2</summary>

`str::eq_ignore_ascii_case` compares without allocating. In `hash`, feed `b.to_ascii_lowercase()` for each byte to `state.write_u8`, then a terminator such as `0xff`. Put the addition in `add_assign`: add the overlapping part in place, then `extend_from_slice` the rest of `rhs`. `add` can then pick the longer operand as its accumulator and `+=` the other into it. Change both methods together: if `add` uses `+=` while `add_assign` still calls `+`, each calls the other forever.

</details>
<details><summary>Check your review</summary>

`Tag` overrides equality to ignore ASCII case but hashes the original bytes, so `"Rust"` and `"rust"` are equal yet hash differently: a `HashSet<Tag>` can hold both. Equality also allocates two lowercase copies per comparison. A fix that hashes `self.0.to_ascii_lowercase()` repairs the contract but allocates a `String` on every map lookup, which is the same waste. `Poly::add` zips the coefficients, which stops at the shorter operand and loses the longer one's tail, and it allocates a new vector even though it owns both inputs. One of the two buffers is always long enough for the sum: the longer one, whether it arrives on the left or on the right. `add_assign` clones both sides just to call `add`, so `+=` copies everything instead of mutating in place. The lesson: when you implement `PartialEq` by hand, implement `Hash` from the same normalized identity.

</details>

## Optional labs

The lesson headers suggest experiments for [the scratch example](../../examples/ch12_scratch.rs). They are optional and have no tests. Open a note only after you tried the lab.

<details><summary>Lab 01 · A bit-mask newtype; operators without traits</summary>

The four impls each hand the work to the `u8` inside: `Mask(self.0 & rhs.0)`, `Mask(self.0 | rhs.0)`, `Mask(!self.0)`, and `self.0 |= rhs.0`. Useful checks: `!Mask(0) == Mask(0xff)`, `x & Mask(0) == Mask(0)`, `x | Mask(0xff) == Mask(0xff)`. For `&&`, `as` and `=` there is nothing to implement: `impl std::ops::And for Mask {}` fails with E0405, `cannot find trait And in module std::ops`. `&&` may skip its right operand, and a method call always evaluates its arguments. `=` is always a move or a copy. `as` is a fixed list of built-in conversions; your own conversions use `From` and `TryFrom`.

</details>
<details><summary>Lab 02 · Eq with a float; a case-insensitive name</summary>

`#[derive(PartialEq, Eq)]` on a struct with an `f64` field fails with E0277, `the trait bound f64: Eq is not satisfied`. For the name newtype, `eq` is `self.0.eq_ignore_ascii_case(&other.0)`. A `Hash` impl must feed the hasher the same normalized form, for example each byte after `to_ascii_lowercase`. If it hashed the original text, `"Ada"` and `"ADA"` would be equal with different hashes, and a `HashSet` could store both.

</details>
<details><summary>Lab 03 · A float wrapper with a total order</summary>

Use `self.0.to_bits() == other.0.to_bits()` for `eq`, `self.0.total_cmp(&other.0)` for `cmp`, and `Some(self.cmp(other))` for `partial_cmp`. These agree, because `total_cmp` returns `Equal` exactly for identical bits, so the wrapper may implement `Eq` and `Ord`. The order differs from native float comparison in two ways. The wrapper says -0.0 is less than +0.0 and not equal to it, while `-0.0 == 0.0` for `f64`. And a NaN equals itself, two NaNs with different bits are unequal and ordered, and a positive NaN sorts after positive infinity.

</details>
<details><summary>Lab 04 · Index that returns a String; Index with a range</summary>

An `index` that ends in `&self.0[i].to_uppercase()` fails with E0515, `cannot return reference to temporary value`: the new `String` dies at the end of the method. `Index` can only return a reference to something the container stores. The range impl is `type Output = [i32];` with the body `&self.0[range]`. `[i32]` has no fixed size, which is allowed because `Output` is declared `?Sized`. The expression `&wrapper[1..3]` has type `&[i32]`, a pointer and a length.

</details>
<details><summary>Lab 05 · BitXor, BitAndAssign, and operators with other rules</summary>

`Flags(self.0 ^ rhs.0)` and `self.0 &= rhs.0`. Both identities hold for every value: `x ^ x == Flags(0)` and, after `y &= Flags(0xff)`, `y` is unchanged. The other operators are special in different ways. `*x` uses `Deref`, which returns a reference, and the compiler also inserts it by itself during method calls and coercions (chapter 13). Call syntax `f(x)` uses the `Fn`, `FnMut` and `FnOnce` traits; closures get them from the compiler, and a manual impl is rejected on stable Rust (E0183). `?` is defined through the `Try` trait, which is unstable (E0658), so on stable Rust it works only with the types the standard library prepared, such as `Result` and `Option`.

</details>

## Drills

Run `cargo drill ch12`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Operators take their operands by value</summary>E0382. `Add::add(self, rhs)` takes `wallet` by value, so `wallet + ...` moves it. Derive `Clone` and write `wallet.clone() + ...`, or also implement `Add<Money>` for `&Money` so you can write `&wallet + ...`.</details>
<details><summary>d02 · Add needs an Output</summary>E0046: `type Output` is missing. The result type of `+` is an associated type the impl must choose, here `type Output = Meters;`.</details>
<details><summary>d03 · Eq for a struct holding a float</summary>E0277: `Eq` can be derived only when every field is `Eq`, and `f64` is not, because NaN != NaN. Derive only `PartialEq`.</details>
<details><summary>d04 · Unary minus on an unsigned integer</summary>E0600. Unsigned integers do not implement `Neg`: there is no negative `u32`. Use a signed type (`let speed: i32 = 30;`), or express the intent with subtraction.</details>
<details><summary>d05 · Operand order matters</summary>E0277. `3 * v` calls `Mul<Vec2>` on `i32`, which does not exist; `Mul<i32> for Vec2` only covers `v * 3`. Add `impl Mul<Vec2> for i32` that forwards to `v * self`. Both the trait and `i32` come from the standard library, but the impl is still allowed. The orphan rule accepts an impl of a foreign trait when one of the types in it (`Self`, or a type parameter of the trait) is defined in your crate and no bare generic `T` comes before that type. Here `Vec2` is the trait's type parameter.</details>
<details><summary>d06 · String + &String</summary>Compiles. `Add<&str> for String` needs `&str` on the right, and `&host` (a `&String`) deref-coerces to `&str`. The left `String` is consumed and its buffer reused; `host` is only borrowed.</details>
<details><summary>d07 · &str + &str</summary>E0369. Neither operand owns a buffer that could grow, so std defines no `Add` for `&str`. Start from an owned `String`: `first.to_owned() + second`, or use `format!` or `concat`.</details>
<details><summary>d08 · PartialOrd builds on PartialEq</summary>E0277: `PartialOrd<Rhs>` has `PartialEq<Rhs>` as a supertrait, because an ordering must agree with equality. Add `#[derive(PartialEq)]`, or better, derive `PartialEq, Eq, PartialOrd, Ord` together.</details>
<details><summary>d09 · Indexing a map with a missing key</summary>Panics. `Index` must return a reference, and there is nothing to point at, so `HashMap`'s `Index` impl panics. Use `ports.get("ssh")`, which returns `Option<&V>`, and handle `None`.</details>
<details><summary>d10 · Compound assignment is its own trait</summary>E0368. `+=` is `AddAssign`, not `Add`. Implement `AddAssign` (`self.0 += rhs.0`). Outside this drill you could also write `total = total + points`, which works because `Score` is `Copy`; here that line is marked KEEP.</details>

## Mastery check: model answers

Answer each question in [NOTES.md](NOTES.md) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**Explain operand ownership and why `Add<Rhs>` has an associated `Output`.** `Add::add(self, rhs)` takes both operands by value, so `a + b` moves them unless their types are `Copy`. `==` and `<` take both operands by reference and move nothing. The assignment traits take `&mut self` and change the left operand in place. `Rhs` is a type parameter because one type may accept several right-hand types, each with its own impl. `Output` is an associated type because, once the two operand types are fixed, there is exactly one result type, and the impl chooses it. It may differ from both operands: a point in time minus a point in time is a length of time.

**Keep `PartialEq`, `Eq`, `PartialOrd`, and `Ord` consistent, including hashing implications.** `eq` must be symmetric and transitive; `Eq` promises that it is also reflexive. `partial_cmp` returns `Some(Equal)` exactly when `eq` is true, and for a type with `Ord` it returns `Some(self.cmp(other))`. Write the logic once, in `cmp`, and delegate. Deriving all four from the same fields keeps them consistent for free. `Hash` must follow the same identity as `eq`: values that compare equal must produce the same hash, or a `HashMap` can store two equal keys or fail to find one. So a hand-written `PartialEq` needs a hand-written `Hash`.

**Explain why floats cannot promise `Eq` or a natural total order.** NaN is not equal to itself, so float `==` is not reflexive, and `Eq` would be a false promise. NaN is also neither less than, equal to, nor greater than any value, so `partial_cmp` returns `None` and no total order exists under the usual comparison. That is why `f64` has `PartialEq` and `PartialOrd` only, and why `sort()` does not compile for a `Vec<f64>`. `f64::total_cmp` defines a separate total order over all bit patterns; it puts -0.0 before +0.0 and gives NaNs fixed places, so it does not agree with `==`.

**Implement indexing that returns a reference and assignment that mutates in place.** `Index::index(&self, idx) -> &Self::Output` returns a reference to an element the container stores, and `container[idx]` means `*container.index(idx)`. `IndexMut::index_mut(&mut self, idx) -> &mut Self::Output` is used when the element is assigned or mutably borrowed. Both use the same checked mapping from index to element, and panic with a clear message for an invalid index. In-place assignment operators such as `+=` are the `...Assign` traits: they take `&mut self` and change the fields, with no new value and no `Output`.

**Name operators that Rust does not allow user code to overload.** `&&` and `||` (they may skip the right operand), `=` (always a move or a copy), `as`, the borrow operators `&` and `&mut`, the range operators `..` and `..=` (they always build a range value), and field access with `.`. On stable Rust you also cannot implement the traits behind `?` or behind call syntax `f(x)` for your own types.

**Redo the capstone; present your work.** These two items are tasks, not questions. For the capstone, compare your new version with [the solution](../../solutions/exercises/chapter12/l05_capstone.rs) only after its tests pass.

</details>
