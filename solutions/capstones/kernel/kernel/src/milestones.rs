//! The code that produces the transcript of SPEC 4, one function per
//! milestone. Each function uses the kernel the way a real subsystem would
//! and prints a line only after the thing the line claims has happened.

use alloc::boxed::Box;
use alloc::format;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::mem::MaybeUninit;

use kcore::{Bump, Message};

use crate::cpu::{self, IrqGuard};
use crate::drivers::Timer;
use crate::ipc::{self, EndpointId};
use crate::{board, exception, heap, task};

// ---------------------------------------------------------------------------
// M2: drivers behind traits
// ---------------------------------------------------------------------------

/// Busy-waits for `ms` milliseconds using any timer driver.
fn delay_ms(timer: &dyn Timer, ms: u64) {
    let ticks = timer.frequency_hz() * ms / 1000;
    let start = timer.now();
    while timer.now().wrapping_sub(start) < ticks {
        core::hint::spin_loop();
    }
}

pub fn m2_drivers() {
    let timer = board::timer();
    println!("[m2] console: {}", board::console().name());
    println!("[m2] timer: {}", timer.name());
    println!("[m2] fmt: {} {:#x} {}", 42, 42, "kernel");

    let before = timer.now();
    delay_ms(timer, 10);
    let elapsed = timer.now() - before;
    assert!(
        elapsed >= timer.frequency_hz() / 100,
        "the counter must advance by at least 10 ms during the delay"
    );
    println!("[m2] delay: 10 ms");
}

// ---------------------------------------------------------------------------
// M3: the heap
// ---------------------------------------------------------------------------

pub fn m3_heap() {
    heap::init();

    // The bump allocator from `kcore`, run on a scratch region that is
    // itself a block of the kernel heap.
    let mut scratch: Box<[MaybeUninit<u8>]> = Box::new_uninit_slice(4096);
    let mut bump = Bump::new();
    // SAFETY: `scratch` is a live, exclusively owned allocation of exactly
    // `scratch.len()` bytes. It is not touched through `scratch` again
    // while `bump` is in use, and `bump` is not used after this function.
    unsafe { bump.init(scratch.as_mut_ptr().cast(), scratch.len()) };
    let word = Layout::new::<u64>();
    let wide = Layout::from_size_align(100, 64).expect("valid layout");
    let a = bump.alloc(word);
    let b = bump.alloc(wide);
    let c = bump.alloc(word);
    assert!(
        !a.is_null() && !b.is_null() && !c.is_null(),
        "bump: three blocks fit in 4 KiB"
    );
    assert!(b.addr().is_multiple_of(64), "bump: alignment respected");
    assert!(c.addr() >= b.addr() + 100, "bump: blocks do not overlap");
    let too_big = Layout::from_size_align(8192, 8).expect("valid layout");
    assert!(
        bump.alloc(too_big).is_null(),
        "bump: exhaustion is reported as null"
    );
    // SAFETY: the three pointers come from `bump.alloc` with these layouts
    // and each is freed exactly once.
    unsafe {
        bump.dealloc(a, word);
        bump.dealloc(b, wide);
        bump.dealloc(c, word);
    }
    assert_eq!(
        bump.used(),
        0,
        "bump: freeing the last block resets the allocator"
    );
    println!("[m3] bump: ok");

    let boxed = Box::new(42_u64);
    println!("[m3] box: {boxed}");

    let numbers: Vec<u64> = (0..1000).collect();
    println!(
        "[m3] vec: {} items, sum {}",
        numbers.len(),
        numbers.iter().sum::<u64>()
    );
    drop(numbers);
    drop(boxed);

    // Allocate many blocks, free them in a scattered order and check that
    // every byte comes back. That only works if freed neighbours are merged.
    let before = heap::free_bytes();
    let mut blocks: Vec<Option<Box<[u8; 96]>>> = (0..64).map(|i| Some(Box::new([i; 96]))).collect();
    for step in [3, 2, 5, 1] {
        for slot in blocks.iter_mut().step_by(step) {
            *slot = None;
        }
    }
    drop(blocks);
    assert_eq!(
        heap::free_bytes(),
        before,
        "heap: all memory returns after the blocks are freed"
    );
    println!("[m3] reuse: ok");
}

// ---------------------------------------------------------------------------
// M4: exceptions and interrupts
// ---------------------------------------------------------------------------

/// Ticks printed by the M4 transcript.
const M4_TICKS: u64 = 3;

pub fn m4_interrupts() {
    exception::install_vectors();
    println!("[m4] vectors: installed");

    // A synchronous exception taken on purpose. The handler returns the
    // `svc` immediate in x0.
    println!("[m4] svc: {:#x}", cpu::svc::<0x42>());

    let controller = board::interrupt_controller();
    controller.init();
    controller.enable(board::TIMER_INTID);
    board::timer().start_periodic(board::TICK_HZ);
    cpu::irq_enable();

    let mut printed = 0;
    while printed < M4_TICKS {
        // If the tick arrives between the check and `wfi`, the CPU sleeps
        // one period longer and then catches up below.
        cpu::wait_for_interrupt();
        while printed < exception::ticks().min(M4_TICKS) {
            printed += 1;
            println!("[m4] tick {printed}");
        }
    }
    board::timer().stop();
}

// ---------------------------------------------------------------------------
// M5: tasks
// ---------------------------------------------------------------------------

const NAMES: [&str; 3] = ["A", "B", "C"];

fn coop_task(index: usize) {
    for round in 0..2 {
        println!("[m5] coop: {}{round}", NAMES[index]);
        task::yield_now();
    }
}

/// Never yields. It ends only after the timer has taken the CPU away from
/// it twice, so it cannot finish without pre-emption.
fn spinner_task(_index: usize) {
    while task::preemptions() < 2 {
        core::hint::spin_loop();
    }
}

pub fn m5_tasks() {
    // Cooperative: the timer is stopped, tasks switch only in `yield_now`.
    let tasks = [0, 1, 2].map(|i| task::spawn(NAMES[i], coop_task, i));
    for id in tasks {
        task::join(id);
    }
    println!("[m5] coop: joined");

    // Pre-emptive: the timer interrupt switches tasks.
    let tasks = [0, 1, 2].map(|i| task::spawn(NAMES[i], spinner_task, i));
    {
        // The boot task must be blocked in `join` before the first tick,
        // or the tick would pre-empt the boot task and the trace would
        // start with it. Masking IRQs makes "start the ticks" and "block"
        // one step; the first spinner unmasks them when it starts.
        let _irq = IrqGuard::new();
        task::set_preemption(true);
        board::timer().start_periodic(board::TICK_HZ);
        task::join(tasks[0]);
    }
    task::join(tasks[1]);
    task::join(tasks[2]);

    print!("[m5] preempt:");
    for _ in 0..6 {
        let (from, to) = task::next_trace_entry().expect("six pre-emptions were recorded");
        print!(" {from}>{to}");
    }
    println!();
    println!("[m5] preempt: joined");
}

// ---------------------------------------------------------------------------
// M6: message passing
// ---------------------------------------------------------------------------

const CONSOLE: EndpointId = EndpointId(0);
const MATH: EndpointId = EndpointId(1);

/// Reply label: the request was carried out.
const OK: u64 = 0;
/// Reply label: the server did not understand the request.
const BAD_REQUEST: u64 = 1;

/// Console request: print `words[0]` bytes of text stored in `words[1..]`.
const CONSOLE_WRITE: u64 = 1;
/// Console request: how many texts were printed so far? Reply: `words[0]`.
const CONSOLE_COUNT: u64 = 2;
/// Math requests: reply `words[0]` is `words[0] + words[1]` or `* words[1]`.
const MATH_ADD: u64 = 1;
const MATH_MUL: u64 = 2;

/// Text bytes that fit in one message.
const TEXT_MAX: usize = 24;

/// Packs `text` into a `CONSOLE_WRITE` request. The bytes are copied into
/// the message; no pointer crosses the task boundary.
fn text_message(text: &str) -> Message {
    assert!(text.len() <= TEXT_MAX, "text does not fit in one message");
    let mut bytes = [0_u8; TEXT_MAX];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    let mut words = [text.len() as u64, 0, 0, 0];
    let (chunks, _) = bytes.as_chunks::<8>();
    for (word, chunk) in words[1..].iter_mut().zip(chunks) {
        *word = u64::from_le_bytes(*chunk);
    }
    Message {
        label: CONSOLE_WRITE,
        words,
    }
}

fn console_server(_arg: usize) {
    let mut printed = 0_u64;
    loop {
        let (client, request) = ipc::receive(CONSOLE).expect("console server: receive");
        let reply = match request.label {
            CONSOLE_WRITE => {
                let mut bytes = [0_u8; TEXT_MAX];
                let (chunks, _) = bytes.as_chunks_mut::<8>();
                for (chunk, word) in chunks.iter_mut().zip(&request.words[1..]) {
                    *chunk = word.to_le_bytes();
                }
                // The length and the bytes come from another task: check both.
                let text = usize::try_from(request.words[0])
                    .ok()
                    .and_then(|len| bytes.get(..len))
                    .and_then(|bytes| core::str::from_utf8(bytes).ok());
                match text {
                    Some(text) => {
                        println!("[m6] {text}");
                        printed += 1;
                        Message {
                            label: OK,
                            words: [text.len() as u64, 0, 0, 0],
                        }
                    }
                    None => Message {
                        label: BAD_REQUEST,
                        ..Message::default()
                    },
                }
            }
            CONSOLE_COUNT => Message {
                label: OK,
                words: [printed, 0, 0, 0],
            },
            _ => Message {
                label: BAD_REQUEST,
                ..Message::default()
            },
        };
        ipc::reply(CONSOLE, client, reply).expect("console server: reply");
    }
}

fn math_server(_arg: usize) {
    loop {
        let (client, request) = ipc::receive(MATH).expect("math server: receive");
        let [a, b, ..] = request.words;
        let reply = match request.label {
            MATH_ADD => Message {
                label: OK,
                words: [a.wrapping_add(b), 0, 0, 0],
            },
            MATH_MUL => Message {
                label: OK,
                words: [a.wrapping_mul(b), 0, 0, 0],
            },
            _ => Message {
                label: BAD_REQUEST,
                ..Message::default()
            },
        };
        ipc::reply(MATH, client, reply).expect("math server: reply");
    }
}

/// Asks a server and insists on an `OK` reply.
fn call(endpoint: EndpointId, request: Message) -> Message {
    let reply = ipc::send(endpoint, request).expect("client: send");
    assert_eq!(reply.label, OK, "the server rejected the request");
    reply
}

/// Never touches the UART: it computes through the math server and prints
/// through the console server.
fn client_task(_arg: usize) {
    for (label, symbol, a, b) in [(MATH_ADD, '+', 2, 3), (MATH_MUL, '*', 6, 7)] {
        let result = call(
            MATH,
            Message {
                label,
                words: [a, b, 0, 0],
            },
        )
        .words[0];
        call(
            CONSOLE,
            text_message(&format!("math: {a} {symbol} {b} = {result}")),
        );
    }
    call(CONSOLE, text_message("client: bye"));
}

pub fn m6_ipc() {
    // Timer ticks are still on from M5, so all of this runs pre-emptively.
    task::spawn("console", console_server, 0);
    task::spawn("math", math_server, 0);
    let client = task::spawn("client", client_task, 0);
    task::join(client);

    // The boot task is a client too.
    let count = call(
        CONSOLE,
        Message {
            label: CONSOLE_COUNT,
            ..Message::default()
        },
    )
    .words[0];
    println!("[m6] ipc: {count} console messages");
}
