<!-- Lesson 07, task D: the advisory for the unsound function, in the form that the
     project would publish with the fixed release. Keep the section headings. -->

# `Workspace::read_range` can return a `&str` that reaches outside the file

## Affected versions

tasklane-plugin 0.4.0, the only release that has `Workspace::read_range`.
tasklane-core 0.4.0 is affected through the built-in `slice` step, which passes
numbers from the Lanefile to this function.

## Description

`Workspace::read_range(path, start, len)` is a safe function. Its documentation
promises `None` when the range does not lie inside the file. The implementation
checked only that `start` is not greater than the length of the file. It then built
the result with `slice::from_raw_parts` and `str::from_utf8_unchecked`, without
checking `start + len` against the length and without checking that both ends of the
range fall on character boundaries.

Safe code can therefore do two things that must be impossible: get a `&str` whose
bytes lie partly or completely after the end of the file's heap allocation, and get a
`&str` that is not valid UTF-8 because it begins or ends inside a multi-byte
character. No `unsafe` is needed in the calling code. A Lanefile is enough:
`step slice out.txt in.txt 2 100` on a short file reaches the first case.

## Impact

Reading through the returned `&str` is an out-of-bounds read. In practice it copies
whatever bytes follow the file's content in memory into another workspace file or
into the log, which can expose unrelated data of the process, or it crashes. Code
that receives the non-UTF-8 `&str` may misbehave in any way, because the standard
library relies on `str` being UTF-8. We have not shown a way to write out of bounds.

## Reproduction

```rust
use tasklane_plugin::Workspace;

#[test]
fn read_range_past_the_end() {
    let mut ws = Workspace::new();
    ws.write("file", "hello");
    // 2 + 4 > 5: the range ends one byte after the file.
    assert!(ws.read_range("file", 2, 4).is_none());
}
```

With 0.4.0 an ordinary `cargo test` only reports the failed assertion. Run the test
under Miri (`cargo +nightly-2026-11-20 miri test`) to see the undefined behaviour itself: Miri
stops inside `read_range` at `slice::from_raw_parts`. Miri does not report the second
case, the split character, because an invalid `str` breaks a library rule and not a
rule of the language; a test for it has to check the returned value.

## Fix

Fixed in 0.4.1. `read_range` computes the end with `checked_add` and slices with
`str::get`, which checks both bounds and both character boundaries. The function no
longer contains `unsafe`. For every range that was valid before, it returns the same
borrowed slice as before; for every other range it returns `None`, as the
documentation always said.

## Workaround

Until you can upgrade: do not call `read_range` with numbers that come from input.
Check them yourself first, for example with
`text.get(start..start.checked_add(len)?)` on the result of `Workspace::read`, and do
not run Lanefiles from people you do not trust if they use the `slice` step.
