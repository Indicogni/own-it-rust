# Capstone: Underkiln, a graphical game

[All chapters](../../README.md) · [Specification](SPEC.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write a real-time 2D game in Rust, from an almost empty crate: a window
opens, a small fire spirit called Nib runs and jumps through a cavern,
gathers sparks, avoids or stomps water creatures, and lights the flue at the
end of the level.

You also write everything under it: the rasteriser that puts pixels into a
framebuffer, the fixed-timestep loop, the tile map and the entity store, the
collision code, the particle pool, the replay system and a headless mode.
There is no game engine. The only third-party crate shows your finished
pixels in a window and tells you which keys are down.

This is a **blank-file project**. There are no lesson files and no function
bodies to fill in one at a time. You get a precise specification, a small
API facade that the tests call, original art to use if you like, and 93
acceptance tests that never open a window.

| file | what it is |
|---|---|
| [SPEC.md](SPEC.md) | the complete specification: read all of it first |
| [CONCEPTS.md](CONCEPTS.md) | the ideas you need, with small compiled examples |
| [HINTS.md](HINTS.md) | design hints per milestone; no code |
| [RUBRIC.md](RUBRIC.md) | how to judge your own result |
| `src/lib.rs` | the API facade, with `todo!()` bodies |
| `src/main.rs` | a stub for the binary |
| `assets/` | the font (part of the SPEC), the first level, a demo replay, optional sprite art |
| `tests/` | the acceptance tests, one file per milestone |

The model solution, with its own `DESIGN.md`, is in
`solutions/capstones/game/` of the course repository. Look at it after you
have finished, or when you have been stuck on one problem for a long time.

## Prerequisites

You need most of the course. The table says what each part is used for.

| from | you use it for |
|---|---|
| [Chapter 03](../../exercises/chapter03/README.md), Numbers, text and arrays | fixed-width integers, wrapping and checked arithmetic, slices, `u32` pixels |
| Chapters [04](../../exercises/chapter04/README.md) and [05](../../exercises/chapter05/README.md), Ownership and moves; Borrowing with references | the whole design: who owns the world, what `step` may borrow at once |
| [Chapter 07](../../exercises/chapter07/README.md), Handling errors | level, sprite and replay errors; the exit code of the binary |
| [Chapter 08](../../exercises/chapter08/README.md), Modules, crates and Cargo | a library plus a binary, modules, cargo features, an optional dependency |
| Chapters [09](../../exercises/chapter09/README.md) and [10](../../exercises/chapter10/README.md), Structs and methods; Enums and pattern matching | entities, tiles, phases, events |
| [Chapter 11](../../exercises/chapter11/README.md), Generic code with traits | `impl Write` for screenshots, small generic containers |
| Chapters [15](../../exercises/chapter15/README.md) and [16](../../exercises/chapter16/README.md), Iterators and adapters; Standard collections | tile ranges, run-length encoding, `Vec` as an arena |
| [Chapter 17](../../exercises/chapter17/README.md), Working with text | parsing levels, sprites, fonts and replays; formatting without allocating |
| [Chapter 18](../../exercises/chapter18/README.md), Files, streams and processes | files, standard output, `std::process::Command` in the tests |
| Mastery [`m7_perf`](../../mastery/m7_perf/README.md) | the allocation budget, data layout, measuring before optimising |
| Mastery [`m8_api`](../../mastery/m8_api/README.md) | keeping a small public facade stable while the inside changes |

Helpful but not required: [Chapter 13](../../exercises/chapter13/README.md) (Everyday traits) for `Default`,
`Clone` and `Copy` decisions, and [Chapter 23](../../exercises/chapter23/README.md) (Unsafe Rust) to understand the
counting allocator in `tests/common/mod.rs`. You do not write any `unsafe`
code in this project.

You do **not** need any knowledge of graphics or games. CONCEPTS.md explains
framebuffers, rasterisation, game loops, fixed-point numbers and collision
from the beginning.

Time: this is the largest kind of project in the course. As a rough
estimate, plan 40 to 70 hours: M1 8 to 12, M2 3 to 5, M3 5 to 8, M4 10 to
18, M5 8 to 14, M6 6 to 10. These are estimates, not targets.

## Setting up

The package stands alone. Work inside `capstones/game/`; it has its own
`Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml` (Rust 1.99.0, the same
as the rest of the course).

The window crate is [`minifb`](https://docs.rs/minifb/0.29.0/minifb/),
pinned to version 0.29.0.

- **macOS and Windows:** nothing to install.
- **Linux:** `minifb` talks to X11 or Wayland and needs a few system
  packages to build. Its README names these for Debian and Ubuntu:

  ```text
  sudo apt install libxkbcommon-dev libwayland-cursor0 libwayland-dev
  ```

  On Fedora the matching packages are `libxkbcommon-devel` and
  `wayland-devel`. (The Linux package names come from the crate's
  documentation; the reference solution was built and run on macOS.)

If you cannot or do not want to install them, you can still do almost all
of the project: everything except the window builds with the standard
library only.

```text
cargo build --no-default-features     # library, headless binary: std only
cargo test  --no-default-features     # all 93 tests, no window crate
cargo build                           # the same plus the window
cargo test                            # the same tests again
```

### The window in the dev container or a codespace

The course's dev container (see [Setup](../../README.md#setup)) already has
the Linux packages that `minifb` needs. A container has no screen of its own,
so it runs a small desktop that you open in your web browser. The desktop
uses noVNC, a VNC client (a remote-screen viewer) that runs in a web page.

1. Open the desktop: in VS Code, open the *Ports* panel, find port 6080, and
   click the globe icon ("Open in Browser") next to it. This works the same
   on your computer and in a codespace. On your computer the address is
   usually <http://localhost:6080>; VS Code picks another port if 6080 is
   already in use. The devcontainer CLI does not forward ports by itself.
   If you use it, add `"appPort": ["127.0.0.1:6080:6080"]` to
   `.devcontainer/devcontainer.json` on your computer (this publishes the
   port on your computer only, not on your network), then open
   <http://localhost:6080>. The password below is public, so never publish
   port 6080 on other addresses, and never make it *Public* in the
   Codespaces *Ports* panel.
2. Click *Connect* and enter the password `vscode`.
3. In the VS Code terminal (which runs inside the container), start the game
   as usual, for example:

   ```text
   cd capstones/game
   cargo run --release
   ```

   The container already tells programs to draw on that desktop (it sets the
   `DISPLAY` variable), so the window appears in the browser tab.

The picture travels through the network as a stream of screen updates, so
the game can feel slow or uneven, especially in a codespace. For playing and
for judging the frame rate, run the game natively on your own computer
(macOS and Windows need nothing extra). The headless mode and all tests work
the same in the container.

## How to work

1. Read SPEC.md from start to end once. Then read CONCEPTS.md.
2. Run `cargo test --no-default-features --test m1_raster`. Every test
   fails with `not yet implemented`. That is the starting line.
3. Work through the milestones in order (SPEC section 17). Each has its
   own test file:

   ```text
   cargo test --no-default-features --test m1_raster
   cargo test --no-default-features --test m2_loop
   cargo test --no-default-features --test m3_world
   cargo test --no-default-features --test m4_gameplay
   cargo test --no-default-features --test m5_feel
   cargo test --no-default-features --test m6_replay
   cargo test --no-default-features --test m6_alloc
   ```

4. Open the window as early as M2, even if it only shows a rectangle that
   moves with the arrow keys. From then on, play your game often. Tests
   tell you that the rules are right; only playing tells you whether it
   feels right.
5. The facade in `src/lib.rs` is the contract with the tests. Keep the
   names and signatures. Split the code into modules as soon as the file
   becomes long; the starter is one file only because it is a starter.

Things that help:

- When a simulation test fails, write the level of that test into a file
  and look at it in your window with `--level`.
- When `tests/m6_replay.rs` reports that a run "leaves the reference at
  tick N", run the earlier milestone tests again first. The message shows
  the expected and actual position, score and lives.
- If a test program stops with `TIMEOUT: the test ... is still running`,
  your code is in an endless loop (the tests give up after 60 seconds
  instead of hanging). The message names the test.
- Keep `cargo clippy --all-targets` clean as you go.

## Running and playing

```text
cargo run --release                         # play the first level
cargo run --release -- --level my.txt       # play your own level
cargo run --release -- --scale 2            # a smaller window
cargo run --release -- --record run.replay  # save your inputs on exit
cargo run --release -- --replay assets/demo.replay   # watch a recorded run
```

Keys: left and right arrow (or A and D) to run; space, up arrow, W or Z to
jump (hold for a higher jump); R to start again; Escape to quit.

Debug builds are playable too: `Cargo.toml` sets `opt-level = 1` for the
`dev` profile, because a software renderer touches every pixel of every
frame.

Levels are plain text (SPEC 8.1). Draw your own:

```text
#..............#
#..o.......o...#
#.###.....###..#
#.....o........#
#P...###....wE.#
######~~~#######
```

## Screenshots and headless runs

The headless mode runs a replay without a window. It prints one line per
simulation step and can save pictures:

```text
cargo run --release --no-default-features -- --headless \
    --replay assets/demo.replay --shots shots --shot-at 1,216,496,930
```

This writes `shots/frame_000216.ppm` and so on. PPM is a very simple image
format that many viewers open directly (GIMP, IrfanView, feh, the `sips`
tool on macOS, ImageMagick's `convert` or `magick`). To get a PNG:

```text
magick shots/frame_000216.ppm -scale 400% shot.png      # ImageMagick
sips -s format png shots/frame_000216.ppm --out shot.png   # macOS
```

In the reference solution F2 saves a screenshot of the current frame while
you play.

The lines printed by a headless run are a fingerprint of the whole game:
a hash of the state and a checksum of the picture after every step. Run it
in a debug and in a release build and compare:

```text
cargo run --no-default-features -- --headless --replay assets/demo.replay --out debug.txt
cargo run --release --no-default-features -- --headless --replay assets/demo.replay --out release.txt
```

The two files must be identical. If you have a second machine with another
operating system, compare across machines too.

## How to assess your work

1. `cargo test --no-default-features` and `cargo test` both pass, in debug
   and with `--release`.
2. `cargo clippy --all-targets` and
   `cargo clippy --all-targets --no-default-features` print no warnings.
3. The debug and release fingerprints above are identical.
4. `grep -rn "f32\|f64" src/` finds nothing, and `grep -rn "unsafe" src/`
   finds at most a `#![forbid(unsafe_code)]` line.
5. You have played your game to the end of the level, and someone else has
   played it and understood what to do without being told.
6. Go through [RUBRIC.md](RUBRIC.md). It covers what tests cannot see:
   the structure of the code, the entity store, the separation between
   simulation and presentation, and how the game feels.
7. Compare with the model solution and read its `DESIGN.md`. Different is
   fine. Look for decisions you made without noticing that you were making
   one.

## Licence

The code, the specification, the font, the sprites, the level and the
replays in this package are original work for this course and are covered
by the MIT licence of the course repository. There are no third-party
assets. The game's name, characters and rules are original; do not add
names, characters, levels, art or sounds from existing games.
