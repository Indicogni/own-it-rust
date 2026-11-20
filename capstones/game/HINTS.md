# Hints for the game capstone

These are design hints. They contain no code, and they do not tell you
what to type. Each milestone has a first hint (a direction), a second hint
(more specific), and questions to check your understanding. Try for a
while before reading a hint, and read the second one only if the first
did not help.

When you are stuck on a failing test, read the test first. Every assertion
has a message, and the tests build tiny levels that you can draw on paper.

---

## M1 · Framebuffer and rasteriser

**Hint 1.** Write the smallest thing first: creating a buffer, setting one
pixel, reading it back. Then build every other operation on a clear idea of
"which pixels, exactly". For each operation, write that set down as a
sentence with `<=` and `<` before you write a loop.

**Hint 2.**

- Clipping a rectangle is the same problem on two axes, and each axis is
  "intersect two ranges". Solve it once, in a type wide enough that no sum
  can overflow, and return "nothing visible" as a value. Rectangles and
  sprites can then share it.
- For a sprite, decide the visible rectangle in *destination* coordinates
  first. The source column for a destination column is then one
  subtraction, and flipping is one more.
- The line algorithm in the SPEC is complete. Translate it line by line;
  do not "simplify" the two `if`s into `if`/`else`.
- For the font, decide when the text file is turned into a table: every
  call, once at start-up, or at compile time. Each choice works; think
  about which mistakes each one reports, and when.
- The checksum and the PPM writer both walk all pixels in storage order
  and need bytes. Choose the byte order explicitly.

**Check your explanation.**

- Why does the test for extreme rectangle arguments fail with an `i32`
  sum in a debug build and in a different way in a release build?
- Why can clipping a line by moving its end point to the edge change
  pixels in the middle of the screen?
- What does a sprite pixel `0x00FFFFFF` draw, and what does `0x01000000`
  draw?

---

## M2 · Window and loop

**Hint 1.** `FixedStep` is a small state machine over one integer. Work the
SPEC's example by hand before writing it. Then open a window that shows a
framebuffer you fill yourself, long before there is a game.

**Hint 2.**

- Keep everything that names the window crate in one file of the binary,
  with a handful of functions: open, is it still open, which buttons are
  held, show this buffer. The rest of the binary should not know which
  crate is behind them.
- The buffer's pixel format was chosen so that it can be handed to the
  window crate without conversion. Check what your crate expects.
- Let the window crate limit the frame rate (most can), or your loop will
  use a whole processor core.
- Give the binary a way to quit by itself after a number of frames. You
  will want it for scripts, and it is the only way to smoke-test a window
  automatically.
- Two tests in `m2_loop.rs` need the finished simulation. Leave them red
  until M4.

**Check your explanation.**

- After `advance` returns the cap, why must the stored time be zero and
  not "what was left"?
- If the display runs at 30 frames per second, how many steps does each
  frame run, and what does the player see compared with 60?
- Where in your loop is real time read, and why nowhere else?

---

## M3 · The world

**Hint 1.** Parse the level into two kinds of data: the grid of tiles,
which never changes, and the lists of things that start somewhere. Decide
how entities are stored before you write any behaviour; CONCEPTS section 6
describes the options.

**Hint 2.**

- The map answers questions ("what is at column c, row r?", "is anything
  solid in this column between these two heights?"). Put the
  outside-the-map rule in exactly one function and build the other
  questions on it.
- The cast of a level is fixed when it loads: nothing spawns later. Think
  about what that means for the simplest store that works, and then about
  what "a removed enemy keeps its number" asks of it.
- Whatever store you choose, spawn order must survive removals. A list of
  handles in spawn order next to the store is one way; tombstones are
  another.
- The camera is a pure function of the player's position and the map
  size. Write it as one; the renderer will want to call it with a position
  that is not the current one.
- Error order matters in the level tests. Follow the list in SPEC 8.2
  literally.

**Check your explanation.**

- Why is an index into a `Vec` easier to live with than a reference to the
  element, when both say "that enemy"?
- What goes wrong with plain indices when slots are reused, and how does a
  generation counter prevent it?
- Why does `below(0)` still have to consume a draw?

---

## M4 · Physics and gameplay

**Hint 1.** Implement SPEC section 10 as a sequence of small phases in
exactly the given order, one function per phase if you like. Get the player
standing, then running, then jumping, then colliding, and run the tests
after each. Do not start with the enemies.

**Hint 2.**

- Moving a box along one axis and stopping it at solid tiles is needed for
  the player (both axes) and for walkers. Write it once, for any box. The
  leading edge of a half-open box is at `position + size - 1`.
- "On the ground" is not remembered between steps by itself: every step
  clears it and the downward collision sets it again. The first step of a
  game therefore lands.
- The jump has three small counters. Write the five numbered rules of
  SPEC 10.3 as five statements in that order and resist merging them.
- `step` needs the map (read), the player (write), the enemies (write),
  the score (write) and the effects (write) at once. If the borrow checker
  objects, look at CONCEPTS section 8: borrow fields, not `self`.
- Losing a life happens in two places with different follow-ups. Find the
  part they share.
- For the state hash, write the fields out one by one in a fixed byte
  order. The worked example in SPEC 11 gives you 52 bytes to compare with.
- When a golden trace fails at tick N, find the event near N in the trace
  (the lines where score or lives change) and reproduce that situation in
  a tiny level.

**Check your explanation.**

- Why is horizontal movement resolved before vertical movement, and what
  would a player notice if it were the other way round?
- After a full jump the feet are level with the floor one step before
  `LANDED`. Why?
- Why is `falling` decided once before looking at any enemy?
- Why does the hash not need the coyote counter?

---

## M5 · Animation and feel

**Hint 1.** Everything in this milestone reads the simulation and never
writes it. Build the renderer back to front: background, tiles, things,
effects, HUD. Look at a screenshot after each layer.

**Hint 2.**

- Particles: a fixed array and a count of live items is all the pool
  needs. Think about what removing an item from the middle should cost,
  and whether the order of particles matters.
- Particles age even when the game is over, and they use their own random
  stream. Decide where that stream is seeded.
- The shake is a number counted down by the simulation and a formula. The
  renderer adds the offset to the camera for the world and ignores it for
  the HUD.
- Interpolation needs the position each moving thing had at the start of
  the step. Store it when the step begins, in every step: also in a step
  of a finished game, in which nothing moves. A respawn must overwrite it.
- The HUD text changes every frame and must not allocate. `format!` is
  out; CONCEPTS section 10 shows the alternative.
- Looks: choose a small set of colours and stick to it. Light things on a
  dark background read well. Variation generated from the tile position
  (a tiny hash) makes repeated tiles look less repeated, and stays
  deterministic.
- Build sprites and tables once. If you put them in a global, think about
  what initialises it and whether that can allocate inside a frame.

**Check your explanation.**

- Why must `render` set every pixel, and which test would catch a
  renderer that only draws what changed?
- With `alpha = 0`, which state is on the screen? Why is that acceptable?
- Which of your effects would change the state hash if you moved it into
  the simulation's random stream?

---

## M6 · Replay and performance

**Hint 1.** A replay is data, and `to_text`/`from_text` are a printer and a
parser for a tiny line format. The headless mode is the windowed loop with
the window and the clock removed. If your binary was structured well in
M2, both are short.

**Hint 2.**

- Runs of equal inputs: the standard library can split a slice into groups
  of equal neighbours.
- The parser must reject a huge claimed length *before* it allocates.
  Validate and count first, build second.
- Number parsing: the standard `parse` accepts a little more than the
  format allows. Check what, and close the gap.
- In headless mode, decide where output goes once (a file or standard
  output) and write through one buffered writer. Errors about bad input
  must be reported before the first line is printed.
- The allocation test tells you the first tick that allocated. Use it like
  a debugger: what happened in that tick? Typical finds are text
  formatting, a `Vec` that grows on the first stomp, a `collect` in the
  renderer, and lazy initialisation that runs late.
- Compare a debug and a release headless run early. A difference there is
  almost always an overflow that wraps in release.

**Check your explanation.**

- Why does the replay store the buttons held in each step and not key
  presses with times?
- A replay made yesterday fails today. Name three different causes and how
  you would tell them apart.
- Why is a state hash that matches while the frame checksum differs good
  news?

---

## Stretch goals

- **Audio.** The `Events` returned by `step` are your sound triggers. The
  audio callback runs on another thread: it must not allocate or lock for
  long. Send events through a channel or a small lock-free queue and
  generate the waves there.
- **GPU renderer.** Start by showing the software framebuffer as a
  texture. Keep the software renderer as the reference and compare
  pictures with a small tolerance.
- **Rewind.** If your `Game` is plain data, a snapshot is a clone. Where
  would the clone allocate, and how could a ring of preallocated snapshots
  avoid that?
- **WebAssembly.** The library needs nothing from the operating system.
  The work is a second platform layer and a timer that calls you once per
  animation frame.
