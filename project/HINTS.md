# loglens hints

[Project guide](README.md) · [All hints](../HINTS.md) · [Solutions](../solutions/project/)

Each milestone has two hints and one model answer:

- **Hint 1** points at the idea. Try it before you open **Hint 2**, which is close to the code.
- **Check your design answer** is a model answer to the milestone's design question in the [project guide](README.md). Open it only after you have written your own answer.

The technique constraints in each contract are part of the exercise: a hint never removes them. Each heading names the course chapter that the milestone follows and links its teaching text. If a hint uses a function or a term you do not know, read that chapter's `CONCEPTS.md` again before you open the solution.

## m03 · Numbers with an overflow policy

Follows [Chapter 3](../exercises/chapter03/CONCEPTS.md).

<details><summary>Hint 1</summary>

Walk `text.bytes().enumerate()`. A byte `b` is a digit when `b.is_ascii_digit()`, and its value is `b - b'0'`. Keep the running value as an `Option<u64>` that becomes `None` once it overflows, and keep scanning so that a later invalid byte is still reported.

</details>
<details><summary>Hint 2</summary>

`value = value.and_then(|v| v.checked_mul(10)).and_then(|v| v.checked_add(digit))`. After the loop, `match (value, policy)` decides between `Ok(v)`, `Err(NumError::Overflow)`, and `Ok(u64::MAX)`. For `mean`, `(count != 0).then(|| total as f64 / count as f64)`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

A call such as `parse_u64(text, true)` does not say what `true` means; `parse_u64(text, Overflow::Saturate)` does, at every call site. An enum can also get a third policy later without changing the meaning of existing calls. Two functions would work for two policies, but callers that receive the policy from elsewhere (the filter rejects, the record saturates) would each need their own `if`.

</details>

## m04 · Zero-copy fields

Follows [Chapter 4](../exercises/chapter04/CONCEPTS.md) and [Chapter 5](../exercises/chapter05/CONCEPTS.md).

<details><summary>Hint 1</summary>

Slicing a `&str` (`&s[a..b]`) and `trim_ascii_start` both return views into the same bytes: no allocation. Find positions with `s.bytes().position(|b| b.is_ascii_whitespace())`. Every byte you search for is ASCII, so each position is a valid slice boundary. Do not use `trim_start` or `char::is_whitespace` here: they also treat Unicode spaces such as U+00A0 as whitespace, and this format does not.

</details>
<details><summary>Hint 2</summary>

For a field: find the first byte that is `=` or whitespace; it must be `=` at a position greater than 0. After the `=`, `strip_prefix('"')` tells you whether the value is quoted. In a quoted value, walk the bytes with an index: on `\` skip two bytes, on `"` stop; running off the end means "no field". The byte after the closing quote must be whitespace or nothing. `find_field` is `fields.iter().find(|f| f.key == key).map(|f| f.raw)`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

The value is a slice of the line, so the line is the only thing that must stay alive. With `Option<&'a str>` tied to the line, a caller can build the vector of fields in an inner block, look a value up, drop the vector, and still use the value. If the result borrowed from `fields`, the vector would have to live as long as every value taken from it, and `Record::get` could not return a value that outlives the record.

</details>

## m07 · Errors with positions

Follows [Chapter 6](../exercises/chapter06/CONCEPTS.md) and [Chapter 7](../exercises/chapter07/CONCEPTS.md).

<details><summary>Hint 1</summary>

`scan_field` is your m04 `split_field` with every `return None` replaced by `return Err(ErrorKind::...)`. Use `ok_or(ErrorKind::MissingEquals)?` where an `Option` becomes an error. Then `split_field` becomes `scan_field(s).ok()`.

</details>
<details><summary>Hint 2</summary>

Because `suffix` is a tail of `line`, it starts at byte `line.len() - suffix.len()`; count the characters before it with `chars().count()` and add 1. In `parse_fields`, keep the trimmed `rest` *before* each call, so that on failure `scan_field(rest).map_err(|kind| ParseError::new(kind, column_of(line, rest)))?` points at the field's first character. The `Display` impls are a `match` and `write!`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

The scanner knows *what* is wrong but not *where* in the whole line, and it does not know the line number at all. Splitting the types lets each layer add what it knows: `scan_field` returns a kind, `parse_fields` adds the column, the pipeline adds the line. A small `Copy` kind is also easy to compare in tests and to match on. One enum with positions in every variant would force the lowest layer to invent positions it does not have.

</details>

## m08 · Modules, a public API, doc tests

Follows [Chapter 8](../exercises/chapter08/CONCEPTS.md).

<details><summary>Hint 1</summary>

`Display` for `Error` is a `match` with one `write!` per variant; `path.display()` formats a `Path`. In `source`, return `Some(e)` for every wrapping variant: each wrapped type implements `std::error::Error + 'static`, so `&e` coerces to the trait object.

</details>
<details><summary>Hint 2</summary>

Each `From` impl is one line, such as `Error::Io(error)`. `?` calls `From::from` on the error, which is why `read_to_string(...)?` works in a function returning `loglens::Result`. For a doc test, copy the shape of the example on `parse_u64`: `use` the item through its public path, then `assert_eq!`. End an example that uses `?` with the hidden line `# Ok::<(), loglens::Error>(())`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

A useful rule: re-export at the root the names that appear in the signatures a typical user writes (`Record`, `Filter`, `Level`, `Sink`, `Error`), and leave the building blocks in their modules (`scan::split_word`). The root is then a short list of what the crate is about, and the module paths still tell an advanced user where a helper belongs. Every re-export is also a promise: moving the item later breaks no caller, but removing the short path does.

</details>

## m09 · The Level enum and the Record struct

Follows [Chapter 9](../exercises/chapter09/CONCEPTS.md) and [Chapter 10](../exercises/chapter10/CONCEPTS.md).

<details><summary>Hint 1</summary>

`from_name` can match with guards: `n if n.eq_ignore_ascii_case("info") => Some(Level::Info)`. For `is_timestamp`, a slice pattern names every byte at once: `let &[y1, y2, y3, y4, b'-', m1, m2, b'-', ...] = s.as_bytes() else { return false };`. Check the digits, then the ranges with `matches!(month, 1..=12)`.

</details>
<details><summary>Hint 2</summary>

In `parse_record`, start from `line.trim_ascii_start()` and keep each remaining text before splitting the next word, because that text is where the next column points. A small closure `let at = |kind, rest: &str| ParseError::new(kind, column_of(line, rest));` keeps the error lines short. `Record::number` is `self.get(key).map(|raw| parse_u64(raw, Overflow::Saturate)).transpose()`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

A log line has a handful of fields. Pushing them onto a `Vec` is one small allocation and keeps the line order, which `Display` needs to print the canonical line and which makes "the first field with this key" well defined when keys repeat. A linear search over five fields is faster than hashing a key. A `HashMap` would hash every key of every line, lose the order, and need a rule for repeated keys, all to speed up lookups that are already cheap.

</details>

## m11 · Traits: Sink, Display, FromStr, TryFrom

Follows [Chapter 11](../exercises/chapter11/CONCEPTS.md), [Chapter 12](../exercises/chapter12/CONCEPTS.md), and [Chapter 13](../exercises/chapter13/CONCEPTS.md).

<details><summary>Hint 1</summary>

`FromStr` reuses `from_name` with `ok_or_else`. `Display` for `Level` is `f.pad(self.as_str())`: `pad` applies width, alignment, and precision, while `write!` would ignore them. `TryFrom<u8>` can index `Level::ALL`.

</details>
<details><summary>Hint 2</summary>

`Display` for `Record`: one `write!` for the header, then one per field, each with `?`. The quoted form is `write!(f, " {}=\"{}\"", field.key, field.raw)`. `LineWriter::accept` is `writeln!(self.out, "{record}")`. The forwarding impls are `(**self).accept(record)`. In `Tee::finish`, store both results before combining them with `first.and(second)`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

`From` and `u8` both belong to the standard library, but the trait has a type parameter, and here it is `Level`: the impl is `From<Level> for u8`. The orphan rule asks that at least one of these is local to the crate: the trait, the type after `for`, or a type argument of the trait. (One more condition applies to generic impls: no bare type parameter `T` may come before the first local type.) Here the trait's argument `Level` is local, and that is enough. No other crate can write the same impl, because no other crate owns `Level`, so two impls can never collide. `impl From<u16> for u8` in this crate would be rejected: nothing in it is local.

</details>

## m14 · A filter language compiled to closures

Follows [Chapter 14](../exercises/chapter14/CONCEPTS.md).

<details><summary>Hint 1</summary>

Split the source with `split_ascii_whitespace`, compile each term, and `collect::<Result<Vec<_>, _>>()?`. In a term, `find(['=', '!', '<', '>', '~'])` locates the operator; match the following bytes against `[b'!', b'=', ..]`, `[b'>', b'=', ..]`, and so on to decide between one- and two-byte operators. A small private `enum Op` keeps the rest readable.

</details>
<details><summary>Hint 2</summary>

Build each predicate as `Box::new(move |r: &Record<'_>| ...)` and convert what the closure needs *before* you create it: `let wanted: Level = value.parse()...?;` or `let value = value.to_string();`. Annotate the closure parameter's type so that it works for records of any lifetime. One helper `fn holds<T: Ord + ?Sized>(op, left: &T, right: &T) -> bool` serves levels, timestamps, and numbers. For `user!=bob`, `Option::is_none_or` expresses "missing or different".

</details>
<details><summary>Check your design answer (after writing your own)</summary>

Every closure has its own anonymous type, even two closures with the same body. A `Vec<F>` holds values of one type `F`, so it could hold many copies of one closure, but not a level term next to a text term. `Filter<F>` would also put the closure type into every signature that mentions a filter, and nobody can write that type down. `Box<dyn Fn ...>` erases the type: all terms become "something callable with a record", at the price of one indirect call per term. For a list built at run time from user input, that is the right trade.

</details>

## m15 · Iterator pipelines

Follows [Chapter 15](../exercises/chapter15/CONCEPTS.md).

<details><summary>Hint 1</summary>

In `Records::next`, loop: take the next line with `self.lines.next()?`, increment the line number, `continue` on ignored lines, and return the parse result with `map_err(|e| e.at_line(n))`. A line is ignored when `line.trim_ascii_start()` is empty or starts with `#`; `trim_start` would also remove Unicode spaces, which this format treats as ordinary characters. `matching` is `Records::new(text).filter_map(Result::ok).filter(move |r| filter.matches(r))`: return that chain itself. If you collect it into a `Vec` first, the whole text is parsed before the caller sees the first record.

</details>
<details><summary>Hint 2</summary>

`slowest`: `filter_map` each record to `Some((value, record))` (`r.number(key).ok()??` is the value or `None`), then `min_by_key(|&(n, _)| Reverse(n))`. `min_by_key` keeps the *first* minimum, and the minimum of `Reverse(n)` is the maximum of `n`. `process` is a `for` loop over `Records` with a `match`; `sink.accept(&record)?` stops at the first sink error.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

With a callback, each error is handled when it is found and then dropped: the program prints it and goes on, and memory use stays flat however many lines are damaged. A returned `Vec<ParseError>` would hold ten million errors before the caller sees the first, and the caller could not stop early. The callback also lets each caller choose: print, count, collect into its own vector, or ignore. `&mut dyn FnMut` instead of a generic parameter keeps `process` from being compiled once per closure type; one indirect call per malformed line costs nothing that matters.

</details>

## m16 · Aggregation and top-k

Follows [Chapter 12](../exercises/chapter12/CONCEPTS.md) and [Chapter 16](../exercises/chapter16/CONCEPTS.md).

<details><summary>Hint 1</summary>

`key_of` is a `match` on the group. In `add`, `*self.levels.entry(level).or_insert(0) += 1` counts levels. For the group, `entry(key.to_string())` would allocate for every record; look the key up as a `&str` first.

</details>
<details><summary>Hint 2</summary>

Inside `add`, a `match` works: `let stats = match self.groups.get_mut(key) { Some(stats) => stats, None => self.groups.entry(key.to_string()).or_default() };`. So does `contains_key`, then `insert`, then `get_mut`. What today's borrow checker rejects (error E0499) is the same `match` as the *return value* of a helper function that returns the `&mut GroupStats`: there the first borrow has to last for the whole function, including the `None` arm. For `top`, push `Reverse((count, Reverse(name)))` into a `BinaryHeap` and `pop` whenever it holds more than `k`; `into_sorted_vec()` then yields the best first. In `render`, `{name:<width$}` pads to a width you compute from the rows.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

`levels` is printed in severity order, and a `BTreeMap<Level, u64>` iterates in key order, so the order comes for free and is the same on every run. There are at most five keys, so its slower lookup does not matter. `groups` may hold thousands of names and is looked up once per record, so it wants the O(1) lookup of a `HashMap`; its order is never shown directly, because `top` sorts what it reports. A `HashMap` for the levels would print them in a different order on each run. A `BTreeMap` for the groups would compare strings on every record to keep an order that nobody reads.

</details>

## m17 · Quoted values and Unicode

Follows [Chapter 17](../exercises/chapter17/CONCEPTS.md).

<details><summary>Hint 1</summary>

Start each function with the cheap check that lets you return `Cow::Borrowed`: no `\` for `unescape`, nothing to quote for `quote`, `s.chars().nth(max_chars).is_none()` for `truncate`. Only then build a `String`.

</details>
<details><summary>Hint 2</summary>

`unescape`: iterate over `chars()`; on `\`, look at `chars.next()` and `match` it. `truncate`: `s.char_indices().nth(max_chars - 1)` gives the byte offset at which to cut, which is always a character boundary; then `format!("{}…", &s[..cut])`. For the filter refactor, `record.text(key)` returns `Option<Cow<str>>`; compare with `*t == *value` or `t.contains(&*value)`. Matching still does not allocate for bare values and for quoted values without escapes; a value with an escape is decoded into a `String` first.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

Most values contain no escape. Returning `String` always would allocate and copy for every one of them, which hurts the caller that only compares or prints the text, and that is nearly every caller. Returning `&str` always is impossible for the values that do change: the decoded text is shorter than the raw text and exists nowhere until someone builds it. `Cow` says exactly this in the type: borrowed when nothing changed, owned when new text had to be built. Callers that only read use it like a `&str` through `Deref`; a caller that needs a `String` calls `into_owned` and pays only then.

</details>

## m18 · Streaming input and the CLI

Follows [Chapter 18](../exercises/chapter18/CONCEPTS.md).

<details><summary>Hint 1</summary>

`process_reader`: `loop { buf.clear(); if reader.read_until(b'\n', &mut buf)? == 0 { break } ... }`. Strip the newline with `strip_suffix(b"\n")` and then `strip_suffix(b"\r")`, convert with `std::str::from_utf8`, and hand the line to the same helper as `process`. `Follower::poll` uses `self.partial` as that buffer and stops when a read returns 0 or a line does not end in `\n`. It must return then, not sleep and try again: the waiting belongs in `run_with`, between two polls. (If a test reports that `Follower::poll` was still running after 30 seconds, this is the cause.)

</details>
<details><summary>Hint 2</summary>

`parse_args`: collect the arguments into a `Vec<OsString>`, scan once for `-h`/`--help`, then walk an iterator so that `args.next()` can take an option's value. `arg.as_encoded_bytes().starts_with(b"-")` checks for an option without converting to UTF-8.

In `run_with`, open every file with `open(path)`; the returned `Box<dyn BufRead>` goes straight into `process_reader` or `Follower::new`. Write one sink for all three commands. Let it remember two things: whether *its own* write failed, and how many records it accepted. The first tells a broken stdout from an unreadable input, since both reach you as an `io::Error` from `process_reader`. The second is the match count you still need when a read error made `process_reader` return `Err` without its tally. Check `error.kind() == io::ErrorKind::BrokenPipe` before you report a write error. In `--follow`, flush stdout after every poll, or the lines stay in the `BufWriter` that `main` wrapped around stdout.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

Some examples: `b_run_in_memory` compares the exact bytes written to stdout and stderr, which needs both to be buffers the test owns. `b_write_errors` passes a stdout that fails like a closed pipe, which no real terminal can be told to do on demand. `b_a_read_error_keeps_what_was_read_and_goes_on` passes an opener whose file fails after two lines; with `File::open` inside `run_with`, the test would need a real disk that breaks at the right moment. The rule behind all three: code that reaches for a global (the process's stdout, the file system) can only be tested by arranging that global. Code that receives it as a parameter can be given anything.

</details>

## m19 · Parallel files, identical results

Follows [Chapter 19](../exercises/chapter19/CONCEPTS.md).

<details><summary>Hint 1</summary>

Inside `std::thread::scope`, spawn `workers` threads that each loop: `let i = next.fetch_add(1, Ordering::Relaxed); let Some(item) = items.get(i) else { break };` then send `(i, f(item))` on an `mpsc` channel. Drop the original sender before you receive, or the receiving loop never ends: the channel closes only when *every* sender is gone, and the one you cloned from is still alive. (If a test tells you that `map_ordered` was still running after 30 seconds, this is the first thing to check.)

</details>
<details><summary>Hint 2</summary>

Pre-allocate `let mut slots: Vec<Option<R>> = items.iter().map(|_| None).collect();`, fill `slots[i]` as results arrive, and unwrap them after the scope. Capture `&next` and `&f` in each `move` closure, not the values.

In `run_with`, the items are the paths. The closure you pass to `map_ordered` does everything for one file on the worker's thread: it calls `open(path)`, reads the file, and writes into its own `Vec<u8>` for stdout and another for stderr. It returns those buffers with its partial `Tally` and `Summary`. Back on the calling thread, write the buffers and merge the counters in input order. If you open the files before `map_ordered` and pass the readers in, it does not compile: a `Box<dyn BufRead>` is not `Sync`.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

`T: Sync`: every worker holds the same `&[T]` and reads items through it, and sharing `&T` between threads is exactly what `Sync` allows. Without it, the `scope.spawn` call that captures `items` is rejected. `R: Send`: each result is created on a worker and moved to the calling thread, through the channel's `send`. `F: Sync`: all workers call the same closure through `&f`, so a shared reference to it crosses threads, again at `scope.spawn`. `F` does not need `Send`, because the closure itself never moves; and nothing needs `'static`, because the scope guarantees that every worker has finished before `map_ordered` returns.

</details>

## m20 · Optional: a std-only async reader

Follows [Chapter 20](../exercises/chapter20/CONCEPTS.md) and [Chapter 21](../exercises/chapter21/CONCEPTS.md).

<details><summary>Hint 1</summary>

`NextLine` holds a `&mut AsyncLines`, so it is `Unpin`, and `self.get_mut()` gives you `&mut NextLine` inside `poll`. Search `buf` for `\n` before you poll the source again; `buf.drain(..=i).collect()` removes one line from the front.

</details>
<details><summary>Hint 2</summary>

`block_on`: `struct Unpark(Thread); impl Wake for Unpark { fn wake(self: Arc<Self>) { self.0.unpark() } }`, then `Waker::from(Arc::new(Unpark(thread::current())))`, `Context::from_waker(&waker)`, and `let mut future = std::pin::pin!(future);`. Loop over `future.as_mut().poll(&mut cx)` and call `thread::park()` on `Pending`. An `unpark` that happened earlier makes the next `park` return at once, so no wake-up is lost. If a test reports that `block_on` was still running after 30 seconds, your waker does not unpark the thread that is parked.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

The rule is the contract of `Future::poll`: a future that returns `Pending` must have arranged for the waker of the current task to be called when progress is possible. The executor relies on it completely. After `Pending`, your `block_on` parks the thread and does nothing until `wake` runs. A real source stores the waker and calls it when data arrives; `Trickle` has no outside event, so it wakes at once. A source that returns `Pending` without doing either leaves `block_on` parked forever. Nothing is wrong in the executor then, and that is why this bug is hard to find: the program is idle, not busy, and no error appears.

</details>

## m22 · Macros: field! and log_line!

Follows [Chapter 22](../exercises/chapter22/CONCEPTS.md).

<details><summary>Hint 1</summary>

`field!(record, user)` expands to `$record.get(::core::stringify!($key))`. A method call auto-references, so `record` and `&record` both work, and `$record` appears once, so it is evaluated once. The `as u64` arm is `$record.number(...).ok().flatten()`.

</details>
<details><summary>Hint 2</summary>

`log_line!` expands to a block: `{ let mut line = String::new(); ... line }`. Write the header with `::std::fmt::Write::write_fmt(&mut line, ::std::format_args!(...))`, then repeat `$( ... )*` one statement per field that formats the value with `::std::string::ToString::to_string(&$value)` and appends `" key=value"` using `$crate::text::quote`. The variables `line` and `value` inside the macro cannot clash with the caller's: local variables of a `macro_rules!` macro are hygienic.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

A macro is worth its cost when it does something a function cannot: take a bare identifier as a key (`user`, checked for spelling by nobody, but free of quotes), accept any number of `key = value` pairs, or produce different types from one name, as `field!(r, latency_ms as u64)` does. `log_line!` passes that test: no function has that call shape. `field!(record, user)` is a close call. It saves little, hides a plain method call, and gives worse error messages and editor support than `record.get("user")`. In a public API, prefer the method and add the macro only if call sites clearly read better with it.

</details>

## m23 · Optional: a fast byte search, and when unsafe is worth it

Follows [Chapter 23](../exercises/chapter23/CONCEPTS.md) and [Chapter 24](../exercises/chapter24/CONCEPTS.md).

<details><summary>Hint 1</summary>

`let (words, tail) = haystack.as_chunks::<8>();` gives a `&[[u8; 8]]` and a tail of at most 7 bytes, with no `unsafe`. For each array, `u64::from_ne_bytes(*word)` is the eight bytes as one integer. XOR it with the needle repeated in every byte; a byte of the result is zero exactly where the needle is. The byte order of the integer does not matter, because you only ask whether *some* byte is zero.

</details>
<details><summary>Hint 2</summary>

When `x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 != 0`, some byte of this word matches: find it with `word.iter().position(...)` and add `8 * n` for the n-th word. Search the tail the same way and add `8 * words.len()`.

In `split_ascii`, `assert!(byte.is_ascii(), "... ASCII ...")` comes first. The safety argument has two parts. In bounds: `find_byte` returned an index `at < s.len()`, so `..at` and `at + 1..` are valid ranges. On `char` boundaries: UTF-8 never uses a byte below 0x80 inside a multi-byte character, so an ASCII byte is always a whole character.

For the optional `find_byte_unaligned`, loop `while i + 8 <= haystack.len()`. That condition is the whole bounds proof for `haystack.as_ptr().add(i).cast::<u64>().read_unaligned()`, and `read_unaligned` is needed because a `&[u8]` may start at any address.

</details>
<details><summary>Check your design answer (after writing your own)</summary>

As written, `split_ascii` is a safe function: no input can make it misbehave, so the safety argument lives in one place, next to the one `unsafe` block, and the author of that block maintains it. The `assert!` is what makes the argument true for every caller. As an `unsafe fn` with the requirement "`byte` must be ASCII", the proof would move to every call site: each caller would need its own `// SAFETY:` comment, and one wrong caller could split a string inside a character and create an invalid `&str`. The check costs one comparison. Prefer the safe function with a checked precondition, and make a function `unsafe` only when the check is impossible or measurably too slow.

</details>
