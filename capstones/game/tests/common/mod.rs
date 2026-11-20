//! Helpers shared by the acceptance tests.
//!
//! Each file in `tests/` is its own crate, and not every one uses every
//! helper, so unused-code warnings are switched off here.
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

use game::{Events, Game, Input};

/// The first level, as shipped.
pub const LEVEL_1: &str = include_str!("../../assets/level1.txt");
/// A recorded run that wins the first level.
pub const DEMO_REPLAY: &str = include_str!("../../assets/demo.replay");
/// A recorded run that loses every life on the first level.
pub const ROUGH_REPLAY: &str = include_str!("../golden/rough.replay");

pub const RIGHT: Input = Input(Input::RIGHT);
pub const LEFT: Input = Input(Input::LEFT);
pub const JUMP: Input = Input(Input::JUMP);
pub const NONE: Input = Input::NONE;

/// One tile in subpixels.
pub const TILE: i32 = 4096;
/// One pixel in subpixels.
pub const SUB: i32 = 256;

/// FNV-1a, 64 bit, written out here so that the tests do not depend on the
/// implementation they are checking.
pub fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The checksum SPEC 3.3 defines for a slice of pixels.
pub fn pixel_checksum(pixels: &[u32]) -> u64 {
    fnv1a(pixels.iter().flat_map(|p| p.to_le_bytes()))
}

/// Builds a game or fails the test with the level in the message.
#[track_caller]
pub fn game(level: &str, seed: u64) -> Game {
    match Game::new(level, seed) {
        Ok(game) => game,
        Err(e) => panic!("Game::new rejected this level with {e:?}:\n{level}"),
    }
}

/// Steps `game` `n` times with the same input; returns all events seen,
/// or-ed together.
pub fn run(game: &mut Game, input: Input, n: usize) -> Events {
    let mut seen = 0;
    for _ in 0..n {
        seen |= game.step(input).0;
    }
    Events(seen)
}

/// Steps until `event` happens, at most `limit` times; returns the events of
/// that step. Fails the test if the event never comes.
#[track_caller]
pub fn run_until(game: &mut Game, input: Input, event: u32, limit: usize) -> Events {
    for _ in 0..limit {
        let events = game.step(input);
        if events.has(event) {
            return events;
        }
    }
    panic!(
        "event bit {event} did not happen within {limit} steps; the game is at tick {} with the \
         player at {:?}",
        game.tick(),
        game.player()
    );
}

// ---------------------------------------------------------------------------
// A watchdog, so that an endless loop fails with a message instead of
// hanging `cargo test`.
// ---------------------------------------------------------------------------

/// How long one test may run. Every test here needs a fraction of a second;
/// the limit only has to be far above that on a slow machine. Set the
/// environment variable `GAME_TEST_TIMEOUT_SECS` to change it.
fn time_limit() -> Duration {
    let secs = std::env::var("GAME_TEST_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    Duration::from_secs(secs)
}

static WATCHED: Mutex<Vec<(u64, String, Instant)>> = Mutex::new(Vec::new());
static NEXT_WATCH: AtomicU64 = AtomicU64::new(0);
static WATCHDOG: Once = Once::new();

/// Keeps an eye on the test that holds it. See [`watch`].
pub struct Watch(u64);

/// Call this first in every test and keep the result alive:
/// `let _watch = common::watch();`. If the test is still running when the
/// time limit is over, the whole test program stops with a message naming
/// the test. A stuck thread cannot be stopped from outside, so ending the
/// process is the only way to get the message onto the screen.
pub fn watch() -> Watch {
    WATCHDOG.call_once(|| {
        let limit = time_limit();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(200));
                let watched = WATCHED.lock().unwrap_or_else(|e| e.into_inner());
                if let Some((_, name, _)) = watched.iter().find(|(_, _, t)| t.elapsed() > limit) {
                    // Written straight to the stderr handle: `eprintln!` in a
                    // thread started by a test is captured by the harness and
                    // would never be shown.
                    let _ = writeln!(
                        std::io::stderr(),
                        "\nTIMEOUT: the test `{name}` is still running after {} seconds.\n\
                         That is almost always an endless (or absurdly long) loop in the code \
                         under test: for example a line that never reaches its end point, or a \
                         loop that runs once per simulation step for a huge elapsed time.\n\
                         Stopping the test program; the other tests of this file did not finish.",
                        limit.as_secs()
                    );
                    std::process::exit(101);
                }
            }
        });
    });
    let id = NEXT_WATCH.fetch_add(1, Ordering::Relaxed);
    let name = std::thread::current().name().unwrap_or("?").to_owned();
    WATCHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push((id, name, Instant::now()));
    Watch(id)
}

impl Drop for Watch {
    fn drop(&mut self) {
        WATCHED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(id, _, _)| *id != self.0);
    }
}

// ---------------------------------------------------------------------------
// A counting allocator for the allocation budget (SPEC section 16).
// ---------------------------------------------------------------------------

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Forwards to the system allocator and counts allocations made by a thread
/// while that thread is inside `count_allocations`.
pub struct Counting;

fn record() {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = COUNT.try_with(|count| count.set(count.get() + 1));
        }
    });
}

// SAFETY: every method forwards unchanged to `System`, which upholds the
// GlobalAlloc contract. `record` touches only const-initialised thread-locals
// without destructors, so it never allocates and never re-enters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: the caller upholds this method's contract; forwarded as is.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: the caller upholds this method's contract; forwarded as is.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record();
        // SAFETY: the caller upholds this method's contract; forwarded as is.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds this method's contract; forwarded as is.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Runs `f` and returns its result together with the number of allocations
/// and reallocations it made on this thread. Only meaningful in a test
/// binary that installs `Counting` as the global allocator.
pub fn count_allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = COUNT.with(Cell::get);
    let was_active = ACTIVE.with(|a| a.replace(true));
    let result = f();
    ACTIVE.with(|a| a.set(was_active));
    (result, COUNT.with(Cell::get) - before)
}
