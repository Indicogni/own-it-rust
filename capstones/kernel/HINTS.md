# Capstone "kernel": design hints

These are hints about design and about where to look. They contain no code.
Each milestone has two levels. Open the first when you do not know how to
start; open the second when you have tried and are stuck. If a hint does not
help, read the matching topic in [CONCEPTS.md](CONCEPTS.md) again before you
open the reference solution.

A general hint first: **make the machine talk as early as possible.** Until
the UART prints, every bug looks the same (silence). Get one character out
with the crudest code that works, then build the real driver, and keep
`-d int` (README, "Debugging") at hand.

## M1 · Boot

<details>
<summary>Hint 1: the order of work</summary>

Work in this order, and run QEMU after each step:

1. A linker script that puts the image into RAM and names the entry symbol.
   Check the result with `llvm-objdump -h` before you run anything.
2. Entry code that only sets the stack pointer and calls a Rust function.
3. One byte written to the UART data register from that function.
4. Power off through PSCI. From now on the tests can tell you things.
5. Only then: clearing `.bss`, enabling floating-point access, the typed
   register block, UART initialisation, the panic handler.

For the path of the linker script to the linker you have two choices: a build
script that prints a `rustc-link-arg` line, or `rustflags` in a Cargo
configuration file. Think about which one still works when Cargo is started
from another directory; the tests start it inside `kernel/`.

</details>

<details>
<summary>Hint 2: things that go wrong here</summary>

- Symbols from the linker script have an address but no content. Declare
  them as external statics of any type and use only their address.
- The entry code can clear `.bss` in steps of 8 bytes only if the linker
  script aligns both ends of the section. Decide the alignment in the script
  and state it in a comment in the entry code.
- Do not put the stack inside `.bss`, or make sure the clearing loop does not
  run on the stack it is clearing.
- The check for `[m1] bss: zeroed` has to run before anything writes to a
  static. Initialising the UART writes to device registers, not to statics,
  but a "console is ready" flag would be a static.
- Enable floating-point access in the entry code, before the first Rust
  instruction. A release build uses SIMD registers earlier than you think.
- For the baud rate, write and test `kcore::reg::pl011_baud_divisors` first.
  QEMU's UART does not care about the baud rate; a real board does.
- A panic handler that prints needs the UART, and the UART might be the thing
  that is broken. Keep the handler as simple as you can, and make sure it
  cannot recurse.

</details>

## M2 · Drivers behind traits

<details>
<summary>Hint 1: where to cut</summary>

Ask of every function: does it need to know *which* device it talks to? If
not, it takes a trait object. Three places should fall out:

- the drivers (know registers, implement traits);
- one "board" module (knows addresses, owns the driver instances, hands out
  `&'static dyn Trait`);
- everything else (knows only traits).

The drivers live in `static`s, so their methods take `&self`, and the driver
types must be `Sync`. You will have to write an `unsafe impl Sync` somewhere.
Decide where: on each driver, or once on the type that wraps a register
address. Write down what you are promising.

</details>

<details>
<summary>Hint 2: the print macros and the delay</summary>

- `print!` needs something that implements `core::fmt::Write`. It can be a
  zero-sized adapter around the console trait object.
- `core::fmt` calls `write_str` once per piece of a formatted line. If a line
  must not be torn apart (SPEC 3.2), the protection has to cover the whole
  formatting call, not a single `write_str`.
- Build `println!` so that the text and the line end go out in one call.
- Macros defined with `macro_rules!` in a binary crate are visible only in
  modules declared after the macro definition, unless you re-export them.
- For the delay, compute the number of counter ticks from the frequency that
  the timer driver reports. Compare differences of counter values, not
  absolute values.

</details>

## M3 · Memory

<details>
<summary>Hint 1: two allocators, one wrapper</summary>

Write both allocators in `kcore` against the host tests first, and run Miri
early. Miri's messages are much better than a kernel that hangs.

For the free list, decide three things before you write code:

1. What is stored in a free block, and so what is the smallest block?
2. Is the list sorted? Sorting by address makes one operation slower and
   another much simpler. Which ones?
3. What happens to the gap in front of a block with a large alignment?

In the kernel, the `#[global_allocator]` is a thin wrapper: a lock around the
`kcore` allocator, plus the two addresses from the linker script.

</details>

<details>
<summary>Hint 2: things that go wrong here</summary>

- If every block starts at a multiple of the minimum block size and has a
  size that is a multiple of it, then every gap that splitting leaves is
  either empty or large enough to be a free block. That removes a whole class
  of special cases.
- `dealloc` receives the layout. Round it exactly as `alloc` did, or the
  accounting drifts and `free_bytes()` never returns to its start value.
- Merging has two directions: with the block after and with the block
  before. The three "merges" host tests each need a different one, or both.
- Never turn an address back into a pointer with `as`. Derive every pointer
  from the region pointer (`with_addr`, `add`), or Miri will object.
- That includes `dealloc`. The pointer you are handed there may be valid for
  fewer bytes than the block you take back: `Box` and `Vec` pass a pointer
  that went through a reference to their content. Take its address and build
  your own pointer from the region pointer.
- A pointer to "the link that points at the current node" (the head field
  first, then each node's next field) removes the special case for the head
  of the list.
- The lock around the heap must keep interrupt handlers out, and it should
  detect being taken twice on the same path. A panic message that allocates
  inside the allocator lock would be such a path.

</details>

## M4 · Exceptions and interrupts

<details>
<summary>Hint 1: one step at a time</summary>

1. First make exceptions visible: a table whose every entry ends in a report
   of the cause and the return address (CONCEPTS topic 6). Trigger one on
   purpose.
2. Then make one synchronous exception return: save a full frame, call Rust,
   restore, `eret`. `svc` is the easiest to return from. Check that code
   after the `svc` still sees its registers.
3. Only then interrupts: controller on, timer on, the CPU's mask off. If
   nothing arrives, go through the four places in CONCEPTS topic 7.

Define the trap frame once as a `repr(C)` struct and derive every offset in
the assembly from it; a constant assertion per field keeps the two in step.

</details>

<details>
<summary>Hint 2: things that go wrong here</summary>

- A vector entry is 32 instructions. Saving all registers does not fit. Let
  each entry do the minimum and branch to shared code, passing along which
  entry it was.
- The compiler may use SIMD registers inside your handler. Either they are in
  the frame, or you can prove that the handler and everything it calls never
  touches them. The first is easier.
- Keep the frame size a multiple of 16.
- The timer line is a level. Silence it at the device in the handler (move
  the compare value or disable the timer) before you complete the interrupt.
- "Re-arm relative to now" and "re-arm relative to the last deadline" differ
  when the emulator stalls: one may lose time, the other may fire several
  times in a row. Either is acceptable here; know which one you chose.
- The handler should do little: count the tick, complete the interrupt.
  Printing belongs to the code that waits for the tick (SPEC, M4 lines).

</details>

## M5 · Tasks

<details>
<summary>Hint 1: policy and mechanism</summary>

Finish `kcore::Scheduler` against the host tests first. The test
`milestone_5_cooperative_order` replays the cooperative demo on the bare
state machine. When it passes, the remaining work is mechanism: one saved
context and one stack per task, and one function that switches.

Every operation in the kernel (`yield`, `block`, `exit`, the tick) then has
the same shape: with interrupts masked, ask the scheduler who is next; if it
names a task, switch from the current context to that one.

Treat the boot code as task 0. It already has a stack; it just needs a
context slot to be saved into.

</details>

<details>
<summary>Hint 2: things that go wrong here</summary>

- A new task has never been switched away from. Build its first context by
  hand so that the switch "returns" into a start routine. The start routine
  must put the task into the interrupt state you want; it will inherit
  "masked" from whoever switched to it.
- Arguments for the start routine have to travel in registers that the
  switch restores.
- Do not hold your scheduler lock across the switch. Take what you need out
  of it (two pointers), leave it, and switch with interrupts still masked.
- `join` is "check whether the task has exited, otherwise block". Between
  the check and the block nothing may run that could make the check stale.
- A task cannot free its own stack. The joiner can.
- For the pre-emptive demo, think about what the trace would show if a tick
  arrived while the boot task was still creating tasks. Make "enable ticks"
  and "block the boot task" a single step.
- Pre-emption is a switch called from the interrupt handler. Complete the
  interrupt at the controller first.
- The trace is state shared with an interrupt handler, and the handler must
  not print. A `kcore::RingBuffer` filled by the handler and drained by the
  boot task fits.

</details>

## M6 · Message passing

<details>
<summary>Hint 1: the state machine first</summary>

`kcore::Endpoint` is pure bookkeeping. Draw the stages of one call (queued,
awaiting reply, replied) and note which method moves a call from one stage to
the next. A single array of call slots with a stage in each is enough;
remember that requests must come out in the order they went in, also after a
slot in the middle was freed and reused.

The kernel layer is then three short functions with the same pattern as
`join`: ask the state machine; if it says "nothing yet", block; when woken,
ask again.

</details>

<details>
<summary>Hint 2: things that go wrong here</summary>

- In `send`, the request is queued (which may wake the server) before the
  client blocks. Close the gap between the two, or a fast server replies to a
  client that is not blocked yet and the wake-up is lost.
- A woken task should not assume that its wait is over. Let it ask the state
  machine again in a loop.
- Design the protocol on paper: for each server, each label, the meaning of
  each word of the request and of the reply, and the error reply.
- The console server gets its text as bytes inside the message words. It must
  check the length before it slices and the bytes before it treats them as
  text.
- The demo is deterministic because exactly one task is able to print each
  line, and each line is printed before the reply that lets the next step
  start. If your output order changes between runs, look for a line that is
  printed *after* a reply.
- The servers never exit. That is fine: at the end they are blocked, and the
  boot task powers the machine off.

</details>

## If the tests fail only sometimes

<details>
<summary>Hint</summary>

Then something depends on when an interrupt arrives. Typical causes: a
"check, then block" that is not atomic; a line printed by two calls instead
of one; the boot task not yet blocked when pre-emption starts; a tick that
is acknowledged after a task switch. Run the QEMU test in a loop while your
computer is busy with something else; a race that shows once in fifty runs
on a quiet machine often shows every few runs under load.

</details>
