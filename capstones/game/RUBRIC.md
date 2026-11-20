# Rubric for the game capstone

The tests tell you whether the rules are right. This rubric is for
everything a test cannot see. Go through it when all tests pass, and be
honest: the point is to find what to improve, not to collect a grade.

Each row has three levels. "Working" is a finished capstone. "Strong" is
what a careful professional would hand in. Most first attempts are a mix.

## 1. Gate: the things that are simply true or false

All of these must hold before the rest matters.

- [ ] `cargo test --no-default-features` passes in debug and with
      `--release`.
- [ ] `cargo test` (with the window crate) passes in debug and with
      `--release`.
- [ ] `cargo clippy --all-targets` and
      `cargo clippy --all-targets --no-default-features` print no warnings.
- [ ] `cargo run --release` opens a window, and the level can be played
      from start to win and from start to loss.
- [ ] The headless output for `assets/demo.replay` is byte-for-byte the
      same from a debug build and a release build.
- [ ] `grep -rn "f32\|f64" src/` finds nothing, and `grep -rn "unsafe" src/`
      finds at most a `#![forbid(unsafe_code)]` line.
- [ ] The library does not mention the window crate:
      `grep -rn "minifb" src/` finds it only in files of the binary, under
      `#[cfg(feature = "window")]`.
- [ ] Nothing in the game is taken from an existing game: no names,
      characters, levels, pictures or sounds.

## 2. Architecture

| | not yet | working | strong |
|---|---|---|---|
| Layers | simulation, drawing and window code call each other freely | the library has no window code; `step` does not draw; `render` takes `&self` | a reader can name the layers from the module list alone; each module has one job and a short doc comment saying which |
| The platform layer | key codes or window types appear in several files | one file names the window crate | that file is under about 100 lines and could be rewritten for another crate without touching anything else |
| The binary | game logic lives in `main.rs` | `main.rs` parses arguments and calls into the library | windowed and headless mode share everything except their front end; bad arguments give a clear message and exit code 2 |
| The facade | internal types leak through extra `pub` items | only the SPEC's items and what the binary needs are public | internals are `pub(crate)` or private; the public surface is documented |

## 3. The entity store and ownership

| | not yet | working | strong |
|---|---|---|---|
| References between entities | `Rc<RefCell<..>>`, or lifetimes on entity structs | entities are plain data in `Vec`s or arrays and refer to each other by index or handle | you can explain, for your store, what happens to a handle after its entity is removed, and a unit test shows it |
| Borrowing in `step` | clones of large structures to satisfy the borrow checker | helper functions borrow the fields they need; no cloning per step | the data is split so that what is read together lives together; no `clone()` in the frame path at all |
| Order | iteration order depends on a `HashMap` or on removal history | enemies and sparks are processed in spawn order as the SPEC says | the code makes the order obvious (one loop, one comment) and a removed enemy cannot disturb it |

## 4. Determinism and numbers

| | not yet | working | strong |
|---|---|---|---|
| Arithmetic | floats, or integer overflow that happens to work in release | integers only; debug builds never panic on overflow in normal play | every wrapping operation is spelled `wrapping_*`; units (pixels, subpixels, tiles) are named in constants or types; `div_euclid` or shifts where negatives can occur |
| Random numbers | effects and simulation share a generator | two streams; only Mistwings draw from the simulation's | the rule "who may draw" is stated in one place in the code |
| Evidence | "it looks the same" | golden traces pass; debug and release fingerprints match | you also compared two operating systems, or you can say exactly why it must match there |

## 5. The rasteriser

| | not yet | working | strong |
|---|---|---|---|
| Clipping | a per-pixel bounds test everywhere, or panics at the edges | rectangles and sprites are clipped once, then filled without tests | the clip is one small function used by rectangles, sprites and blended areas; extreme arguments are handled by widening, not by special cases |
| Inner loops | index arithmetic per pixel | rows are handled as slices (`fill`, `chunks`, iterators) | the hot loops have no bounds checks that the compiler could not remove, and you looked at a profile to know which loops are hot |
| The font | glyphs typed into source by hand | the font file is parsed, at start-up or at compile time | a mistake in the font file is reported clearly (at compile time or with a message naming the glyph) |

## 6. Performance

| | not yet | working | strong |
|---|---|---|---|
| Allocation | the allocation test fails, or passes only in release | zero allocations per frame in debug and release | you can list where your start-up allocations happen and why each is needed |
| Frame cost | a release build cannot hold 60 frames per second | comfortable at 60 frames per second | you measured a frame (a simple loop of 10000 `step` + `render` calls and a stopwatch is enough) and know where the time goes |
| Recording | the input record reallocates during play | capacity is reserved once | long sessions are handled deliberately (a stated limit, or run-length encoding while recording) |

## 7. Code quality

| | not yet | working | strong |
|---|---|---|---|
| Constants | numbers from the SPEC appear as literals in the logic | all SPEC constants are named, in one place | names and units match the SPEC table, so a reviewer can check them line by line |
| Errors | `unwrap()` on user input (files, arguments, level text) | every user error becomes a message and exit code 2 | error types implement `Display` and `Error`; messages say what and where (line, column) |
| Tests of your own | none | unit tests for the parts the acceptance tests cannot reach (your store, your clip function, your font parser) | when a bug cost you an hour, there is now a test with its name on it |
| Comments | none, or comments that repeat the code | each module says what it is for | comments explain decisions: why this order, why this type, what would break |

## 8. The game itself

Ask someone who has not seen it to play for five minutes. Watch without
explaining.

| | not yet | working | strong |
|---|---|---|---|
| Readability | the player cannot tell Nib, enemies, sparks and background apart | everything important is recognisable at a glance | a new player understands the goal without reading anything |
| Feel | movement is stiff or slippery; deaths feel unfair | running and jumping feel under control | landing, stomping, getting hurt and winning each have a visible reaction (particles, shake, animation) that a player would miss if you removed it |
| Looks | flat rectangles | sprites, a background, a consistent set of colours | motion in the scenery, light around bright things, variation in the tiles; a screenshot you would show someone |
| Completeness | no way to restart; closing is the only exit | win and loss screens, restart key | messages tell the player what to press; a second level, or a level of your own design |

## 9. Reflection

Write a short `DESIGN.md` for your version (the model solution has one).
Answer at least these:

1. How are entities stored, and what did you try first?
2. Which borrow-checker error cost you the most time, and what was the
   real cause?
3. Where exactly is the line between simulation and presentation in your
   code? Name one thing that was on the wrong side for a while.
4. What was the first non-determinism you found, and how did you find it?
5. If you had to add a second player, what would have to change?

If you can answer all five in a few sentences each, you understood the
project, whatever the tables above say.
