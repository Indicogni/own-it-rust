# Underkiln: specification

This is the complete specification of the game capstone. You write an
original, real-time, graphical 2D game: it opens a window, draws moving
sprites and reacts to the keyboard. You also make it testable, which is the
harder and more valuable half.

The words **must**, **must not** and **may** are used in their plain meaning.
Everything marked "exact" is checked by a test. Everything marked "free" is
yours to design.

Contents:

1. The game
2. Architecture and build rules
3. The framebuffer
4. The rasteriser
5. The font
6. The fixed timestep
7. The random number generator
8. The level and the world
9. Units and constants
10. One simulation step
11. The state hash
12. Presentation: animation, particles, shake, HUD, rendering
13. The replay format
14. The binary
15. The API facade
16. The performance budget
17. Milestones and their tests
18. Stretch goals
19. Non-goals
- Appendix A: font file format
- Appendix B: sprite sheet format
- Appendix C: the shipped art

---

## 1. The game

**Underkiln** is a platform game set in a cold cavern under an old kiln.

- You play **Nib**, a small kiln spirit made of fire.
- Scattered through the cavern are **sparks**. Nib gathers them by touching
  them.
- When every spark of a level has been gathered, the **flue** (a stone
  chimney, the exit) lights up. Touching the lit flue wins the level.
- **Dousers** are creatures of water that walk along the ground. **Mistwings**
  are creatures of mist that drift through the air. Touching one from the
  side or from below costs Nib one of its three **lives**. Landing on one from
  above defeats it.
- **Water** and **falling out of the cavern** cost a life and send Nib back to
  where the level started.
- When the last life is gone, the game is lost.

The player has three buttons: **left**, **right** and **jump**.

Score: 10 for a spark, 50 for a Douser, 80 for a Mistwing and, on winning,
100 for every life left.

The names, the rules, the level and the art in this capstone are original.
Your implementation must not copy names, characters, levels, pictures or
sounds from any existing game.

## 2. Architecture and build rules

The architecture is part of the specification, because it is what makes a
graphical game testable.

```text
            +--------------------- library crate `game` ---------------------+
 keys ----> |  Input --> Game::step --> (state) --> Game::render --> pixels  | ---> window
            |             simulation                  renderer   Framebuffer |
            +----------------------------------------------------------------+
   platform layer (binary, feature `window`)          platform layer (binary)
```

1. **Software rendering.** All drawing goes into a `Framebuffer`: a
   `Vec<u32>` of pixels that your own code fills. No GPU, no drawing calls of
   a window crate.
2. **A thin platform layer.** The only job of the window crate is to show a
   finished framebuffer in a window and to report which keys are down. Use
   `minifb` (pinned as `=0.29.0` in `Cargo.toml`) or replace it with another
   small crate; pin whatever you use with `=`.
3. **The feature.** The window crate is an optional dependency behind the
   cargo feature `window`, which is on by default. With
   `--no-default-features` the library, the headless mode of the binary and
   every test must build and pass using the standard library only.
4. **The library never names the window crate.** Only code in the binary,
   under `#[cfg(feature = "window")]`, may do so.
5. **The simulation is separate from presentation.** `Game::step` never
   draws. `Game::render` takes `&self` and never changes the game.
6. **Integers only.** The simulation and the renderer use integer
   arithmetic. There must be no `f32` or `f64` anywhere in the library.
   This is what makes results identical on Linux, macOS and Windows and in
   debug and release builds. Where arithmetic is meant to wrap (the checksum
   and the generator), say so with `wrapping_*` operations; everything else
   must stay in range, so that a debug build never panics on overflow.
7. **No clock in the library.** The library never reads the time. The binary
   measures real time and feeds it to `FixedStep`.
8. **No `unsafe`** in the library or the binary. (The counting allocator in
   `tests/common` is the only `unsafe` code in the package, and it is given.)
9. **No other dependencies** in the required part. Stretch goals may add
   crates behind their own features.

## 3. The framebuffer

### 3.1 Coordinates

A framebuffer has a width `w` and a height `h` in pixels, each in
`1..=16384`. Pixel `(x, y)` is in column `x` (0 is the left-most) and row
`y` (0 is the top row; y grows downward). Pixels are stored row by row from
the top: `pixels()[y * w + x]`.

`Framebuffer::new` panics if a side is outside `1..=16384`.

The game picture is `SCREEN_W` by `SCREEN_H` = 320 by 180 pixels.

### 3.2 Pixel format

A pixel is a `u32` holding four 8-bit channels:

```text
bits 31..24   23..16   15..8    7..0
     alpha    red      green    blue          0xAARRGGBB
```

Every pixel stored in a framebuffer is **opaque**: its alpha byte is `0xFF`.
A new framebuffer is opaque black, `0xFF000000`, everywhere. Every drawing
operation forces the alpha byte of what it stores to `0xFF`, whatever alpha
its colour argument has.

Alpha is meaningful only in sprites (section 4.4), where alpha 0 marks a
transparent pixel.

(This layout is the one `minifb` and most other software-presentation
crates expect, with the top byte ignored, so the buffer can be shown without
conversion.)

### 3.3 The checksum (exact)

One checksum algorithm is used for frames and for game state: **FNV-1a,
64 bit**.

```text
h = 0xcbf29ce484222325
for each byte b of the input, in order:
    h = h XOR b
    h = (h * 0x00000100000001b3) mod 2^64
result = h
```

`Framebuffer::checksum` feeds the pixels in storage order, each as its four
bytes in **little-endian** order (blue, green, red, alpha). The width and
height are not part of the input.

Worked example. A 1 by 1 framebuffer holds the pixel `0xFF000000`, the bytes
`00 00 00 FF`. After the first byte `h` is `0xaf63bd4c8601b7df`; after all
four the checksum is `0x4d25077f9dcd5758`.

### 3.4 PPM screenshots (exact)

`Framebuffer::write_ppm` writes a binary PPM image: the ASCII header

```text
P6\n<width> <height>\n255\n
```

(decimal numbers, single spaces, three newline characters as shown, nothing
else) followed by three bytes per pixel in storage order: red, green, blue.
A 320 by 180 screenshot is 15 + 172800 bytes long.

## 4. The rasteriser

All operations **clip**: parts that fall outside the framebuffer are not
drawn, and nothing else changes. No operation may panic or overflow for any
`i32` arguments (lines: see 4.3).

### 4.1 Pixels

- `clear(color)` sets every pixel.
- `set_pixel(x, y, color)` sets one pixel, or does nothing if `(x, y)` is
  outside.
- `get_pixel(x, y)` returns `Some(pixel)`, or `None` outside.

### 4.2 Rectangles (exact)

`fill_rect(x, y, w, h, color)` sets every pixel `(px, py)` with
`x <= px < x + w` and `y <= py < y + h` that lies inside the framebuffer.
The sums are mathematical: `x + w` may be larger than `i32::MAX` and the
result must still be right. If `w <= 0` or `h <= 0` nothing is drawn.

Example on an 8 by 5 buffer, `fill_rect(2, 1, 3, 2, c)`:

```text
........
..###...
..###...
........
........
```

### 4.3 Lines (exact)

`line(x0, y0, x1, y1, color)` draws the following pixels, both end points
included. Each pixel is drawn with the rule of `set_pixel`, so clipping
never moves a pixel.

```text
dx =  |x1 - x0|          sx = +1 if x0 < x1, otherwise -1
dy = -|y1 - y0|          sy = +1 if y0 < y1, otherwise -1
err = dx + dy
x = x0, y = y0
repeat:
    draw (x, y)
    if x == x1 and y == y1: stop
    e2 = 2 * err
    if e2 >= dy:  err = err + dy;  x = x + sx
    if e2 <= dx:  err = err + dx;  y = y + sy
```

Both `if` tests use the value of `e2` computed before either of them ran.
Use an integer type wide enough that `2 * err` cannot overflow for any `i32`
end points. (The tests only use coordinates between -10000 and 10000, so you
do not need to make very long off-screen lines fast.)

Worked example, `line(0, 0, 5, 2, c)`: `dx = 5`, `dy = -2`, `err = 3`.

| draw | `e2` | `e2 >= dy` | `e2 <= dx` | new `err` | next point |
|---|---|---|---|---|---|
| (0, 0) | 6 | yes | no | 1 | (1, 0) |
| (1, 0) | 2 | yes | yes | 4 | (2, 1) |
| (2, 1) | 8 | yes | no | 2 | (3, 1) |
| (3, 1) | 4 | yes | yes | 5 | (4, 2) |
| (4, 2) | 10 | yes | no | 3 | (5, 2) |
| (5, 2) | stop | | | | |

```text
##.....
..##...
....##.
```

The algorithm is not symmetric: `line(4, 1, 0, 3, c)` and
`line(0, 3, 4, 1, c)` set different pixels. Draw from the first point to the
second.

### 4.4 Sprites (exact)

A `Sprite` is a rectangle of `u32` pixels, at least 1 by 1, in the same
row-major order as a framebuffer. A sprite pixel whose alpha byte is `0x00`
is **transparent**. Every other pixel is opaque, whatever its alpha value.
A sprite keeps its pixels exactly as given.

`Sprite::new(width, height, pixels)` returns `None` if a side is zero or
`pixels.len() != width * height`.

`Sprite::from_art(art, palette)` builds a sprite from text:

- `art` is split at `\n`. One `\r` at the end of a line is removed. Lines
  that are then empty are skipped (they are not rows).
- Every remaining line is one row; every `char` in it is one pixel, looked
  up in `palette`, a list of `(char, colour)` pairs. The first pair with
  that `char` wins.
- Errors, checked in reading order: a character that is not in the palette
  gives `ArtError::UnknownChar { line, ch }`; a row whose length differs
  from the first row gives `ArtError::Ragged { line }`; no rows at all gives
  `ArtError::Empty`. `line` counts all lines from 1, skipped ones included.
  Within one line, an unknown character is reported in preference to a
  wrong length.

`blit(sprite, x, y, flip)` copies the sprite so that its top-left corner
lands on `(x, y)`. For every framebuffer pixel `(x + i, y + j)` with
`0 <= i < width` and `0 <= j < height` that lies inside the framebuffer:

```text
si = width  - 1 - i   if flip.x, otherwise i
sj = height - 1 - j   if flip.y, otherwise j
p  = sprite pixel (si, sj)
if the alpha byte of p is not 0:  framebuffer pixel = p with alpha forced to 0xFF
```

Flipping mirrors the sprite inside its own rectangle; the rectangle does not
move.

### 4.5 Blending (exact)

`blend_pixel(x, y, color, alpha)` mixes `color` over the pixel already at
`(x, y)`, or does nothing outside the framebuffer. For each of the red,
green and blue channels, with `s` the channel of `color`, `d` the channel of
the old pixel and `a = alpha` (0 to 255):

```text
new = (s * a + d * (255 - a) + 127) / 255        (integer division)
```

The alpha byte of `color` is ignored; the stored pixel is opaque. `alpha` 0
leaves the pixel unchanged and 255 replaces it.

Worked example: old pixel `0xFF64C800` (100, 200, 0), `color` `0x00FF0A32`
(255, 10, 50), `alpha` 77:

```text
red   (255 * 77 + 100 * 178 + 127) / 255 = 37562 / 255 = 147 = 0x93
green ( 10 * 77 + 200 * 178 + 127) / 255 = 36497 / 255 = 143 = 0x8F
blue  ( 50 * 77 +   0 * 178 + 127) / 255 =  3977 / 255 =  15 = 0x0F
result 0xFF938F0F
```

## 5. The font (exact)

Text is drawn with the fixed 5 by 7 pixel font in `assets/font5x7.txt`
(format: appendix A). The file is part of this specification; do not change
the glyphs.

`text(s, x, y, color)` draws the characters of `s` from left to right and
returns the x coordinate of the pen after the last character.

- The pen starts at `x`. Each `char` of `s` (each Unicode scalar value, not
  each byte) is drawn in a cell whose top-left corner is `(pen, y)`, then the
  pen moves right by **6** (5 pixels of glyph, 1 pixel of space).
- A glyph row `r` (0 to 6) and column `c` (0 to 4) that is lit in the font
  file sets the pixel `(pen + c, y + r)` with the rule of `set_pixel`.
  Unlit cells and the sixth column leave the framebuffer unchanged.
- A space draws nothing.
- The ASCII lower-case letters `a` to `z` use the glyphs of `A` to `Z`.
- Any other character without an entry in the file uses the entry
  `glyph missing` (a hollow box).
- The return value is `x + 6 * n` for `n` characters. It does not depend on
  clipping.

Worked example on a 13 by 9 buffer, `text("A1", 1, 1, c)` returns 13:

```text
.............
..###....#...
.#...#..##...
.#...#...#...
.#####...#...
.#...#...#...
.#...#...#...
.#...#..###..
.............
```

## 6. The fixed timestep

### 6.1 Why and how long

The simulation advances only in **steps** of one fixed length,
`STEP_MICROS = 16667` microseconds (about 1/60 second). A step is also
called a **tick**. How many steps run between two displayed frames depends
on how much real time passed; what one step does never depends on time.

### 6.2 `FixedStep` (exact)

`FixedStep::new(step_micros, max_steps)` creates an accumulator with zero
stored time. It panics if `step_micros` is 0.

`advance(elapsed_micros)`:

```text
acc = acc + elapsed_micros            (saturating at u64::MAX)
due = acc / step_micros               (integer division)
if due > max_steps:
    acc = 0                           (the backlog is dropped)
    return max_steps
else:
    acc = acc - due * step_micros
    return due
```

`alpha()` returns `acc * 256 / step_micros`, rounded down: a value in
`0..=255` that says how far real time has moved into the step that has not
run yet. The multiplication must not overflow.

Dropping the backlog when the cap is hit is deliberate: a machine that
cannot keep up slows the game down instead of falling further and further
behind. The game uses `max_steps = MAX_STEPS = 5`.

Example with `step_micros = 1000`, `max_steps = 5`: `advance(2500)` returns
2 and leaves `alpha() == 128`; then `advance(499)` returns 0 and leaves
`alpha() == 255`; then `advance(1)` returns 1 and leaves `alpha() == 0`.

### 6.3 The game loop

The windowed binary runs this loop (the structure is required; the code is
yours):

```text
clock = FixedStep::new(STEP_MICROS, MAX_STEPS)
last = now()
while the window is open:
    t = now();  elapsed = t - last;  last = t
    input = the buttons held right now
    repeat clock.advance(elapsed) times:
        game.step(input)
    game.render(framebuffer, clock.alpha())
    present the framebuffer
```

Input is sampled once per displayed frame; every step owed by that frame
gets the same input. Because the simulation only ever sees whole steps and
their inputs, two runs with the same seed and the same sequence of inputs
are identical, whatever the frame rate was.

## 7. The random number generator (exact)

The simulation uses one generator, known as SplitMix64. Its state is one
`u64`.

### 7.1 Algorithm

`Rng::new(seed)` sets the state to `seed`, unchanged. `state()` returns the
state.

`next_u64()`, with all additions and multiplications modulo 2^64:

```text
state = state + 0x9E3779B97F4A7C15
z = state
z = (z XOR (z >> 30)) * 0xBF58476D1CE4E5B9
z = (z XOR (z >> 27)) * 0x94D049BB133111EB
return z XOR (z >> 31)
```

`next_u32()` returns the **high** 32 bits of `next_u64()`.

Worked example: from seed 0 the first three results of `next_u64()` are
`0xE220A8397B1DCDAF`, `0x6E789E6AA1B965F4` and `0x06C45D188009454F`; after
the first one the state is `0x9E3779B97F4A7C15`.

### 7.2 `below`

`below(n)` returns `(next_u32() * n) >> 32`, computed in 64 bits. The result
is in `0..n`. `below(0)` returns 0. Every call consumes exactly one draw,
also for `n == 0` and `n == 1`. There is no rejection loop.

Worked example: from seed 0, four calls of `below(8)` return 7, 3, 0, 7.

### 7.3 Streams

`Game::new(level, seed)` starts the simulation's generator with
`Rng::new(seed)`. **Only the rule for Mistwings in 10.8 draws from it.**
Effects (particles) need random numbers too; they must use a second
generator of your own so that they never disturb the simulation's stream.
How you seed and use that second generator is free, as long as it is a
fixed function of `seed` (frames must be reproducible).

## 8. The level and the world

### 8.1 Level text (exact)

A level is text. Split it at `\n`; remove one `\r` from the end of each
line; then remove empty lines from the **end** of the list. Each remaining
line is one row of tiles, top row first; each `char` is one tile, left
column first.

| char | tile | also places |
|---|---|---|
| `.` | Empty | |
| `#` | Solid (rock) | |
| `~` | Water | |
| `P` | Empty | the player's start (exactly one) |
| `E` | Empty | the exit, the flue (exactly one) |
| `o` | Empty | a spark |
| `w` | Empty | a walker (Douser) |
| `f` | Empty | a flyer (Mistwing) |

Enemies are numbered from 0 in **reading order**: top row first, left to
right within a row, walkers and flyers together. Sparks are numbered the
same way. All "in order" rules below use these numbers. This is called
**spawn order**.

### 8.2 Level errors (exact)

`Game::new` returns the first error found by these checks, in this order:

1. No lines, or an empty first line: `LevelError::Empty`.
2. More than 256 columns (length of the first line) or more than 256 rows:
   `LevelError::TooLarge`.
3. Going through the lines from the top: a line whose length in `char`s
   differs from the first line gives `Ragged { line }`; otherwise the first
   `char` in it outside the table above gives
   `UnknownChar { line, column, ch }`. Lines and columns count from 1.
4. No `P`: `NoPlayer`. More than one: `ManyPlayers`.
5. No `E`: `NoExit`. More than one: `ManyExits`.

### 8.3 The tile map

`map_size()` is `(columns, rows)`. `tile(col, row)` returns the tile, with
this rule for coordinates outside the map, checked in this order:

- `col < 0`, `col >= columns` or `row < 0`: `Solid`;
- otherwise `row >= rows`: `Empty`.

So the cavern has rock to the left, to the right and above, and nothing
below.

### 8.4 Boxes and where things start

Everything that can touch something is an axis-aligned box: a top-left
corner `(x, y)` in subpixels (section 9) and a fixed size. A box covers the
half-open ranges `x .. x + w` and `y .. y + h`. Two boxes **overlap** when

```text
a.x < b.x + b.w  and  b.x < a.x + a.w  and  a.y < b.y + b.h  and  b.y < a.y + a.h
```

Boxes that only touch do not overlap.

The player, walkers and flyers start **standing in their tile**: centred
left to right, with the bottom of the box on the bottom edge of the tile.
For a box of size `(w, h)` in tile `(col, row)`, with `T = 4096`:

```text
x = col * T + (T - w) / 2
y = (row + 1) * T - h
```

A spark's box is 8 by 8 pixels, centred in its tile:
`x = col * T + 1024`, `y = row * T + 1024`. The exit's box is its whole
tile.

### 8.5 The camera (exact)

The camera is the top-left corner `(cx, cy)` of the 320 by 180 view, in map
pixels. It is a pure function of the player's box. With `px, py` the centre
of the box in whole pixels (`(x + 1280) >> 8` and `(y + 1792) >> 8`) and
`mw, mh` the map size in pixels (16 per tile):

```text
cx = (mw - 320) / 2                      if mw <= 320
     clamp(px - 160, 0, mw - 320)        otherwise
cy = (mh - 180) / 2                      if mh <= 180
     clamp(py - 90, 0, mh - 180)         otherwise
```

A map smaller than the screen is centred (the camera coordinate is then
negative or zero and never changes). `Game::camera()` returns this value
for the current state. It does not include screen shake.

### 8.6 The entity store (design requirement)

How you store the player, the enemies, the sparks and the particles is
free, with two conditions:

- Entities must not own or borrow each other, and there must be no
  `Rc<RefCell<..>>` graph. Refer to an entity by an index or by a handle
  (for example a generational index), or keep the data in parallel arrays.
- An enemy that has been **removed** keeps its number. There are two ways
  to be removed: being stomped (10.9) and, for a walker, falling out of the
  map (10.8). `enemy_count()` never changes, and `enemy(i)` for a removed
  enemy returns `alive: false` with `x == 0` and `y == 0`.

This is checked by reading your code (RUBRIC.md), not by a test.

## 9. Units and constants (exact)

Positions are `i32` **subpixels**: 256 subpixels are one pixel, so one tile
(16 pixels) is `T = 4096` subpixels. Velocities are subpixels per step. The
pixel that contains a subpixel coordinate `v` is `v >> 8`, and the tile is
`v.div_euclid(4096)`; both round toward negative infinity.

| name | value | meaning |
|---|---|---|
| player box | 2560 by 3584 | 10 by 14 pixels |
| walker box | 3072 by 2560 | 12 by 10 pixels |
| flyer box | 3072 by 2048 | 12 by 8 pixels |
| `GRAVITY` | 64 | added to vertical speed every step |
| `MAX_FALL` | 1536 | largest downward speed (6 pixels per step) |
| `JUMP_SPEED` | 1344 | upward speed of a jump |
| `JUMP_CUT` | 448 | largest upward speed while JUMP is not held |
| `RUN_ACCEL` | 96 | change of horizontal speed per step while running |
| `RUN_FRICTION` | 128 | change of horizontal speed per step while not running |
| `MAX_RUN` | 640 | largest running speed (2.5 pixels per step) |
| `COYOTE_TICKS` | 6 | see 10.3 |
| `BUFFER_TICKS` | 6 | see 10.3 |
| `INVULN_TICKS` | 90 | see 10.7 |
| `START_LIVES` | 3 | |
| `KNOCKBACK_X` | 512 | see 10.9 |
| `KNOCKBACK_Y` | 448 | see 10.9 |
| `STOMP_BOUNCE` | 1088 | see 10.9 |
| `STOMP_DEPTH` | 2048 | see 10.9 (8 pixels) |
| `WALKER_SPEED` | 128 | half a pixel per step |
| `FLYER_SPEED` | 96 | |
| `FLYER_PERIOD` | 48 | steps between changes of heading |
| `FLYER_LEASH` | 12288 | 3 tiles |

With these values no box moves more than 6 pixels in a step, which is less
than a tile. The collision rules below rely on that.

## 10. One simulation step (exact)

`Game::step(input)` performs the phases 10.1 to 10.11 in order and returns
the events that happened as a bit set (`Events`).

State that a new game starts with: tick 0; phase `Playing`; score 0; lives
3; the player at its start (8.4) with zero velocity, `on_ground` false,
facing right, and the counters `coyote`, `buffer` and `invuln` all 0; "JUMP
was held in the previous step" false; walkers with horizontal velocity
`-WALKER_SPEED` (they start out walking left) and vertical velocity 0;
flyers with zero velocity, their `timer` 0 and their `home` equal to their
start position.

Two helper definitions are used below.

`approach(v, target, step)`: move `v` toward `target` by `step` without
passing it: if `v < target` then `min(v + step, target)`, otherwise
`max(v - step, target)`.

For a box at `(x, y)` of size `(w, h)`: its **columns** are the tile columns
from `x.div_euclid(T)` to `(x + w - 1).div_euclid(T)` inclusive, and its
**rows** are the tile rows from `y.div_euclid(T)` to
`(y + h - 1).div_euclid(T)` inclusive.

### 10.1 Bookkeeping

1. `tick = tick + 1`.
2. Effects age by one step (12.2, 12.3).
3. If the phase is not `Playing`, the step ends here with no events. A won
   or lost game changes nothing but `tick` and its effects.
4. If `invuln > 0`, `invuln = invuln - 1`.

### 10.2 Running

`dir` is +1 if RIGHT is held and LEFT is not, -1 if LEFT is held and RIGHT
is not, and 0 otherwise.

- If `dir != 0`: `vx = approach(vx, dir * MAX_RUN, RUN_ACCEL)`, and the
  player now faces that way.
- Otherwise: `vx = approach(vx, 0, RUN_FRICTION)`.

The same rule applies in the air.

### 10.3 Jumping and gravity

1. *Buffer.* If JUMP is held now and was not held in the previous step (a
   **press**): `buffer = BUFFER_TICKS`. Otherwise, if `buffer > 0`:
   `buffer = buffer - 1`.
2. *Coyote time.* If `on_ground` (the value left by the previous step):
   `coyote = COYOTE_TICKS`. Otherwise, if `coyote > 0`:
   `coyote = coyote - 1`.
3. *Launch.* If `buffer > 0` and `coyote > 0`: `vy = -JUMP_SPEED`,
   `buffer = 0`, `coyote = 0`, and the event `JUMPED` happens.
4. *Gravity.* `vy = min(vy + GRAVITY, MAX_FALL)`.
5. *Jump cut.* If JUMP is not held and `vy < -JUMP_CUT`: `vy = -JUMP_CUT`.

So a press is remembered for a few steps (it still works if it came
slightly before landing), the ground is remembered for a few steps (a jump
still works slightly after walking off a ledge), a held button does not
jump again, and letting go early makes a lower jump.

Worked example. The player stands on a floor with `y = Y`. JUMP is pressed
and kept held.

| step | what happens | `vy` after | `y` after |
|---|---|---|---|
| 1 | launch -1344, gravity +64 | -1280 | Y - 1280 |
| 2 | gravity | -1216 | Y - 2496 |
| 3 | gravity | -1152 | Y - 3648 |
| ... | | | |
| 20 | gravity | -64 | Y - 13440 |
| 21 | gravity | 0 | Y - 13440 |
| 22 | gravity | 64 | Y - 13376 |
| ... | | | |
| 41 | gravity | 1280 | Y |
| 42 | gravity 1344; the floor stops the fall | 0 | Y |

The highest point is 13440 subpixels (52.5 pixels) above the floor. After
step 41 the feet are level with the floor but the boxes only touch, so
`on_ground` is still false; step 42 finds the floor and reports `LANDED`.

If JUMP is released after step 1, step 2 gives `vy = -1216`, which the jump
cut changes to -448; the steps after that give -384, -320, and so on, and
the jump rises 3072 subpixels (12 pixels) in total.

### 10.4 Horizontal movement and walls

`x = x + vx`. Then:

- If `vx > 0`: let `c = (x + w - 1).div_euclid(T)`. If any tile `(c, r)`,
  for `r` in the box's rows, is Solid: `x = c * T - w` and `vx = 0`.
- If `vx < 0`: let `c = x.div_euclid(T)`. If any tile `(c, r)`, for `r` in
  the box's rows, is Solid: `x = (c + 1) * T` and `vx = 0`.
- If `vx == 0` nothing is tested.

The rows are those of the box at its current `y` (vertical movement comes
next).

### 10.5 Vertical movement, floor and ceiling

Remember the old value of `on_ground`, then set `on_ground = false`.
`y = y + vy`. Then:

- If `vy > 0`: let `r = (y + h - 1).div_euclid(T)`. If any tile `(c, r)`,
  for `c` in the box's columns (at the new `x`), is Solid: `y = r * T - h`,
  `vy = 0`, `on_ground = true`; and if `on_ground` was false before, the
  event `LANDED` happens.
- If `vy < 0`: let `r = y.div_euclid(T)`. If any tile `(c, r)`, for `c` in
  the box's columns, is Solid: `y = (r + 1) * T` and `vy = 0`. (A ceiling;
  `on_ground` stays false.)

Horizontal movement is resolved completely before vertical movement. This
order is part of the rules: it decides what happens at corners.

Because gravity is added in every step, a standing player has `vy = 64`
before this phase and is pushed back onto the floor in it, which is how
`on_ground` stays true. In the very first step of a game this produces a
`LANDED` event (and again in the step after every respawn).

### 10.6 Hazards

If the player's box touches Water (any tile in the box's columns and rows
is Water), or if `y >= rows * T` (the top of the box has left the map
through the bottom):

1. The player loses a life (10.7).
2. If the game is still `Playing`: the player is put back at its start
   position with `vx = vy = 0`, `on_ground = false`, `coyote = 0` and
   `buffer = 0`.

Invulnerability does not protect against hazards. If the game is now
`Lost`, the player stays where it died, and the step skips 10.8 to 10.11.

### 10.7 Losing a life

`lives = lives - 1`; the event `HURT` happens; a shake of 12 steps starts
(12.3). Then:

- If `lives == 0`: the phase becomes `Lost` and the event `LOST` happens.
- Otherwise: `invuln = INVULN_TICKS`.

### 10.8 Enemies move

Every living enemy, in spawn order:

**Walker.**

1. `vy = min(vy + GRAVITY, MAX_FALL)`; `y = y + vy`. If `vy > 0`, test the
   floor exactly as in 10.5 (first bullet). If the floor was hit:
   `y = r * T - h`, `vy = 0`, and the walker is **grounded** for this step.
2. If the walker is not grounded and `y >= rows * T`, it is removed from the
   game (it fell out of the map; no score, no event). The step is over for
   this walker, and from now on it is not alive (8.6).
3. If the walker is grounded, it tries to move to `nx = x + vx` (its `vx`
   is `+WALKER_SPEED` or `-WALKER_SPEED`). The move is **blocked** if
   - *wall:* the box at `(nx, y)` hits a Solid tile by the test of 10.4, or
   - *ledge:* the tile below its leading foot is not Solid. The foot point
     is `(nx + w - 1, y + h)` when moving right and `(nx, y + h)` when
     moving left; the tile is that point's `div_euclid(T)` in both
     coordinates.

   If the move is blocked, the walker does not move in this step and its
   `vx` changes sign. Otherwise `x = nx`.

Walkers ignore water (it does not hurt them, and since Water is not Solid
they turn around at its edge).

**Flyer.**

1. If `timer == 0`: draw `d = rng.below(8)` from the simulation's generator;
   set `(vx, vy) = HEADINGS[d] * FLYER_SPEED` (both components, no
   normalising of diagonals); set `timer = FLYER_PERIOD`.

   ```text
   HEADINGS = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
   ```
2. `timer = timer - 1`.
3. `x = x + vx`; `y = y + vy`.
4. The leash: if `x - home.x > FLYER_LEASH` and `vx > 0`, or
   `home.x - x > FLYER_LEASH` and `vx < 0`, then `vx = -vx`. The same for
   `y` and `vy`.

Flyers ignore tiles, water and gravity. A flyer therefore draws in steps
1, 49, 97, ... for as long as it lives; several flyers draw in spawn order
within a step.

### 10.9 The player meets the enemies

Let `falling` be "the player's `vy > 0`", decided once, before the first
enemy is looked at. Then, for every living enemy in spawn order:

1. If `invuln > 0`, or the player's box and the enemy's box do not overlap:
   nothing happens with this enemy.
2. Otherwise, if `falling` and `player.y + player.h - enemy.y <= STOMP_DEPTH`
   (the player's feet are at most 8 pixels below the enemy's top): a
   **stomp**. The enemy is defeated and removed; the score grows by 50 for
   a walker or 80 for a flyer; the player's `vy = -STOMP_BOUNCE`; the event
   `STOMP` happens; a shake of 6 steps starts.
3. Otherwise the player is **hurt**: `vx = -KNOCKBACK_X` if the centre of
   the player's box is left of the centre of the enemy's box
   (`player.x + 1280 < enemy.x + 1536`), else `vx = +KNOCKBACK_X`;
   `vy = -KNOCKBACK_Y`; `on_ground = false`; then the player loses a life
   (10.7). If the game is now `Lost`, the step skips the rest of 10.9 and
   10.10 to 10.11.

Consequences worth noticing: `falling` is decided once so that one fall can
stomp two enemies. After being hurt the player is invulnerable, and an
invulnerable player passes through enemies: it can neither be hurt nor
stomp. The bounce of a stomp is cut by the jump cut of 10.3 in the next
step unless JUMP is held, so holding JUMP gives a high bounce.

### 10.10 Sparks

Every spark not yet collected, in spawn order: if the player's box overlaps
the spark's box, the spark is collected, the score grows by 10 and the
event `SPARK` happens.

### 10.11 The exit

If no spark is left (also true for a level without sparks) and the player's
box overlaps the exit's box: the phase becomes `Won`, the score grows by
`100 * lives`, and the event `WON` happens.

### 10.12 Events

`Events` is a bit set: `JUMPED` 1, `LANDED` 2, `SPARK` 4, `STOMP` 8,
`HURT` 16, `WON` 32, `LOST` 64. A bit is set if that event happened at
least once in the step.

## 11. The state hash (exact)

`Game::state_hash()` is FNV-1a (3.3) over the following values, each in
little-endian byte order, in this order:

| value | type |
|---|---|
| `tick` | `u64` |
| the state of the simulation's generator | `u64` |
| phase: 0 `Playing`, 1 `Won`, 2 `Lost` | `u32` |
| score | `u32` |
| lives | `u32` |
| player `x`, `y`, `vx`, `vy` | four `i32` |
| number of enemies the level started with | `u32` |
| for each enemy in spawn order: kind (0 walker, 1 flyer), alive (1 or 0), `x`, `y` | `u32`, `u32`, `i32`, `i32` |
| number of sparks the level started with | `u32` |
| for each spark in spawn order: collected (1 or 0) | `u32` |

A removed enemy (8.6), whether it was stomped or fell out of the map,
contributes alive 0, `x` 0 and `y` 0.

Counters such as `coyote` and timers are not hashed. They do not need to
be: a wrong counter changes a position a few steps later, and positions are
hashed.

Worked example. The level `#.P.E#` over `######`, seed 0, before any step:
the player's box is at `x = 2 * 4096 + 768 = 8960`, `y = 4096 - 3584 = 512`.
The 52 input bytes are

```text
00 00 00 00 00 00 00 00   tick 0
00 00 00 00 00 00 00 00   generator state 0
00 00 00 00               phase Playing
00 00 00 00               score 0
03 00 00 00               lives 3
00 23 00 00               x 8960
00 02 00 00               y 512
00 00 00 00               vx 0
00 00 00 00               vy 0
00 00 00 00               0 enemies
00 00 00 00               0 sparks
```

and the hash is `0x0017b15115d86541`.

## 12. Presentation

### 12.1 Animation (exact)

At the end of every step in which the phase was `Playing` at 10.1, the
player's animation is chosen from the body:

| condition (first match) | animation |
|---|---|
| not `on_ground` and `vy < 0` | `Jump` |
| not `on_ground` | `Fall` |
| `vx != 0` | `Run` |
| otherwise | `Idle` |

If the animation is the same as after the previous step, its counter
`anim_ticks` grows by 1; otherwise the animation changes and `anim_ticks`
becomes 0. A new game starts with `Fall` and `anim_ticks = 0`.

`Game::anim()` returns the animation and the frame number
`(anim_ticks / period) % frames`:

| animation | period (steps per frame) | frames |
|---|---|---|
| `Idle` | 24 | 2 |
| `Run` | 5 | 4 |
| `Jump` | 1 | 1 |
| `Fall` | 1 | 1 |

What the frames look like is free.

### 12.2 Particles (counts and lifetimes exact, the rest free)

Particles are small, short-lived specks for dust, splashes and glitter.
They are effects: they never influence the simulation and are not part of
the state hash.

- The game owns a **pool** of at most **64** live particles, created with
  the game. `Game::particles()` returns the number of live particles.
- Spawning: each occurrence of an event spawns particles in that step.

  | occurrence | particles |
  |---|---|
  | `JUMPED` | 4 |
  | `LANDED` | 4 |
  | each spark collected | 6 |
  | each enemy stomped | 12 |
  | each life lost (`HURT`) | 16 |
  | `WON` | 48 |

  If the pool is full, the particles that do not fit are simply not
  spawned. A live particle is never removed to make room.
- Lifetime: each particle gets a lifetime `L` of your choice with
  `10 <= L <= 40` steps. A particle spawned in step `t` is counted by
  `particles()` after the steps `t` to `t + L - 1` and is gone after step
  `t + L`. Ageing happens in 10.1, also when the game is won or lost.
- Updating and spawning particles must not allocate (section 16).
- Position, movement, colour, size and the choice of `L` are free. Use the
  effects generator of 7.3 for anything random.

### 12.3 Screen shake (exact)

The game holds a counter `shake`, 0 in a new game. In 10.1, if `shake > 0`
it is reduced by 1. A life lost sets `shake = max(shake, 12)`; a stomp sets
`shake = max(shake, 6)`.

`Game::shake()` returns the offset in pixels for the current value
`t = shake`:

```text
if t == 0: (0, 0)
amp = (t + 2) / 3                          (integer division)
dx = +amp if t is odd, otherwise -amp
dy = +amp if (t / 2) is odd, otherwise -amp
```

For example `t = 12` gives `(-4, -4)`, `t = 11` gives `(4, 4)`, and `t = 1`
gives `(1, -1)`. The renderer moves the world by this offset; the HUD does
not move.

### 12.4 The HUD (exact)

The top 11 rows of every rendered frame are the HUD, drawn last, on top of
everything else in those rows:

```text
fill_rect(0, 0, 320, 11, 0xFF10101C)
text("SPARKS <c>/<n>  LIVES <l>  SCORE <ssssss>", 4, 2, 0xFFF4F0E0)
```

`<c>` is the number of sparks collected, `<n>` the number the level started
with, `<l>` the lives, all in plain decimal; `<ssssss>` is the score in
decimal, padded with zeros to at least 6 digits. There are two spaces
before `LIVES` and before `SCORE`. Example:
`SPARKS 3/14  LIVES 2  SCORE 000130`.

Building this text every frame must not allocate.

### 12.5 Rendering (contract exact, picture free)

`Game::render(&self, fb, alpha)` draws the current game into a 320 by 180
framebuffer.

- It sets **every** pixel of the buffer; nothing of the previous contents
  may remain visible.
- It is a pure function of the game state and `alpha`: the same state and
  `alpha` always give the same pixels. It does not change the game.
- `alpha` is in `0..=256` and asks for **interpolation**: moving things are
  drawn at `previous + (current - previous) * alpha / 256`, where
  "previous" is the position at the start of the latest step. `alpha = 256`
  draws the current state exactly. If nothing moved in the latest step, the
  frame must not depend on `alpha`. Two cases need care:
  - *A finished game.* A step of a won or lost game moves nothing (10.1),
    so after such a step "previous" equals "current" for the player and for
    every enemy. If you only update "previous" where things move, the
    picture of a finished game keeps sliding between its last two
    positions.
  - *A respawn.* The respawn of 10.6 is a teleport, not a movement: do not
    interpolate across it. In the step of a respawn the player's "previous"
    is the start position too. (A particle that was spawned in the latest
    step has not moved yet either.)
- The camera of 8.5 (computed from the interpolated player position), moved
  by the shake of 12.3, decides which part of the map is visible.
- The HUD of 12.4 is drawn exactly.

Everything else is free: the background, how tiles, Nib, the enemies, the
sparks and the flue look, lighting, messages for winning and losing. Make
it look good. The reference draws a gradient with two parallax layers of
rock, textured tiles generated in code, sprites from `assets/sprites.txt`,
and soft lights made with `blend_pixel`.

## 13. The replay format (exact)

A replay is the seed and the input of every step, as text.

```text
underkiln-replay 1
seed 2026
ticks 9
3 0
2 2
1 6
2 2
1 0
```

- Line 1 is exactly `underkiln-replay 1`.
- Line 2 is `seed ` followed by the seed in decimal (`u64`).
- Line 3 is `ticks ` followed by the number of steps in decimal, at most
  10000000.
- Every further line is a **run**, `<count> <mask>`: `count` (decimal, at
  least 1) steps in a row with the input `mask` (decimal, 0 to 7; bit 1 is
  LEFT, 2 is RIGHT, 4 is JUMP).
- Numbers are plain decimal digits: no sign, no spaces. Leading zeros are
  accepted. A number that does not fit in a `u64` is not valid.

`Replay::to_text()` writes the **canonical** form: the three header lines,
then one line per maximal run of equal inputs, every line ended by `\n`,
nothing else.

`Replay::from_text()` accepts the canonical form and also: runs that are
not merged, lines that are empty or start with `#` (ignored, anywhere),
and `\r\n` line endings. It returns the first error found:

1. The first line that is not ignored is missing or is not the header:
   `BadHeader`.
2. The next one is missing or is not a valid `seed` line: `BadSeed`.
3. The next one is missing or is not a valid `ticks` line (or the number is
   above 10000000): `BadTicks`.
4. A later line that is not a valid run: `BadRun { line }`, with the line
   number counted from 1 over all lines, ignored ones included.
5. All runs are valid but their counts add up to `found`, which is not the
   promised `expected`: `LengthMismatch { expected, found }`. The counts
   come from a file, so their sum can be larger than a `u64` can hold: add
   them with saturation (`found` is then `u64::MAX`), never with a `+` that
   can overflow.

`from_text` must check the runs before it allocates the inputs, so that a
small file cannot make it allocate more than 10000000 inputs.

Replaying means: build the game from the same level and `seed`, then call
`step` with each input in order. The result must be identical to the
recorded run in every state hash and every rendered frame.

`assets/demo.replay` is a recorded run that wins `assets/level1.txt` with a
score of 590 in 1016 steps.

## 14. The binary

The package builds one binary, `game`.

### 14.1 Windowed mode (needs the `window` feature)

`game [--level FILE] [--seed N] [--replay FILE] [--record FILE] [--frames N] [--scale N]`

- Opens a window showing the 320 by 180 picture scaled up by a whole
  factor (default 4, set with `--scale N`) and runs the loop of 6.3. Which
  factors you accept is free; `minifb` offers 1, 2, 4 and larger powers of
  two. Refuse any other value with a message and exit code 2.
- Keys: left and right arrow (or A and D) run; space, up arrow, W or Z jump;
  R starts a new game; Escape or closing the window quits. Other keys are
  free (the reference saves a screenshot on F2).
- `--level FILE` loads a level; without it the built-in first level is used.
- `--seed N` seeds a new game (default 1).
- `--replay FILE` feeds the game from a replay instead of the keyboard.
- `--record FILE` writes the inputs of the session as a replay when the
  program ends.
- `--frames N` quits after presenting N frames. (This is how the window can
  be smoke-tested by a script.)

A build without the feature must still compile; asked for windowed mode, it
prints a message on standard error and exits with code 2.

### 14.2 Headless mode (exact; must work without the feature)

`game --headless --replay FILE [--level FILE] [--out FILE] [--shots DIR] [--shot-at T,T,...]`

Builds the game from the level (default: the built-in first level) and the
replay's seed. Then, for every input of the replay: `step`, then `render`
into a 320 by 180 framebuffer with `alpha = 256`, then print one line

```text
<tick> <state hash> <frame checksum>
```

with the tick in decimal and both checksums as exactly 16 lower-case
hexadecimal digits. After the last step, print

```text
end phase=<playing|won|lost> score=<score> lives=<lives> ticks=<tick>
```

Every line ends with a newline. Nothing else is printed on standard output.

- `--out FILE`: write the lines to `FILE` instead of standard output
  (standard output then stays empty).
- `--shots DIR`: create `DIR` if needed and save screenshots into it as
  `frame_<tick, 6 digits, zero padded>.ppm` (format 3.4, the frame that was
  just rendered). With `--shot-at T,T,...` the frames after those steps are
  saved; without it, only the frame after the last step.

Errors (no `--replay`, a file that cannot be read, a replay or level that
does not parse, an unknown option): print a message on standard error,
print nothing on standard output, exit with code **2**. The program must
not panic on bad input.

## 15. The API facade (exact)

The tests use only these items of the library crate `game`, plus the
binary. The starter's `src/lib.rs` contains all of them; the plain data
types are complete, the rest have `todo!()` bodies. Keep the names and
signatures. Everything else in your crate is private design.

```rust,ignore
pub const SCREEN_W: usize = 320;
pub const SCREEN_H: usize = 180;
pub const STEP_MICROS: u64 = 16_667;
pub const MAX_STEPS: u32 = 5;

pub struct Framebuffer { /* yours */ }
impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
    pub fn pixels(&self) -> &[u32];
    pub fn clear(&mut self, color: u32);
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32>;
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32);
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8);
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32);
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32);
    pub fn blit(&mut self, sprite: &Sprite, x: i32, y: i32, flip: Flip);
    pub fn text(&mut self, s: &str, x: i32, y: i32, color: u32) -> i32;
    pub fn checksum(&self) -> u64;
    pub fn write_ppm<W: std::io::Write>(&self, out: &mut W) -> std::io::Result<()>;
}

pub struct Sprite { /* yours */ }
impl Sprite {
    pub fn new(width: usize, height: usize, pixels: Vec<u32>) -> Option<Sprite>;
    pub fn from_art(art: &str, palette: &[(char, u32)]) -> Result<Sprite, ArtError>;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
    pub fn pixel(&self, x: usize, y: usize) -> Option<u32>;
}

pub struct FixedStep { /* yours */ }
impl FixedStep {
    pub fn new(step_micros: u64, max_steps: u32) -> FixedStep;
    pub fn advance(&mut self, elapsed_micros: u64) -> u32;
    pub fn alpha(&self) -> u32;
}

pub struct Rng { /* yours */ }
impl Rng {
    pub fn new(seed: u64) -> Rng;
    pub fn next_u64(&mut self) -> u64;
    pub fn next_u32(&mut self) -> u32;
    pub fn below(&mut self, n: u32) -> u32;
    pub fn state(&self) -> u64;
}

pub struct Game { /* yours */ }
impl Game {
    pub fn new(level: &str, seed: u64) -> Result<Game, LevelError>;
    pub fn step(&mut self, input: Input) -> Events;
    pub fn render(&self, fb: &mut Framebuffer, alpha: u32);
    pub fn tick(&self) -> u64;
    pub fn phase(&self) -> Phase;
    pub fn score(&self) -> u32;
    pub fn lives(&self) -> u32;
    pub fn player(&self) -> Body;
    pub fn on_ground(&self) -> bool;
    pub fn map_size(&self) -> (u32, u32);
    pub fn tile(&self, col: i32, row: i32) -> Tile;
    pub fn enemy_count(&self) -> usize;
    pub fn enemy(&self, index: usize) -> Option<EnemyView>;
    pub fn sparks_left(&self) -> u32;
    pub fn camera(&self) -> (i32, i32);
    pub fn shake(&self) -> (i32, i32);
    pub fn particles(&self) -> usize;
    pub fn anim(&self) -> (Anim, u32);
    pub fn state_hash(&self) -> u64;
}

impl Replay {
    pub fn to_text(&self) -> String;
    pub fn from_text(text: &str) -> Result<Replay, ReplayError>;
}

// Plain data, complete in the starter:
pub struct Flip { pub x: bool, pub y: bool }            // NONE, X, Y, BOTH
pub struct Input(pub u8);                               // LEFT 1, RIGHT 2, JUMP 4, ALL 7
pub struct Events(pub u32);                             // see 10.12
pub enum Phase { Playing, Won, Lost }
pub enum Tile { Empty, Solid, Water }
pub enum EnemyKind { Walker, Flyer }
pub struct Body { pub x: i32, pub y: i32, pub vx: i32, pub vy: i32 }
pub struct EnemyView { pub kind: EnemyKind, pub alive: bool, pub x: i32, pub y: i32 }
pub enum Anim { Idle, Run, Jump, Fall }
pub enum ArtError { Empty, Ragged { line: usize }, UnknownChar { line: usize, ch: char } }
pub enum LevelError { Empty, TooLarge, Ragged { .. }, UnknownChar { .. },
                      NoPlayer, ManyPlayers, NoExit, ManyExits }
pub enum ReplayError { BadHeader, BadSeed, BadTicks, BadRun { line: usize },
                       LengthMismatch { expected: u64, found: u64 } }
pub struct Replay { pub seed: u64, pub inputs: Vec<Input> }
```

`Game::player()` returns the player's box corner and velocity in subpixels.
The tests always pass a 320 by 180 framebuffer to `render`.

## 16. The performance budget (exact)

After start-up, a frame must not allocate. Precisely: once the game, the
framebuffer and the clock exist and one frame has been rendered,

```text
for each displayed frame:
    clock.advance(..)
    game.step(input)          zero or more times
    game.render(fb, alpha)
    fb.checksum()
```

performs **zero** heap allocations and reallocations, for a whole game
including pickups, stomps, injuries, winning and losing. The test
`tests/m6_alloc.rs` installs a counting allocator and runs both shipped
replays this way.

Start-up may allocate freely: loading the level, building sprites and
tables, creating the particle pool. The platform layer is outside the
budget, but a careful implementation reserves the memory for recording
inputs once.

There is no timing test. As a guide, a frame of the reference takes well
under a millisecond in a release build.

## 17. Milestones and their tests

Work in this order. Each milestone has one test file; a later file needs
the earlier milestones.

| milestone | you build | SPEC | test file |
|---|---|---|---|
| M1 | framebuffer, rectangles, lines, sprites, blending, font, checksum, PPM | 3, 4, 5 | `tests/m1_raster.rs` |
| M2 | `FixedStep`; the window, the loop, the keys (by hand: run it) | 6, 14.1 | `tests/m2_loop.rs` |
| M3 | `Rng`, level parsing, the tile map, the entity store, the camera | 7, 8 | `tests/m3_world.rs` |
| M4 | physics, collisions, enemies, sparks, winning and losing, the state hash | 9, 10, 11 | `tests/m4_gameplay.rs` |
| M5 | animation, particles, shake, the HUD, a good-looking `render` | 12 | `tests/m5_feel.rs` |
| M6 | replays, headless mode, the allocation budget | 13, 14.2, 16 | `tests/m6_replay.rs`, `tests/m6_alloc.rs` |

Notes on the order:

- `tests/m2_loop.rs` contains two tests that need a working `Game` (frame
  rate independence and determinism). They pass once M4 is done. The
  `FixedStep` tests pass right away.
- In M2 you can already open a window and move a rectangle with the keys:
  there is no game yet, but the loop is the real one. Do this; it is the
  moment the project becomes fun.
- The golden traces in `tests/golden/` list `tick x y score lives
  state_hash` at chosen steps of two recorded runs. When a trace test
  fails, it shows the first line that differs.

All tests are deterministic and none opens a window. No test measures
speed; the only clock in the tests is a watchdog that stops a test program
after 60 seconds, so that an endless loop in your code fails with a message
(`TIMEOUT: the test ...`) instead of hanging. Run the tests with
`cargo test --no-default-features` (standard library only) and with
`cargo test` (the window crate is compiled but not used by the tests).

## 18. Stretch goals

None of these is tested. Put each behind its own cargo feature so that the
required build stays as it is.

- **More levels** and a title screen; a level select.
- **Audio:** jump, pickup and stomp sounds, generated in code (square and
  noise waves) and mixed in a callback that never allocates. Drive it from
  the `Events` of each step.
- **Gamepad input** in the platform layer.
- **A GPU renderer** with `wgpu` behind a feature `gpu`: upload the
  framebuffer as a texture first; then draw tiles and sprites as textured
  quads and compare its frames with the software renderer's.
- **A level editor:** a second mode of the window that paints tiles with
  the mouse and saves level text.
- **Data-driven levels and enemies:** move the constants of section 9 into
  a text file loaded at start-up.
- **A WebAssembly build:** the library already has no window and no clock;
  write a small platform layer for a browser canvas.
- **Save states and rewind:** the whole game is plain data, so a snapshot
  is a copy. Keep the last few seconds and let a key step backwards.
- **One-way platforms, moving platforms, new enemies.** Extend the rules
  and then extend the tests the same way this specification does.

## 19. Non-goals

- No GPU, no shaders, no 3D in the required part.
- No floating-point numbers in the library.
- No physics engine, no ECS framework, no game engine crate. The point is
  to write these parts.
- No networking and no multiplayer.
- No art, sound, names or levels taken from any existing game.
- No frame-rate or timing assertions in tests.

---

## Appendix A: font file format

`assets/font5x7.txt` is ASCII text.

- Lines that are empty or start with `;` are ignored.
- `glyph <c>` starts the glyph for the character `<c>` (one printable ASCII
  character other than space). `glyph missing` starts the glyph used for
  characters without an entry.
- The next 7 lines are the rows, top to bottom, of exactly 5 characters
  each: `#` is a lit pixel, `.` is not.

The file defines the digits, the upper-case letters, the characters
`. , : ! ? - + / ' ( ) % = *` and `missing`. A line may end in `\r\n` if
your checkout converts line endings; accept that.

You may read this file at run time, include it with `include_str!`, or
turn it into a table at compile time. The glyphs must be the ones in the
file.

## Appendix B: sprite sheet format

`assets/sprites.txt` is an optional convenience: original pixel art for
Nib, the Dousers, the Mistwings and the sparks. No test looks at it.

- Lines that are empty or start with `#` are ignored.
- `color <c> <aarrggbb>` adds the palette entry: character `<c>`, colour in
  8 hexadecimal digits. Alpha `00` is transparent.
- `sprite <name>` starts a sprite. The lines up to the next `sprite` line
  are its rows, one palette character per pixel: exactly the input of
  `Sprite::from_art`.

Sprites in the file: `nib_body_a`, `nib_body_b`, `nib_body_up`,
`nib_body_down` (16 by 14, drawn above a pair of feet); `nib_feet_stand`,
`nib_feet_left`, `nib_feet_right`, `nib_feet_tuck` (16 by 2);
`douser_a`, `douser_b` (16 by 12; the 12 by 10 collision box is centred
left to right and sits on the bottom); `mistwing_a`, `mistwing_b` (16 by
12; the 12 by 8 box is centred); `spark_a`, `spark_b` (8 by 8).

## Appendix C: the shipped art

The font, the sprite sheet, the level and the replays in this package were
made for this course. They are original work and are covered by the same
MIT licence as the rest of the course repository. There are no third-party
assets.
