# Concepts for the game capstone

Read this before you start, and come back to a section when you reach the
milestone that needs it.

**What you will be able to do afterwards.** Explain how a picture gets onto
the screen without a graphics library; write a rasteriser that clips
correctly; run a simulation at a fixed rate while the display runs at any
rate; make a real-time program deterministic and prove it with replays;
store game entities in a way the borrow checker accepts without a fight;
and keep a frame free of heap allocations.

**What is assumed.** Chapters 03 to 11 and 15 to 18 of the course: integers
and slices, ownership and borrowing, structs and enums, traits, iterators,
collections, text and files. No knowledge of graphics or games is assumed.

**Time (estimates).** About 15 to 25 minutes per section; three to four
hours for the whole file if you type in and change the examples, which you
should.

| section | read it before |
|---|---|
| 1. Framebuffers and pixel formats | M1 |
| 2. Rasterisation and clipping | M1 |
| 3. Fixed-point numbers | M3 |
| 4. The game loop and the fixed timestep | M2 |
| 5. Why determinism matters | M3 |
| 6. Entity storage and ownership | M3 |
| 7. Collision | M4 |
| 8. Simulation and presentation | M4 |
| 9. Replays as a testing tool | M6 |
| 10. Frames that do not allocate | M5 |
| 11. What a GPU pipeline changes | after M6 |

The examples here are deliberately different from the things you build.
They show the idea in a smaller or neighbouring setting; carrying it over
is your work.

---

## 1. Framebuffers and pixel formats

A screen shows a grid of coloured dots, the **pixels**. A **framebuffer** is
that grid in memory: one number per pixel, stored in one long array. A
program that owns a framebuffer can draw anything, because drawing is only
"write these numbers into these places".

The grid has a width and a height, but memory is one-dimensional. The usual
layout is **row-major**: the first row from left to right, then the second
row, and so on. The pixel in column `x` and row `y` is at index
`y * width + x`. Row 0 is the top row and `y` grows downward. That is the
opposite of a graph in a mathematics book, and it comes from how screens
were scanned.

```text
 width = 4, height = 3

 (0,0) (1,0) (2,0) (3,0)        index  0  1  2  3
 (0,1) (1,1) (2,1) (3,1)   -->         4  5  6  7
 (0,2) (1,2) (2,2) (3,2)               8  9 10 11
```

A **pixel format** says what the number for one pixel means. A simple
format uses one byte for brightness. The example below draws a diagonal
into such a buffer and prints it:

```rust
struct Gray {
    width: usize,
    height: usize,
    cells: Vec<u8>, // one byte per pixel: 0 is black, 255 is white
}

impl Gray {
    fn new(width: usize, height: usize) -> Gray {
        Gray { width, height, cells: vec![0; width * height] }
    }

    fn put(&mut self, x: usize, y: usize, value: u8) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = value;
        }
    }
}

let mut g = Gray::new(6, 3);
for i in 0..3 {
    g.put(2 * i, i, 255);
}
let rows: Vec<String> = g
    .cells
    .chunks(g.width)
    .map(|row| row.iter().map(|&v| if v > 127 { '#' } else { '.' }).collect())
    .collect();
assert_eq!(rows, ["#.....", "..#...", "....#."]);
```

`chunks(width)` gives the rows back. You will use that often, for example
to print a failing picture in a test.

Colour needs three numbers per pixel: how much red, green and blue light.
With 8 bits for each, a pixel fits in a `u32` with one byte to spare. That
spare byte is often used for **alpha**, a measure of how opaque the pixel
is. The order of the four bytes inside the `u32` is a convention, and there
are several. This project uses alpha in the top byte, then red, green,
blue. Shifts and masks take a pixel apart and put it together:

```rust
fn pack(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn red(pixel: u32) -> u8 {
    (pixel >> 16) as u8
}

let orange = pack(0xEE, 0x7B, 0x2E);
assert_eq!(orange, 0xFFEE_7B2E);
assert_eq!(red(orange), 0xEE);
// In memory, on a little-endian machine, the bytes are blue first.
assert_eq!(orange.to_le_bytes(), [0x2E, 0x7B, 0xEE, 0xFF]);
```

The last line matters when a pixel leaves your program. A file format or a
checksum works on bytes, and "the bytes of a `u32`" depends on the machine
unless you choose an order. Always choose: `to_le_bytes` or `to_be_bytes`,
never a raw view of the memory.

**Mistakes people make.**

- Swapping `x` and `y`, or using `x * width + y`. The picture comes out
  transposed or sheared. Test with a non-square buffer and a shape that is
  not symmetric.
- Writing with an `x` that is too large. Index `y * width + x` with
  `x == width` is a valid index: it is the first pixel of the next row. The
  write succeeds and the pixel appears on the wrong side of the screen.
  Bounds checks on the array do not catch this; you must check `x` and `y`
  yourself.
- Forgetting the alpha byte. A buffer of `0x00RRGGBB` values looks right in
  one window crate and fully transparent in another.

Read more: [`slice::chunks`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks),
[`u32::to_le_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.to_le_bytes).

## 2. Rasterisation and clipping

**Rasterisation** means turning a shape (a rectangle, a line, a letter)
into the set of pixels that represents it. For a test, "roughly the right
pixels" is useless: a rasteriser needs a rule that says exactly which
pixels belong to the shape. Good rules are stated with integers and
half-open ranges. "The pixels with `x0 <= x < x1`" has no doubt about the
edges and makes neighbouring shapes fit together without a gap or a double
pixel.

**Clipping** means drawing only the part of a shape that is inside the
buffer. There are two ways to do it:

1. Test every pixel before writing it. Simple and always correct, but it
   does work for pixels that cannot be visible.
2. Cut the shape down to the visible part first, then write without tests.
   Faster, and the place where most rasteriser bugs live.

Way 2 is an **interval intersection**. A horizontal run of pixels starting
at `x` with length `len` covers `x .. x + len`. The buffer covers
`0 .. width`. The visible part is from the larger start to the smaller end:

```rust
/// The visible part of the run `x .. x + len` in a row of `width` pixels.
fn clip_run(x: i32, len: i32, width: usize) -> Option<std::ops::Range<usize>> {
    // i64 holds the sum of any two i32 values, so nothing can overflow.
    let start = (x as i64).max(0);
    let end = (x as i64 + len as i64).min(width as i64);
    (start < end).then(|| start as usize..end as usize)
}

assert_eq!(clip_run(2, 3, 10), Some(2..5)); // fully inside
assert_eq!(clip_run(-4, 6, 10), Some(0..2)); // cut on the left
assert_eq!(clip_run(8, 100, 10), Some(8..10)); // cut on the right
assert_eq!(clip_run(12, 5, 10), None); // outside
assert_eq!(clip_run(3, 0, 10), None); // empty
assert_eq!(clip_run(5, i32::MAX, 10), Some(5..10)); // would overflow in i32
```

With a clipped run you can fill a row with one `fill` call on a sub-slice,
which is much faster than a loop of single writes. A filled circle shows
the pattern. For each row, integer arithmetic finds how wide the circle is
there, and the run is clipped and filled:

```rust
# fn clip_run(x: i32, len: i32, width: usize) -> Option<std::ops::Range<usize>> {
#     let start = (x as i64).max(0);
#     let end = (x as i64 + len as i64).min(width as i64);
#     (start < end).then(|| start as usize..end as usize)
# }
fn disc(cells: &mut [u8], width: usize, cx: i32, cy: i32, radius: i32) {
    let height = cells.len() / width;
    for dy in -radius..=radius {
        let y = cy + dy;
        if y < 0 || y as usize >= height {
            continue; // the whole row is outside
        }
        // The largest dx with dx*dx + dy*dy <= radius*radius.
        let mut dx = 0;
        while (dx + 1) * (dx + 1) + dy * dy <= radius * radius {
            dx += 1;
        }
        if let Some(run) = clip_run(cx - dx, 2 * dx + 1, width) {
            let row = y as usize * width;
            cells[row + run.start..row + run.end].fill(b'#');
        }
    }
}

let mut cells = vec![b'.'; 8 * 5];
disc(&mut cells, 8, 1, 2, 2); // centre near the left edge: partly outside
let rows: Vec<&str> = cells.chunks(8).map(|r| std::str::from_utf8(r).unwrap()).collect();
assert_eq!(rows, [".#......", "###.....", "####....", "###.....", ".#......"]);
```

Lines are different: a line is thin, so the question is which single pixel
to pick in each column or row. The classic answer is an **error
accumulator**: walk along the longer direction one pixel at a time and keep
an integer that says how far the true line has drifted from the pixels
chosen so far; when it passes a threshold, step in the other direction too.
The SPEC gives one such algorithm completely. Follow it literally: there
are many variants, they differ in single pixels, and a checksum notices.

Sprites are clipped like rectangles, with one more idea: when the left part
of a sprite is cut off, you must start reading the *source* further in by
the same amount. Compute the visible rectangle first, then derive both the
destination and the source coordinates from it.

**Mistakes people make.**

- Overflow in the clip arithmetic. `x + w` in `i32` panics in a debug build
  and wraps in a release build:

  ```text
  thread 'main' (123065958) panicked at b.rs:4:17:
  attempt to add with overflow
  ```

  (The number in parentheses is a thread id; yours will differ.) The same
  program built with `-O` printed `-1794967296` and went on. Widen to `i64`
  before adding, as `clip_run` does, or use `saturating_add`.
- Clipping by moving the shape instead of cutting it. If a line's end point
  is outside and you move the end point to the edge, the line changes its
  slope and the visible pixels shift. Clip pixels, not end points, unless
  you know an exact clipping algorithm.
- Off-by-one at the far edge: `<=` where `<` belongs gives a rectangle that
  is one pixel too wide, or a write one past the row.

**When to use which.** Per-pixel tests for thin things (lines, single
points, text glyphs), interval clipping for filled areas and sprites.

Read more: [`i32::saturating_add`](https://doc.rust-lang.org/std/primitive.i32.html#method.saturating_add),
[`slice::fill`](https://doc.rust-lang.org/std/primitive.slice.html#method.fill),
[`bool::then`](https://doc.rust-lang.org/std/primitive.bool.html#method.then).

## 3. Fixed-point numbers

A character that moves 2.5 pixels per frame needs positions finer than
whole pixels. The first idea is `f32`. This project uses integers instead,
and the technique is called **fixed point**: choose a unit smaller than a
pixel and count in that unit. With 256 **subpixels** per pixel, 2.5 pixels
is the integer 640, and one tile of 16 pixels is 4096.

```text
 value in subpixels:    640
 in binary:             0000 0010 . 1000 0000
                        whole pixels   fraction (in 256ths)
                              2      .    128/256 = 0.5
```

Addition and subtraction of fixed-point values are plain integer
operations. Converting to whole pixels is a shift. A product of two
fixed-point values has the scale twice and must be shifted back once:

```rust
const ONE: i32 = 256; // subpixels per pixel

fn to_pixels(v: i32) -> i32 {
    v >> 8 // rounds toward negative infinity
}

fn mul(a: i32, b: i32) -> i32 {
    // Widen first: the intermediate product needs more than 32 bits.
    ((a as i64 * b as i64) >> 8) as i32
}

let speed = 2 * ONE + ONE / 2; // 2.5
assert_eq!(speed, 640);
assert_eq!(to_pixels(speed), 2);
assert_eq!(mul(speed, 3 * ONE), 7 * ONE + ONE / 2); // 2.5 * 3 = 7.5

// A ball thrown upward: 40 steps of "add gravity, then move".
let (mut y, mut vy) = (0, -5 * ONE);
let mut highest = 0;
for _ in 0..40 {
    vy += 64; // gravity: a quarter pixel per step, per step
    y += vy;
    highest = highest.min(y);
}
assert_eq!(highest, -12_160); // exactly 47.5 pixels up, on every machine
assert_eq!(y, 1_280);
```

Why not floats? The basic operations on `f32` and `f64` are well defined
and give the same result on the common platforms. But the standard library
says of functions such as `sin` and `powf`: "The precision of this function
is non-deterministic. This means it varies by platform, Rust version, and
can even differ within the same execution from one invocation to the next."
A specification that says "the jump is exactly this high" would also have
to say how every intermediate value is rounded. With integers the rounding
is visible in the code, the specification can give exact numbers, and a
test can compare with `==`.

The price is that you must think about **range**. An `i32` with 8 fraction
bits holds about plus or minus 8 million pixels, which is plenty for a
map; but a product of two such values overflows quickly, which is why `mul`
widens to `i64`.

**Mistakes people make.**

- Rounding negative numbers with `/`. Integer division rounds toward zero,
  a shift rounds down, and they differ below zero:

  ```rust
  assert_eq!(-1 / 256, 0);
  assert_eq!(-1 >> 8, -1);
  assert_eq!((-1_i32).div_euclid(256), -1);
  assert_eq!((-300_i32).rem_euclid(256), 212);
  ```

  A tile index computed with `/` is wrong for every position left of zero:
  both `-1 / 4096` and `1 / 4096` are tile 0. Use `>>` or `div_euclid` for
  positions.
- Mixing units. Adding a value in pixels to a value in subpixels compiles
  and is wrong by a factor of 256. Name your constants with their unit, or
  wrap the two kinds in different types.
- Shifting before multiplying to "avoid overflow", which throws away the
  fraction bits first.

Read more: [`i32::div_euclid`](https://doc.rust-lang.org/std/primitive.i32.html#method.div_euclid),
the note on precision in [`f32::sin`](https://doc.rust-lang.org/std/primitive.f32.html#method.sin).

## 4. The game loop and the fixed timestep

A game is a loop: read input, advance the world a little, draw, repeat. The
question is what "a little" means.

**Variable timestep.** Measure how long the last frame took, call it `dt`,
and advance the world by `dt`. This looks natural and has a hidden flaw:
the result depends on how time was cut into frames. A falling body shows
it. Advance by "add `g * dt` to the speed, then add `speed * dt` to the
position", once with one step of length 2 and once with two steps of
length 1:

```rust
fn fall(steps: &[i32]) -> (i32, i32) {
    let g = 10;
    let (mut y, mut v) = (0, 0);
    for &dt in steps {
        v += g * dt;
        y += v * dt;
    }
    (y, v)
}

let one_long_frame = fall(&[2]);
let two_short_frames = fall(&[1, 1]);
assert_eq!(one_long_frame, (40, 20));
assert_eq!(two_short_frames, (30, 20));
```

The same two units of time, the same final speed, but a different
position. On a fast machine a jump is a different height than on a slow
one. Collisions make it worse: with a long `dt` a fast object can pass
through a thin wall without ever overlapping it. Nothing about this can be
tested, because no two runs are alike.

**Fixed timestep.** Advance the world only in steps of one fixed length,
for example 1/60 second. The loop measures real time and decides *how
many* steps to run, never *how long* a step is:

```text
accumulator = 0
loop:
    accumulator += time since the last turn of the loop
    while accumulator >= STEP:
        world.step(input)        # always the same amount of game time
        accumulator -= STEP
    draw(world)
```

The accumulator is a debt of time that the simulation still owes. On a
60 Hz display each turn pays one step. On a 144 Hz display most turns pay
nothing and some pay one. On a slow machine a turn pays two or three. In
all cases the world sees the same sequence of identical steps.

Two details complete the pattern.

*The left-over.* After the inner loop the accumulator holds a fraction of a
step. The picture on the screen is therefore slightly older than "now". On
a display that refreshes faster than the simulation steps, motion would
look uneven. The cure is **interpolation**: keep each moving thing's
previous position, and draw it at `previous + (current - previous) *
fraction`. The simulation is untouched; only the drawing uses the fraction.

*The spiral of death.* If one step takes longer to compute than `STEP`,
each turn adds more debt than it pays, the inner loop runs longer and
longer, and the program freezes. A cap on the number of steps per turn,
together with dropping the unpaid debt, turns a freeze into slow motion.

**Mistakes people make.**

- Sampling input inside the inner loop from the live keyboard. The window
  only updates its key state once per turn, so nothing is gained, and the
  code suggests a precision that is not there. Sample once per turn and
  pass the same input to every step of that turn.
- Using the interpolation fraction inside the simulation ("just this
  once"). From that moment the simulation depends on the frame rate again.
- Saving the previous position only in the code that moves a thing. When
  that code stops running (the game is over, or paused), "previous" and
  "current" stay different for ever, and the picture slides back and forth
  between them as the fraction changes from frame to frame. Save "previous"
  for everything at the start of every step, whether or not it will move.
- Interpolating across a teleport. A thing that is put somewhere else in
  one step did not travel the distance; drawn at a fraction of the way, it
  flashes through the middle of the screen. When you teleport a thing, set
  its previous position to the new place as well.
- Measuring time with a wall clock that can jump. Use
  [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html),
  which never goes backwards.
- Keeping the accumulator in floating-point seconds. It works, but integer
  microseconds can be tested exactly.

**When to use which.** A fixed step for anything that must be reproducible
or stable: physics, gameplay, networking. A variable step is acceptable for
things that are pure decoration and do not feed back into the game.

## 5. Why determinism matters

A program is **deterministic** when the same inputs always produce the same
outputs. For this game the inputs are the level, the seed, and the buttons
held in each step. The outputs are everything else: every position, the
score, every pixel.

Determinism is not a goal in itself. It is what makes the following
possible:

- **Tests.** A test can run 1000 steps with scripted input and compare the
  result with one stored number.
- **Replays.** Recording the inputs records the whole game (section 9).
- **Debugging.** A bug that happened once can be made to happen again, as
  often as needed, with a debugger attached.
- **Portability claims.** "Works the same on Linux, macOS and Windows"
  becomes something you can check by comparing two text files.

What breaks determinism, and what to do instead:

| source | instead |
|---|---|
| reading the clock inside the simulation | pass time in as whole steps (section 4) |
| `HashMap` and `HashSet` iteration order (random per process) | `Vec`, `BTreeMap`, or sort before iterating |
| random numbers from the operating system | a small generator with an explicit seed |
| threads racing to update shared state | one thread for the simulation, or a fixed merge order |
| floating-point library functions | integers, or only the basic operations |
| `usize` in stored or hashed data (32 or 64 bits) | fixed-width types such as `u32` |
| overflow (panics in debug, wraps in release) | stay in range; where wrapping is intended, say `wrapping_add` |
| order of memory addresses | never sort or hash by pointer |

A seeded generator is a few lines. This one (a linear congruential
generator, not the one the SPEC asks for) also shows explicit wrapping:

```rust
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        // wrapping_* states the intent: debug and release agree.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
}

let mut a = Lcg(42);
let mut b = Lcg(42);
let first: Vec<u32> = (0..3).map(|_| a.next()).collect();
let again: Vec<u32> = (0..3).map(|_| b.next()).collect();
assert_eq!(first, again); // same seed, same stream, always
let mut c = Lcg(43);
assert_ne!(first[0], c.next()); // another seed, another stream
```

A subtle point: *the order of draws is part of the behaviour*. If a
particle effect takes a random number from the same generator as the
enemies, then adding one particle changes where every enemy goes. Give
decoration its own generator.

**Mistakes people make.**

- Believing a program is deterministic because it looked the same twice.
  Compare checksums of the full state at every step, between a debug and a
  release build, and across machines if you can.
- Hashing a struct's raw bytes. Padding bytes and field order are not
  yours to rely on. Write the fields out one by one in a fixed byte order.
- Iterating a `HashMap` "just to draw". If drawing order affects pixels
  (overlapping sprites), the frame checksum changes from run to run.

Read more: [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
(see the paragraph on its hashing and iteration order),
[`u64::wrapping_mul`](https://doc.rust-lang.org/std/primitive.u64.html#method.wrapping_mul).

## 6. Entity storage and ownership

A game world is many small things that refer to each other: this enemy
chases that player, this bullet belongs to that ship. In many languages
each thing holds pointers to the others. In Rust that design runs straight
into the ownership rules, and understanding why saves a week.

**References do not work.** Suppose the things live in a `Vec` and an
update needs two of them at once:

```rust,compile_fail,E0499
struct Ball {
    x: i32,
    vx: i32,
}

fn bounce(a: &mut Ball, b: &mut Ball) {
    std::mem::swap(&mut a.vx, &mut b.vx);
}

fn main() {
    let mut balls = vec![Ball { x: 0, vx: 2 }, Ball { x: 9, vx: -1 }];
    let first = &mut balls[0];
    let second = &mut balls[1];
    bounce(first, second);
    println!("{} {}", balls[0].x, balls[1].x);
}
```

```text
error[E0499]: cannot borrow `balls` as mutable more than once at a time
  --> a.rs:13:23
   |
12 |     let first = &mut balls[0];
   |                      ----- first mutable borrow occurs here
13 |     let second = &mut balls[1];
   |                       ^^^^^ second mutable borrow occurs here
14 |     bounce(first, second);
   |            ----- first borrow later used here
   |
   = help: use `.split_at_mut(position)` to obtain two mutable non-overlapping sub-slices
```

The compiler cannot see that index 0 and index 1 are different elements.
Storing a reference *inside* one ball that points at another is worse: the
`Vec` could not be changed at all while such a reference exists.

**`Rc<RefCell<..>>` moves the problem to run time.** It compiles, and then
an update that reaches the same object by two paths panics:

```rust,should_panic
use std::cell::RefCell;
use std::rc::Rc;

struct Monster {
    hp: i32,
    target: Option<Rc<RefCell<Monster>>>,
}

let a = Rc::new(RefCell::new(Monster { hp: 10, target: None }));
a.borrow_mut().target = Some(Rc::clone(&a)); // it targets itself
// The update: every monster hits its target.
let mut me = a.borrow_mut();
let target = me.target.clone().unwrap();
target.borrow_mut().hp -= 1; // second mutable borrow of the same cell
me.hp += 0;
```

```text
thread 'main' (123066100) panicked at c.rs:15:12:
RefCell already borrowed
```

It also leaks (the monster above keeps itself alive for ever), costs a
reference count on every access, and scatters the monsters over the heap.

**Indices work.** Keep the things in a `Vec` and let them refer to each
other by position. An index is a plain number: it is `Copy`, it borrows
nothing, and it can be stored anywhere. You borrow the `Vec` only for the
moment you look something up. For two elements at once the standard
library has `get_disjoint_mut`:

```rust
struct Ball {
    x: i32,
    vx: i32,
}

let mut balls = vec![Ball { x: 0, vx: 2 }, Ball { x: 9, vx: -1 }];
if let Ok([a, b]) = balls.get_disjoint_mut([0, 1]) {
    std::mem::swap(&mut a.vx, &mut b.vx);
}
assert_eq!((balls[0].vx, balls[1].vx), (-1, 2));
// Asking for the same element twice is an error, not undefined behaviour.
assert!(balls.get_disjoint_mut([1, 1]).is_err());
assert_eq!(balls[0].x, 0);
```

Often you do not even need two mutable borrows: copy the few numbers you
need out of one element (they are `Copy`), release the borrow, then change
the other.

**The weakness of a plain index** is removal. If element 3 is removed and
its slot is reused for something new, an old "3" stored elsewhere now
points at the wrong thing, silently. A **generational index** fixes that:
each slot carries a counter, the *generation*, that goes up every time the
slot is emptied; a handle is the pair (index, generation); a lookup
succeeds only if the generations match.

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
struct Handle {
    index: usize,
    generation: u32,
}

struct Slots<T> {
    slots: Vec<(u32, Option<T>)>, // (generation, value)
}

impl<T> Slots<T> {
    fn insert(&mut self, value: T) -> Handle {
        // Reuse the first empty slot, or add one. (A real store keeps a
        // free list instead of searching.)
        let index = match self.slots.iter().position(|(_, v)| v.is_none()) {
            Some(index) => index,
            None => {
                self.slots.push((0, None));
                self.slots.len() - 1
            }
        };
        self.slots[index].1 = Some(value);
        Handle { index, generation: self.slots[index].0 }
    }

    fn remove(&mut self, h: Handle) -> Option<T> {
        let slot = self.slots.get_mut(h.index)?;
        if slot.0 != h.generation {
            return None;
        }
        slot.0 += 1; // every handle issued so far is now stale
        slot.1.take()
    }

    fn get(&self, h: Handle) -> Option<&T> {
        let slot = self.slots.get(h.index)?;
        if slot.0 == h.generation { slot.1.as_ref() } else { None }
    }
}

let mut names = Slots { slots: Vec::new() };
let old = names.insert("moth");
names.remove(old);
let new = names.insert("drop"); // takes the same slot
assert_eq!(new.index, old.index);
assert_eq!(names.get(new), Some(&"drop"));
assert_eq!(names.get(old), None); // the stale handle is refused
```

**Struct of arrays.** A different arrangement keeps one `Vec` per field
instead of one `Vec` of structs: all x positions together, all speeds
together. An entity is then a column across the arrays. A loop that needs
only positions touches only that memory, and separate fields can be
borrowed separately:

```rust
struct Movers {
    x: Vec<i32>,
    vx: Vec<i32>,
    alive: Vec<bool>,
}

let mut m = Movers { x: vec![0, 10, 20], vx: vec![1, -2, 3], alive: vec![true, false, true] };
// Two fields borrowed at once, one mutably: no conflict.
for ((x, vx), alive) in m.x.iter_mut().zip(&m.vx).zip(&m.alive) {
    if *alive {
        *x += *vx;
    }
}
assert_eq!(m.x, [1, 10, 23]);
```

**When to use which.**

- A fixed cast that never changes: a `Vec` and plain indices are enough.
- Things created and destroyed during play, referred to from elsewhere: a
  generational store.
- Thousands of similar things updated in tight loops: struct of arrays.
- `Rc<RefCell<..>>`: for genuinely shared ownership with rare mutation.
  A game world updated 60 times a second is not that.

Read more: [`slice::get_disjoint_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut),
[`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut),
[error E0499](https://doc.rust-lang.org/error_codes/E0499.html).

## 7. Collision

Almost every 2D game treats its objects as **axis-aligned boxes**: a
rectangle with a position and a size, never rotated. Two boxes overlap when
they overlap on both axes, and overlap on one axis is an interval test:

```rust
#[derive(Clone, Copy)]
struct Span {
    start: i32,
    len: i32,
}

fn overlaps(a: Span, b: Span) -> bool {
    a.start < b.start + b.len && b.start < a.start + a.len
}

let floor = Span { start: 0, len: 10 };
assert!(overlaps(floor, Span { start: 9, len: 5 }));
// Half-open ranges: 0..10 and 10..15 share an edge but no point.
assert!(!overlaps(floor, Span { start: 10, len: 5 }));
```

Decide once whether touching counts as overlapping, and write it down. With
half-open ranges it does not, and an object resting exactly on the floor
does not overlap the floor. That has a consequence you will meet: to know
that something stands on the ground you must move it a little *into* the
ground and see that it was pushed back.

**Against a grid of tiles** there is no need to test all tiles. A box can
only touch the tiles under it, and those are found by division: the tile
column of a coordinate is the coordinate divided by the tile size (rounded
down; see section 3).

**Detect, then resolve.** Finding an overlap is half of the work. The other
half is deciding where the object goes instead. The robust method for tile
worlds is: move along one axis, fix that axis; then move along the other
axis, fix that one. "Fix" means: put the box flush against the tile it ran
into and stop its speed on that axis.

A one-dimensional version, a block sliding toward a wall:

```rust
const TILE: i32 = 100;

/// Tiles 5 and up are solid. Moves a block of width `w` by `v`.
/// Returns the new position and the new speed.
fn slide(x: i32, w: i32, v: i32) -> (i32, i32) {
    let solid = |tile: i32| tile >= 5;
    let moved = x + v;
    let leading_tile = (moved + w - 1).div_euclid(TILE); // right edge, inclusive
    if v > 0 && solid(leading_tile) {
        (leading_tile * TILE - w, 0) // flush against the wall, stopped
    } else {
        (moved, v)
    }
}

assert_eq!(slide(400, 30, 40), (440, 40)); // free
assert_eq!(slide(440, 30, 40), (470, 0)); // right edge would be 509: snap to 500 - 30
assert_eq!(slide(470, 30, 40), (470, 0)); // pressing against the wall: stays
```

Note `moved + w - 1`: the right edge of a half-open box is the last
coordinate *inside* it. A block from 470 to 500 does not touch tile 5.

Why one axis at a time? Think of a box moving down and to the right into
the corner of a floor. If both movements are applied first, the box
overlaps the floor tile, and the code must guess whether to push it up or
to the left; the wrong guess makes characters stick to walls or pop
through floors. With one axis at a time, each test has only one direction
to push back in. The order of the axes changes what happens at corners, so
a specification has to fix it.

**Tunnelling.** This method looks only at the destination. If an object
moves further in one step than the thickness of a wall, it can jump over
the wall. The cheap cure is a speed limit below the tile size, which the
fixed timestep makes meaningful: "at most 6 pixels per step" is a real
guarantee only when a step is always the same length.

**Mistakes people make.**

- Testing the leading edge with `x + w` instead of `x + w - 1`. The object
  then stops one unit early, or collides with tiles it only touches.
- Resolving both axes from the original position and adding the results.
  It works in open space and fails in corners.
- Checking "is the tile below solid" with the position from before the
  move. The answer is one step old.
- Using a contact that happened to set a flag ("on ground") and never
  clearing the flag. Clear it every step and let the collision set it
  again.

**When to use which.** Boxes and one-axis-at-a-time resolution for
platformers and top-down games on a grid. Circles, swept tests and
separating-axis tests for rotation and for fast small objects; none of
that is needed here.

## 8. Simulation and presentation

A game has two jobs that look like one: deciding what happens, and showing
it. Keep them apart:

- The **simulation** owns the state and has one entry point that advances
  it by one step. It knows nothing about pixels, windows, sound or time.
- The **presentation** reads the state and produces a picture (and sound).
  It never changes the state.

In Rust the signatures carry that contract: `fn step(&mut self, input)` and
`fn render(&self, target)`. A renderer that takes `&self` *cannot* change
the game by accident; the compiler holds the line for you.

What the separation buys:

- The simulation can run without the presentation: in tests, in a headless
  tool, faster than real time, or many copies at once.
- The presentation can run at another rate, be skipped, or be replaced (a
  GPU renderer, a text renderer for debugging) without touching the rules.
- Bugs sort themselves: wrong position is simulation, wrong picture of a
  right position is presentation.

**How the simulation talks to the presentation.** Mostly it does not need
to: the renderer reads positions and draws. But some things are *moments*,
not states: a jump started, a coin was picked up. The simulation should not
call `play_sound`. It reports what happened and lets the caller decide:

```rust
#[derive(Default)]
struct Counter {
    value: u32,
}

#[derive(Debug, PartialEq)]
enum Happened {
    Nothing,
    WrappedAround,
}

impl Counter {
    /// The simulation: changes state, reports the moment, draws nothing.
    fn step(&mut self) -> Happened {
        self.value = (self.value + 1) % 3;
        if self.value == 0 { Happened::WrappedAround } else { Happened::Nothing }
    }

    /// The presentation: reads state, changes nothing.
    fn render(&self, out: &mut String) {
        out.clear();
        for i in 0..3 {
            out.push(if i == self.value { '#' } else { '.' });
        }
    }
}

let mut c = Counter::default();
let mut screen = String::new();
let mut beeps = 0;
for _ in 0..7 {
    if c.step() == Happened::WrappedAround {
        beeps += 1; // the caller reacts: sound, particles, a log line
    }
    c.render(&mut screen);
    c.render(&mut screen); // drawing twice changes nothing
}
assert_eq!(screen, ".#.");
assert_eq!(beeps, 2);
```

A set of events returned from `step` is also exactly what a test wants to
assert on.

**The borrow checker and big `step` functions.** A step usually touches
many parts of the world: read the map, change the player, change the
score. Written as methods on one big struct, that runs into this:

```rust,compile_fail,E0502
struct World {
    scores: Vec<u32>,
    names: Vec<String>,
}

impl World {
    fn longest_name(&self) -> usize {
        self.names.iter().map(|n| n.len()).max().unwrap_or(0)
    }

    fn add_bonus(&mut self) {
        for score in &mut self.scores {
            *score += self.longest_name() as u32;
        }
    }
}

fn main() {
    let mut w = World { scores: vec![1, 2], names: vec!["nib".to_string()] };
    w.add_bonus();
    println!("{:?}", w.scores);
}
```

```text
error[E0502]: cannot borrow `*self` as immutable because it is also borrowed as mutable
  --> d.rs:13:23
   |
12 |         for score in &mut self.scores {
   |                      ----------------
   |                      |
   |                      mutable borrow occurs here
   |                      mutable borrow later used here
13 |             *score += self.longest_name() as u32;
   |                       ^^^^ immutable borrow occurs here
```

`longest_name` takes all of `self`, although it reads only `names`. The
compiler checks signatures, not bodies. Three fixes, in order of
preference:

1. Compute first, then mutate: `let bonus = self.longest_name();` before
   the loop.
2. Borrow fields, not `self`: write the helper as a function that takes
   `&[String]` and call it with `&self.names`. Borrows of different fields
   do not conflict.
3. Split the struct so that things that are used together live together
   (a `Map` that is only read during a step, a `Player` that is changed).

```rust
fn longest(names: &[String]) -> usize {
    names.iter().map(|n| n.len()).max().unwrap_or(0)
}

struct World {
    scores: Vec<u32>,
    names: Vec<String>,
}

let mut w = World { scores: vec![1, 2], names: vec!["nib".to_string()] };
for score in &mut w.scores {
    *score += longest(&w.names) as u32; // two different fields: allowed
}
assert_eq!(w.scores, [4, 5]);
```

**Mistakes people make.**

- Letting the renderer advance animations or timers ("it is only
  cosmetic"). Then the picture depends on how often it was drawn.
- Putting the window handle or the framebuffer inside the game state.
  Then the game cannot exist without a window.
- State that is only needed for drawing but changes the rules when it is
  wrong. Decide for each field which side owns it.

Read more: [error E0502](https://doc.rust-lang.org/error_codes/E0502.html),
the section on splitting borrows in the
[Rustonomicon](https://doc.rust-lang.org/nomicon/borrow-splitting.html).

## 9. Replays as a testing tool

If the simulation is deterministic and its only inputs are the seed and
the buttons of each step, then a list of those inputs *is* the whole game.
Storing it is called recording a **replay**; feeding it back reproduces
every state and every frame. A minute of play is a few hundred bytes.

Players know replays as a way to show a great run to a friend. For a
programmer they are the best testing tool a real-time program can have:

- **An end-to-end test for free.** Play once, keep the replay and the
  final checksum. From then on a single assertion checks input handling,
  physics, collisions, enemies, scoring and winning together.
- **A regression alarm.** Any change to the rules changes some checksum.
  If the change was meant, record a new expected value. If not, you have
  found a bug before a player did.
- **Bisecting in time.** Store a checksum per step, not just one at the
  end. Then a failure says "the first difference is at step 4127", and you
  look at one step instead of at a minute of play.
- **Cross-build checks.** Debug against release, one operating system
  against another: run the same replay, compare the text files.
- **Bug reports that reproduce.** A replay attached to a report is the bug
  itself.

The whole idea in miniature, with a toy simulation:

```rust
/// A toy world: a position on a line with walls at 0 and 9.
#[derive(Clone, PartialEq, Debug)]
struct Toy {
    x: i32,
    bumps: u32,
}

impl Toy {
    fn step(&mut self, input: i8) {
        let next = self.x + input as i32;
        if (0..=9).contains(&next) {
            self.x = next;
        } else {
            self.bumps += 1;
        }
    }

    /// A checksum of the state: fields in a fixed order, fixed byte order.
    fn checksum(&self) -> u32 {
        let mut sum = 0u32;
        for byte in self.x.to_le_bytes().into_iter().chain(self.bumps.to_le_bytes()) {
            sum = sum.wrapping_mul(31).wrapping_add(byte as u32);
        }
        sum
    }
}

// "Live" play: inputs arrive one at a time and are recorded.
let live_inputs = [1, 1, 1, -1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1];
let mut live = Toy { x: 0, bumps: 0 };
let mut recorded = Vec::new();
let mut trace = Vec::new();
for input in live_inputs {
    live.step(input);
    recorded.push(input);
    trace.push(live.checksum());
}

// Later, or on another machine: replay and compare step by step.
let mut replayed = Toy { x: 0, bumps: 0 };
for (step, &input) in recorded.iter().enumerate() {
    replayed.step(input);
    assert_eq!(replayed.checksum(), trace[step], "first difference at step {step}");
}
assert_eq!(replayed, live);
assert_eq!(live.bumps, 2);
```

**State checksums and frame checksums are different tools.** A state
checksum covers the simulation. A frame checksum covers what is on the
screen, which includes the renderer and all decoration. Keep both: when
the state agrees and the frame does not, the bug is in drawing.

**What makes replays break**, apart from non-determinism: changing the
rules (old replays no longer fit, which is the regression alarm working),
changing the level, and forgetting an input (anything that influences the
simulation and is not in the replay: the seed, a pause key that skips
steps, the frame rate).

**Mistakes people make.**

- Recording key *events* with timestamps instead of the buttons held in
  each step. Timestamps bring real time back in.
- Recording after input smoothing or key repeat in the platform layer, and
  replaying before it (or the other way round). Record exactly what `step`
  receives.
- A replay file without a version line. The first format change then
  makes every old file fail in a confusing way.

## 10. Frames that do not allocate

A game at 60 frames per second has 16.7 milliseconds per frame. A single
allocation is usually fast, so the average cost is not the problem. The
problem is the *worst case*: now and then the allocator has to ask the
operating system for memory or tidy its lists, and one frame takes several
times longer than the others. Players see that as a stutter. The
professional habit is therefore: allocate at start-up, then never during a
frame.

The usual sources of hidden allocations, and what replaces them:

| allocates | does not allocate |
|---|---|
| `format!`, `to_string`, `String` building per frame | `write!` into a buffer you keep |
| `Vec::new()` + `push` per frame, `collect()` | a `Vec` kept between frames: `clear()` keeps its capacity |
| `Box::new` per spawned object | a pool: a fixed array plus a count of live items |
| growing a `Vec` beyond its capacity | `with_capacity` at start-up for the largest size |
| `sort` (stable sort may allocate) | `sort_unstable`, or keep things in order |

`clear` and capacity in action:

```rust
let mut scratch: Vec<u32> = Vec::with_capacity(64);
let before = scratch.as_ptr();
for frame in 0..1000 {
    scratch.clear(); // length 0, capacity unchanged
    scratch.extend((0..50).map(|i| i + frame));
}
assert_eq!(scratch.capacity(), 64);
assert_eq!(scratch.as_ptr(), before, "the same block of memory for 1000 frames");
```

Formatting a number for the screen without a `String`: `write!` works with
anything that implements `std::fmt::Write`, and that can be a small array
on the stack.

```rust
use std::fmt::Write;

/// Text in a fixed array. Writing more than fits is an error, not a panic.
struct Label {
    bytes: [u8; 16],
    len: usize,
}

impl Write for Label {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

let mut label = Label { bytes: [0; 16], len: 0 };
write!(label, "HP {:03}/{}", 7, 20).unwrap();
assert_eq!(std::str::from_utf8(&label.bytes[..label.len]).unwrap(), "HP 007/20");
assert!(write!(label, " and far too much text").is_err());
```

A **pool** for short-lived things such as particles is an array that is
never resized, plus the number of items in use. Items `0..live` are alive.
Spawning writes at index `live` and adds one. Removing item `i` copies the
last live item over it and subtracts one ("swap remove"): no gap, no
shifting, no allocation. The order of the items changes, which is fine
when order does not matter.

```rust
let mut pool = [0u8; 4]; // remaining lifetimes
let mut live = 0;

for lifetime in [3, 1, 2] {
    if live < pool.len() {
        pool[live] = lifetime;
        live += 1;
    } // a full pool drops the new item
}

// One update: age every item, remove the ones that reach zero.
let mut i = 0;
while i < live {
    pool[i] -= 1;
    if pool[i] == 0 {
        live -= 1;
        pool[i] = pool[live]; // the last live item fills the hole
        // do not advance: the item now at `i` has not been aged yet
    } else {
        i += 1;
    }
}
assert_eq!(&pool[..live], [2, 1]);
```

**How it is measured.** A program can replace the global allocator with
one that counts calls and forwards them to the system allocator. A test
turns counting on, runs frames, and asserts that the count is zero. This
project's `tests/common/mod.rs` contains such an allocator; read it once.

**Mistakes people make.**

- Removing from a pool inside a `for i in 0..live` loop and advancing `i`
  after a swap remove: every second dead item survives.
- A `Vec` that "never grows in practice". Reserve the proven maximum, or
  make the limit explicit and drop what does not fit.
- Trusting a release build to remove an allocation. Sometimes it does; a
  debug build does not, and the budget should hold in both.

Read more: [`Vec::clear`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.clear),
[`std::fmt::Write`](https://doc.rust-lang.org/std/fmt/trait.Write.html),
[`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html).

## 11. What a GPU pipeline changes

Your game computes every pixel on the processor. That is the simplest
model and, at 320 by 180, fast enough many times over. Modern games draw
with the graphics processor (GPU). It helps to know what stays and what
changes, because the design of this project is the one that survives the
move.

**What stays.** Everything outside the renderer: the simulation, the fixed
timestep, input, replays, the state checksum. A renderer that takes `&Game`
and produces a picture can be exchanged without the rest noticing.

**What changes in the renderer.**

- *You stop writing pixels and start describing triangles.* A sprite
  becomes a rectangle made of two triangles, with four corner positions and
  four texture coordinates. The GPU decides which pixels each triangle
  covers; rasterisation and clipping are done for you, in parallel, by
  hardware.
- *Pictures become textures.* Sprite pixels are uploaded once into GPU
  memory, usually many sprites packed into one large image (an **atlas**),
  and referred to by coordinates.
- *Per-pixel work moves into shaders.* A **vertex shader** places the
  corners; a **fragment shader** runs once per covered pixel and computes
  its colour. Your `blend_pixel` lights become a few lines of shader code
  that run for all pixels at once.
- *The unit of cost becomes the draw call.* Sending one batch of 10000
  triangles is cheap; sending 10000 batches of one triangle is slow.
  Renderers therefore collect sprites that share a texture into one vertex
  buffer per frame. Your allocation discipline carries over: that buffer is
  reused, not rebuilt.
- *The framebuffer is no longer yours.* It lives on the GPU. Reading it
  back is possible but slow, and different GPUs and drivers may round
  colours differently. A frame checksum that must match exactly across
  machines is realistic for a software renderer and usually not for a GPU
  one. Tests of a GPU renderer compare against the software picture with a
  small tolerance, or test the list of draw commands instead of the pixels.

**A gentle path.** The first GPU step for this project is tiny: keep the
software renderer, upload its framebuffer as one texture each frame, and
draw one screen-filling rectangle. Scaling to the window then happens on
the GPU and nothing else changes. After that, tiles and sprites can move
to textured rectangles one kind at a time, with the software renderer as
the reference picture for each step. In Rust the usual crate for this is
`wgpu`; SPEC section 18 lists it as a stretch goal.

**When to use which.** A software renderer for small resolutions, for
tools, for tests and for learning how pictures are made. A GPU for large
resolutions, many layers, rotation, scaling and effects per pixel. Many
real projects keep both: the software renderer as the oracle that says
what the picture should be.
