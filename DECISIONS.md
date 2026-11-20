# My Rust decision rules

[Course overview](README.md)

Fluency means making these calls quickly and correctly while you write real code. Each unit's `NOTES.md` records what you learned there. This page condenses the whole course into the rules you apply.

**How to use it.** When you finish the unit in the "After" column, write the rule in your own words, with one example from your own code. Only then open the model answer below the table and compare. Rewrite your rules during the spaced reviews as your understanding sharpens. A model answer is one good rule, not the only one; if yours differs, make sure you can say why.

## Core chapters

| # | After | Decision | My rule and an example |
| --- | --- | --- | --- |
| [D1](#d1) | Ch3 | Which integer type, and which overflow policy (wrapping, checked, saturating, strict)? | |
| [D2](#d2) | Ch3 | `String`, `&str`, `Vec<u8>`, `PathBuf` or `OsString` at a boundary? | |
| [D3](#d3) | Ch4 | Move, clone, or share with `Rc`/`Arc`? | |
| [D4](#d4) | Ch5 | Should a function take `T`, `&T` or `&mut T`? Return owned or borrowed? | |
| [D5](#d5) | Ch5 | When do I need an explicit lifetime, and what does it promise? | |
| [D6](#d6) | Ch7 | `panic!`, `Result` or `Option`? `?` or `match`? One error enum or `Box<dyn Error>`? | |
| [D7](#d7) | Ch8 | What is `pub`, `pub(crate)` or private? How do I lay out modules? | |
| [D8](#d8) | Ch9 | Field visibility, constructors, and when to use `Cell`/`RefCell`? | |
| [D9](#d9) | Ch10 | Enum or trait object? How do I make invalid states unrepresentable? | |
| [D10](#d10) | Ch11 | Generic `T: Trait`, `impl Trait`, or `dyn Trait`? | |
| [D11](#d11) | Ch12 | Operator or named method? Which comparison traits? | |
| [D12](#d12) | Ch13 | Which conversion trait: `From`, `TryFrom`, `AsRef`, `Borrow` or `Into`? When is `Cow` worth it? | |
| [D13](#d13) | Ch14 | Which closure bound: `Fn`, `FnMut` or `FnOnce`? When do I need `move`? | |
| [D14](#d14) | Ch15 | Iterator chain or `for` loop? When to `collect`? `impl Iterator` or `Box<dyn Iterator>`? | |
| [D15](#d15) | Ch16 | `Vec`, `VecDeque`, `HashMap`, `BTreeMap`, `HashSet` or `BinaryHeap`? | |
| [D16](#d16) | Ch17 | Bytes, chars or graphemes? `format!` or `write!`? | |
| [D17](#d17) | Ch18 | Buffered or unbuffered? Where do I handle `io::Error`? | |
| [D18](#d18) | Ch19 | Scoped threads, channels, `Mutex`, `RwLock`, or atomics? | |
| [D19](#d19) | Ch19, M3 | Which memory ordering? | |
| [D20](#d20) | Ch20–21 | Threads or async? What must be `Send`, and what is cancellation-safe? | |
| [D21](#d21) | Ch22, M5 | Function, generic, `macro_rules!` or procedural macro? | |
| [D22](#d22) | Ch23 | When is `unsafe` justified, and what goes in the `// SAFETY:` comment? | |

## Ecosystem track and mastery modules

| # | After | Decision | My rule and an example |
| --- | --- | --- | --- |
| [D23](#d23) | E01 | Library errors or application errors: thiserror or anyhow? | |
| [D24](#d24) | E02 | How do I keep a wire format compatible as it changes? | |
| [D25](#d25) | M1 | Typestate or a run-time state check? When to seal a trait? | |
| [D26](#d26) | M2 | How do I design an unsafe abstraction that no safe caller can break? | |
| [D27](#d27) | Ch24, M6 | At an FFI boundary: who allocates, who frees, and do I write bindings by hand? | |
| [D28](#d28) | E06, M4 | Which async runtime and flavour? Where does blocking or CPU-heavy work go? | |
| [D29](#d29) | M7 | When do I optimise, and how do I know it helped? | |
| [D30](#d30) | M8 | Is this change breaking? How do I leave room for the next version? | |
| [D31](#d31) | E09, M11 | How do I structure a service, and at which level do I test HTTP handlers? | |
| [D32](#d32) | Game capstone | Who owns the game world, and how do I keep the simulation deterministic? | |
| [D33](#d33) | Kernel capstone | Where does `unsafe` live in a kernel, and how is shared state protected from interrupts? | |

## Model answers

Open one only after you have written your own rule.

<a id="d1"></a>
<details><summary><b>D1</b> · Integer types and overflow</summary>

Use `usize` for indices and lengths, `u64` or `i64` for counts and quantities that can grow, and the narrowest type a format demands at a boundary (a `u16` port, a `u8` byte). Decide the overflow policy where overflow is possible, because the default differs by profile: a debug build panics and a release build wraps. Use `checked_*` for values from outside the program (return an error when it fails), `saturating_*` for counters that should stop at a limit, `wrapping_*` only where wrapping is the meaning (hashes, checksums, sequence numbers), and `strict_*` where overflow is a bug that must panic even in release. Example: `let total = a.checked_add(b).ok_or(Error::TooLarge)?;`.

</details>

<a id="d2"></a>
<details><summary><b>D2</b> · Text, bytes and paths at a boundary</summary>

Use `&str` to accept text you only read and `String` to store or return text you own. Use `&[u8]` and `Vec<u8>` for data that is not guaranteed to be UTF-8, such as network input or file contents before validation. Use `&Path` and `PathBuf` for file names, because on Unix and Windows a file name need not be valid Unicode, and `OsStr`/`OsString` for environment variables and arguments you pass through unchanged. Convert to `str` once, at the edge, and handle the error there. Example: `fn load(path: &Path) -> io::Result<Vec<u8>>`, then `str::from_utf8` when the format says it is text.

</details>

<a id="d3"></a>
<details><summary><b>D3</b> · Move, clone or share</summary>

Move when the receiver should own the value and the caller is done with it; that is free. Clone when two parts of the program really need independent copies and the cost is acceptable; a clone that only silences the borrow checker is a design smell. Share with `Rc` when several owners in one thread need the same value and no single owner outlives the others, and with `Arc` when the owners are on different threads. Example: a parsed configuration read by every worker thread becomes `Arc<Config>`, cloned once per thread (a pointer copy and a counter increment).

</details>

<a id="d4"></a>
<details><summary><b>D4</b> · Parameters and return values</summary>

Take `&T` when the function only reads, `&mut T` when it changes the value in place, and `T` when it stores or consumes the value. Accept the most general borrowed form: `&str` rather than `&String`, `&[T]` rather than `&Vec<T>`. Return an owned value when you create new data, and a reference only when you return a view into one of the arguments. Example: `fn longest<'a>(words: &'a [String]) -> Option<&'a str>` returns a view; `fn normalise(text: &str) -> String` creates new text.

</details>

<a id="d5"></a>
<details><summary><b>D5</b> · Explicit lifetimes</summary>

You need one when a function returns a reference and the elision rules cannot tell which input it borrows from (usually two or more reference parameters and no `&self`), and when a struct or enum holds a reference. A lifetime parameter does not make anything live longer. It states a relation that the compiler then checks: "the result is valid only while this argument is borrowed". Example: in `fn pick<'a>(a: &'a str, _b: &str) -> &'a str`, the caller may drop `_b` at once but must keep `a` alive while it uses the result.

</details>

<a id="d6"></a>
<details><summary><b>D6</b> · Panic, Result, Option, and error types</summary>

Panic when the program itself is wrong: a broken invariant, an index your own code computed. Return `Result` when the world can be wrong (input, files, the network) and a caller could reasonably react. Return `Option` when absence is normal and needs no explanation, such as a lookup. Use `?` to pass an error up and `match` (or a method such as `unwrap_or_else`) where you can handle it. A library exposes an error enum so callers can match on the cases; an application or a prototype can use `Box<dyn Error + Send + Sync>` and report at the top. Example: `fn parse_port(s: &str) -> Result<u16, ConfigError>`.

</details>

<a id="d7"></a>
<details><summary><b>D7</b> · Visibility and module layout</summary>

Start private. Make an item `pub(crate)` when other modules of the same crate need it, and `pub` only when it is part of the API you promise to keep. Group modules by responsibility (parsing, storage, output), not by kind (all structs in one file), and re-export a small facade from the crate root so users write `mylib::Parser`, not `mylib::parse::inner::Parser`. Integration tests in `tests/` can only see `pub` items, which makes them a good check of what the API really offers.

</details>

<a id="d8"></a>
<details><summary><b>D8</b> · Fields, constructors and interior mutability</summary>

Make fields private and provide a constructor when the struct has an invariant (a validated email, a sorted vector, `min <= max`); the constructor is the one place that checks it. Public fields are fine for plain data with no invariant, such as a point. Use `Cell` for small `Copy` values that must change behind a shared reference (a counter), and `RefCell` for single-threaded data that must change behind `Rc` (a graph node, a cache); remember that `RefCell` moves the borrow check to run time and panics on a conflict. First ask whether a `&mut` method or a different owner would remove the need.

</details>

<a id="d9"></a>
<details><summary><b>D9</b> · Enums, trait objects and invalid states</summary>

Use an enum when you know every alternative and own the type: the compiler then checks that every `match` handles every case. Use a trait object when other code must be able to add alternatives later, such as plugins. Make invalid states unrepresentable by putting data inside the variant that needs it: `enum Connection { Closed, Open { socket: TcpStream } }` instead of `is_open: bool` plus `socket: Option<TcpStream>`, and by using newtypes for validated values. Example: `struct Percent(u8)` with a `TryFrom<u8>` that rejects values above 100.

</details>

<a id="d10"></a>
<details><summary><b>D10</b> · Generics, `impl Trait` and `dyn Trait`</summary>

Use a generic parameter when the caller chooses the type and you want static dispatch, or when several arguments must share one type. `impl Trait` in argument position is a shorter way to write an anonymous generic; in return position it hides one concrete type, such as an iterator or a closure. Use `dyn Trait` for collections of different types, for types chosen at run time, and to cut code size or compile time when the dynamic call is cheap compared with the work. Example: `fn render(shapes: &[Box<dyn Shape>])` for a mixed list; `fn total<I: Iterator<Item = u32>>(it: I)` for one fast loop.

</details>

<a id="d11"></a>
<details><summary><b>D11</b> · Operators and comparison traits</summary>

Implement an operator only when its meaning is the one every reader expects: `+` for vectors and money, `*` for matrices, `[]` for container access. For anything else, a named method is clearer (`a.merge(b)`, not `a + b` for two configurations). Derive `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord` together when field-by-field comparison is right, because derived implementations agree with each other; write them by hand only together, and keep equal values hashing equal and `cmp` consistent with `==`. Floating-point types are only `PartialOrd`; sort them with `f64::total_cmp`.

</details>

<a id="d12"></a>
<details><summary><b>D12</b> · Conversion traits and `Cow`</summary>

Implement `From` for conversions that cannot fail or lose information, and `TryFrom` for those that can; implement `From`, never `Into`, because `Into` comes for free. Accept `impl AsRef<str>` or `impl AsRef<Path>` when a function only needs a cheap borrowed view of several possible types. Use `Borrow` where a borrowed form must hash and compare exactly like the owned one, which is why `HashMap<String, V>` can be searched with a `&str`. Return `Cow<'_, str>` when a function usually returns its input unchanged and only sometimes allocates, such as an escaping function.

</details>

<a id="d13"></a>
<details><summary><b>D13</b> · Closure bounds and `move`</summary>

Ask for the bound that lets the most closures in: `FnOnce` if you call it once, `FnMut` if you call it many times in sequence, `Fn` if you call it through a shared reference or from several threads. The bound you choose decides what callers may capture, so a needlessly strict `Fn` rejects closures that count or collect. Write `move` when the closure must outlive the current scope (a spawned thread, a returned closure, a stored callback), or when it should own a cheap value such as an `Arc` clone.

</details>

<a id="d14"></a>
<details><summary><b>D14</b> · Iterators, loops and collecting</summary>

Use an iterator chain when you transform or summarise a sequence and the steps read like a sentence (`filter`, `map`, `sum`). Use a `for` loop when the body has several side effects, complex early exits, or updates more than one thing. `collect` only when you need the collection itself; pass an iterator along otherwise. Return `impl Iterator` when one concrete type is returned, and `Box<dyn Iterator>` when different branches return different iterator types. Example: `lines.iter().filter(|l| !l.is_empty()).count()` needs no intermediate `Vec`.

</details>

<a id="d15"></a>
<details><summary><b>D15</b> · Choosing a collection</summary>

Start with `Vec`: it is the fastest for iteration and appending, and a sorted `Vec` with binary search is often the best set. Use `VecDeque` for a queue that adds and removes at both ends, `HashMap` and `HashSet` for fast lookup by key when order does not matter, and `BTreeMap` and `BTreeSet` when you need sorted iteration, range queries or output that does not depend on hashing. Use `BinaryHeap` for "the largest next" (with `Reverse` for the smallest) and for top-k of a stream. Example: a leaderboard that is printed in rank order is a `BTreeMap`, or a `Vec` sorted once at the end.

</details>

<a id="d16"></a>
<details><summary><b>D16</b> · Bytes, chars, graphemes and formatting</summary>

Work on bytes for protocols and ASCII formats, where the meaning is defined per byte. Work on `char`s for Unicode-aware processing such as case changes and classification. A user-perceived character (a grapheme cluster, such as an emoji with a skin-tone modifier) can be several `char`s; the standard library cannot split those, so use a crate such as `unicode-segmentation` for cursor movement and truncation of displayed text. Use `format!` when you need a new `String`, and `write!` into an existing buffer or a `Formatter` to avoid a temporary allocation, for example inside a `Display` implementation.

</details>

<a id="d17"></a>
<details><summary><b>D17</b> · Buffering and I/O errors</summary>

Wrap a reader in `BufReader` and a writer in `BufWriter` when you do many small reads or writes (line by line, record by record); one large `fs::read` or `write_all` needs no buffer. Call `flush` on a `BufWriter` yourself and check its result, because dropping it flushes but discards any error. Handle `io::Error` where you can decide what to do: retry, skip a file, or report. Below that level, add context (which file, which operation) and pass it up with `?`.

</details>

<a id="d18"></a>
<details><summary><b>D18</b> · Choosing a concurrency tool</summary>

Use scoped threads when the work borrows local data and must finish before the function returns. Use channels when work or ownership flows in one direction, as in a pipeline. Use a `Mutex` around state that several threads change and whose invariant covers more than one field; keep the guarded section short. Use `RwLock` only when reads dominate and are long enough to matter. Use atomics for a single counter or flag. If you are unsure, a channel or a `Mutex` is easier to get right than atomics.

</details>

<a id="d19"></a>
<details><summary><b>D19</b> · Memory orderings</summary>

Use `Relaxed` when the atomic itself is the only thing that matters, such as a statistics counter read at the end. Use a `Release` store paired with an `Acquire` load when the atomic publishes other data: everything written before the store is visible after a load that sees it. Read-modify-write operations that both take and publish, such as a lock's state or a reference count's last decrement, use `AcqRel` (or `Release` plus an `Acquire` fence). `SeqCst` is needed only when correctness depends on one global order of operations on several different atomics. Write the pairing down next to the code, and check it with loom (M3).

</details>

<a id="d20"></a>
<details><summary><b>D20</b> · Threads or async; `Send`; cancellation</summary>

Use threads for CPU-bound work and for a handful of blocking tasks. Use async when the program waits on many I/O operations at once, such as a server with thousands of connections. A future spawned on a multi-threaded runtime must be `Send`, so do not hold an `Rc` or a `std::sync::MutexGuard` across an `.await`. A future is cancellation-safe when dropping it at any `.await` loses no data and leaves no state half-changed; check this for every branch of a `select`. Example: keep a partially read message in a buffer owned outside the future, not in a local of the future.

</details>

<a id="d21"></a>
<details><summary><b>D21</b> · Functions, generics and macros</summary>

Write a function first. Make it generic when only the types vary. Write a `macro_rules!` macro only when you need what a function cannot do: a variable number of arguments of different types, new items (functions, impls, tests) generated from a list, or a small syntax of your own. Write a procedural macro when the input is Rust code that must be parsed, as in a derive. Every macro costs readers and error messages something, so keep its job small and its expansion obvious.

</details>

<a id="d22"></a>
<details><summary><b>D22</b> · Justifying `unsafe`</summary>

Use `unsafe` only when safe code cannot express the operation (FFI, a primitive the standard library lacks, hardware access) or when a measured bottleneck cannot be removed safely. Keep each block small and put it behind a safe function whose signature makes misuse impossible. The `// SAFETY:` comment names every precondition of the unsafe operation and says why it holds at this point, with reference to the type's invariant, not to "this is fine". Run the tests under Miri.

</details>

<a id="d23"></a>
<details><summary><b>D23</b> · Library errors and application errors</summary>

A library returns its own error type, usually an enum with one variant per failure a caller might handle, implementing `std::error::Error` with `source()` for the cause; thiserror writes that boilerplate. An application mostly reports errors rather than matching on them, so `anyhow::Error` with `.context("reading config")` gives a readable chain with little code. Do not expose `anyhow::Error` from a library's public API: callers can no longer match on the cases.

</details>

<a id="d24"></a>
<details><summary><b>D24</b> · Wire-format compatibility</summary>

Treat field names, enum representations and number types as a public promise. Add new fields as optional (`#[serde(default)]`) so old messages still parse. Do not reject unknown fields in data that newer peers send you, or old readers break when writers upgrade; reject them in configuration files, where a typo should fail loudly. Rename a Rust field freely, but keep its wire name with `#[serde(rename = "...")]`. When a change cannot be compatible, add a version field and keep a test with a stored sample of every version you accept.

</details>

<a id="d25"></a>
<details><summary><b>D25</b> · Typestate and sealed traits</summary>

Use typestate (a different type per state, such as `Request<Unsent>` and `Request<Sent>`) when the state is known at compile time for each value and calling a method in the wrong state is a bug. Use a run-time enum when the state comes from data or I/O, or when there are many states and transitions. Seal a public trait (a public trait with a private supertrait) when users should use it but not implement it, so that you can add methods later without a breaking change.

</details>

<a id="d26"></a>
<details><summary><b>D26</b> · Designing an unsafe abstraction</summary>

Write the invariant down first, as documentation on the type: which fields are initialised, who owns the allocation, which pointers may alias. Then check that every safe method keeps it, including when user code panics in the middle (a `Clone` or `Drop` that panics) and when a value is leaked with `mem::forget`. Every `unsafe` block's SAFETY comment cites the invariant. Test with Miri, and think about what your tests cannot reach: variance, `Send` and `Sync`, and the drop checker.

</details>

<a id="d27"></a>
<details><summary><b>D27</b> · FFI ownership and bindings</summary>

Memory is freed by the side that allocated it: a C object goes back through the library's own free function, a `CString` given to C with `into_raw` comes back with `from_raw`. Wrap every foreign handle in a Rust type whose `Drop` frees it exactly once, and tie borrowed foreign pointers to that owner's lifetime. Never let a panic cross into C; catch it at the boundary. Write bindings by hand for a few functions; for a large header, generate them with bindgen into a `-sys` crate, and check either kind against the real header.

</details>

<a id="d28"></a>
<details><summary><b>D28</b> · Async runtime and blocking work</summary>

For a network service, tokio's multi-threaded runtime is the usual choice; its current-thread flavour suits tests, small tools and code that holds non-`Send` values. A runtime is not the place for blocking calls or long computations: they stall every task on that worker. Move blocking calls to `spawn_blocking`, and send CPU-heavy work to a thread pool such as rayon, returning the result through a channel. Build your own executor (M4) to understand one, not to ship one.

</details>

<a id="d29"></a>
<details><summary><b>D29</b> · Optimising</summary>

Optimise when a measured number misses a stated budget, not before. First write a benchmark with realistic input, then profile to find where the time or the allocations go, change one thing, and measure again with enough repetitions to see whether the difference is real. Prefer changes that remove work (fewer allocations, a better data layout, a better algorithm) over clever code. Keep a test that counts the work (allocations, comparisons), so that the improvement cannot quietly disappear.

</details>

<a id="d30"></a>
<details><summary><b>D30</b> · API evolution and semver</summary>

A change is breaking if any reasonable user's code stops compiling or changes meaning: removing or renaming a public item, adding a required trait method, adding a public field to a struct that users can build with a literal, adding a variant to an exhaustive enum, or losing an auto trait such as `Send`. Leave room in the first version: `#[non_exhaustive]` on enums and structs that will grow, private fields with constructors, sealed traits, and builders for functions with many options. When in doubt, check the change against the Cargo book's SemVer compatibility chapter and a test crate that uses the old API.

</details>

<a id="d31"></a>
<details><summary><b>D31</b> · Service structure and handler tests</summary>

Keep HTTP handlers thin: they parse and validate input into domain types at the edge, call plain functions that hold the logic, and turn domain errors into status codes in one place. Shared state (a database pool, configuration) goes into the router's state, not into globals. Test most handlers in-process by sending requests to the router as a tower service, with a real in-memory or temporary database, which is fast and covers routing, extraction and serialisation. Keep a few tests that go over a real loopback socket, and test the domain functions without HTTP at all.

</details>

<a id="d32"></a>
<details><summary><b>D32</b> · Game architecture</summary>

One owner holds the whole world: entities in vectors addressed by index or by generational handle, not a web of `Rc<RefCell<..>>`. Advance the simulation in fixed time steps, separate from rendering, so that physics does not depend on the frame rate. Keep the simulation deterministic: no wall clock, no hash-order iteration, a seeded random number generator and integer or fixed-point arithmetic; then a replay of the inputs reproduces the game exactly and becomes a test.

</details>

<a id="d33"></a>
<details><summary><b>D33</b> · Kernel architecture</summary>

Confine `unsafe` to thin layers: the boot code, volatile access to device registers, page tables and the context switch. Build safe types on top, and put the logic (allocator, scheduler, ring buffers, message passing) in code that also compiles and is tested on the host. State shared with interrupt handlers is protected by a lock that is taken with interrupts disabled on that core, so a handler can never wait for a lock that the code it interrupted already holds. Every `static` with interior mutability states who may touch it and when.

</details>
