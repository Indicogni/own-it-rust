# Capstone "kernel": rubric

Green tests show that your kernel prints the right lines on one emulator.
They do not show that it is sound, that it would survive a second CPU model,
or that somebody else could maintain it. This rubric covers the rest. Use it
to review your own work, or give it to a reviewer together with your code and
your `EVIDENCE.md` (SPEC section 9).

## How to score

Each criterion gets 0 to 3 points.

| Points | Meaning |
| --- | --- |
| 0 | Missing, or wrong in a way that matters. |
| 1 | Present, but with a gap that you can name. |
| 2 | Solid. A reviewer finds only small things. |
| 3 | You could defend every line, and you have evidence. |

There are 12 criteria, so 36 points.

| Total | Result |
| --- | --- |
| 30 to 36, and no 0 anywhere | Mastery. This is work you can show. |
| 22 to 29, and no 0 in criteria 1 to 4 | Pass. Note what you would fix next. |
| below 22, or a 0 in criteria 1 to 4 | Not yet. Fix the lowest criteria first. |

## The criteria

### 1. The acceptance tests pass

| Points | What a reviewer sees |
| --- | --- |
| 0 | Fewer than four milestones pass. |
| 1 | M1 to M4 pass in debug and release. |
| 2 | All QEMU tests and all host tests pass in debug and release. |
| 3 | The same, three runs in a row in each profile, plus `cargo +nightly-2026-11-20 miri test`; the outputs are in `EVIDENCE.md` with the QEMU version. |

### 2. Unsafe code is small, placed and justified

Count your `unsafe` blocks and read each `SAFETY` comment as a stranger would.

| Points | What a reviewer sees |
| --- | --- |
| 0 | `unsafe` blocks without comments, or comments that state no fact ("this is safe"). |
| 1 | Every block has a comment, but some comments describe *what* the code does and not *why it is sound*. |
| 2 | Every comment names the facts it relies on (IRQs masked, single core, exclusive ownership of a device, a contract of the caller). Every `unsafe fn` has a `# Safety` section. `unsafe_op_in_unsafe_fn` is denied. |
| 3 | The same, and unsafe code appears only where CONCEPTS topic 10 says it is necessary. The scheduler policy, the IPC state machine, the servers and the interrupt dispatch logic contain none. The inventory in `EVIDENCE.md` matches the code. |

### 3. Device access is volatile and reference-free

| Points | What a reviewer sees |
| --- | --- |
| 0 | Plain `*ptr` reads or writes of device registers, or `&mut` references to a register block. |
| 1 | Volatile access, but offsets are bare numbers spread over the driver. |
| 2 | A typed register block or typed accessors; all access through raw pointers with `read_volatile` and `write_volatile`. |
| 3 | The same, the layout is checked at compile time against the data sheet offsets, and register values are computed by tested functions in `kcore::reg`. |

### 4. The boot path is correct, not only lucky

The tests cannot see most of this, because QEMU gives a fresh machine a
friendly state.

| Points | What a reviewer sees |
| --- | --- |
| 0 | `.bss` is not cleared, or floating-point access is never enabled, or the stack is not 16-byte aligned. |
| 1 | All three are done, but the linker script relies on section order by accident (no `KEEP`, no explicit entry, symbols without alignment). |
| 2 | Entry code and linker script agree on every symbol and alignment; the clearing loop cannot write past `.bss`; the stack is outside `.bss`. |
| 3 | The same, and each instruction of the entry code has a reason in a comment. The kernel checks the exception level it started at instead of assuming it. |

### 5. The hardware boundary

| Points | What a reviewer sees |
| --- | --- |
| 0 | Device addresses or register writes outside the drivers. |
| 1 | Drivers exist, but the rest of the kernel names concrete driver types. |
| 2 | The kernel proper uses only the traits; one file binds drivers to addresses and interrupt numbers. |
| 3 | The same, and you can say in two sentences which files a port to another board would touch. The traits contain nothing that only one device can do. |

### 6. Interrupt safety

| Points | What a reviewer sees |
| --- | --- |
| 0 | State shared between tasks and handlers without protection, or `static mut` for such state. |
| 1 | Protection exists, but critical sections unmask interrupts on exit instead of restoring the previous state, or a lock can be taken twice on one path without detection. |
| 2 | One clearly defined mechanism (for example a lock that masks interrupts) with a written soundness argument; critical sections restore the earlier mask; printing a line is atomic. |
| 3 | The same, and for each shared item you can name who touches it (tasks, which handler) and why the chosen protection is enough. Critical sections are short; no allocation or printing inside the scheduler's lock unless you can justify it. |

### 7. Exception entry and exit

| Points | What a reviewer sees |
| --- | --- |
| 0 | Fewer than 16 vector entries, or unexpected exceptions vanish silently. |
| 1 | All entries exist, but the trap frame omits registers that handler code may change (check the SIMD registers), or the frame size is not a multiple of 16. |
| 2 | Complete frame, matching struct and assembly (checked by constant assertions), unexpected exceptions report cause and address through the panic path. |
| 3 | The same, and you can explain for `svc` and for an abort where `ELR_EL1` points and why. The handler completes an interrupt at the controller before any task switch. |

### 8. The allocator

| Points | What a reviewer sees |
| --- | --- |
| 0 | The heap tests fail, or Miri reports an error. |
| 1 | Tests pass, but invariants are not written down, or the kernel's `GlobalAlloc` wrapper does not keep interrupt handlers out. |
| 2 | Invariants of the free list are stated in comments (block alignment, minimum size, ordering), every pointer keeps the provenance of the region, the wrapper is interrupt-safe. |
| 3 | The same, and you measured something: for example the largest block after the random workload, or the number of list nodes visited per allocation, and you can say what you would change for a heap a thousand times larger. |

### 9. Tasks and the context switch

| Points | What a reviewer sees |
| --- | --- |
| 0 | The switch saves fewer registers than the calling convention requires. |
| 1 | The switch is correct, but task stacks leak, or a new task inherits whatever interrupt mask its creator had. |
| 2 | Callee-saved general and floating-point registers are switched; new tasks start in a defined state; `join` frees the stack; what is held across a switch is documented. |
| 3 | The same, and "check, then block" sequences cannot lose a wake-up: you can point to the line that makes each one atomic. The pre-emption trace is deterministic by construction, not by luck. |

### 10. Message passing

| Points | What a reviewer sees |
| --- | --- |
| 0 | Tasks call each other's functions or share data instead of sending messages. |
| 1 | Messages work, but a server trusts lengths or labels from the client, or a pointer travels in a message. |
| 2 | Requests are copied and validated; unknown requests get an error reply; the client never touches the UART. |
| 3 | The same, and the protocol of each server (labels, meaning of each word, replies) is written down in one place. You can say what happens when two clients call at once and where that is tested. |

### 11. Tests of your own

The facade tests are given. This criterion is about what *you* added.

| Points | What a reviewer sees |
| --- | --- |
| 0 | Nothing beyond the given tests. |
| 1 | A few unit tests in `kcore`. |
| 2 | Unit tests for the internals you designed (for example the free-list invariants after every operation), run under Miri. |
| 3 | The same, plus at least one test that you wrote *before* fixing a bug it catches, with a note in `EVIDENCE.md` about the bug. |

### 12. Readability and notes

| Points | What a reviewer sees |
| --- | --- |
| 0 | One large file, or names that need the author to explain them. |
| 1 | Reasonable modules, but no overview; a reader must find the boot order by searching. |
| 2 | Each module starts with a comment that says what it is for. Clippy is clean on the host and on the target. Magic numbers have names and a source. |
| 3 | The same, and your design notes (SPEC 9, item 5) name one real trade-off and one thing you would do differently. |

## What the tests cannot see

Check these by reading your code. Each has caught experienced people.

- `.bss` is really cleared (QEMU's RAM starts as zero anyway).
- Every register access is volatile (a debug build hides the difference).
- The trap frame includes the floating-point state.
- The interrupt is completed at the controller before a task switch.
- A critical section restores the previous interrupt mask.
- The boot task is blocked before the first tick of the pre-emptive demo
  (a kernel with this race passes most runs).
- No lock is held across a context switch.
- The console server checks the length it receives.
- The panic handler works from inside a critical section.
- Nothing depends on the value of `CNTFRQ_EL0` being the one QEMU reports
  for one CPU model. Try `-cpu max` by hand: the transcript must be the same.

## Questions to answer without looking at the code

A reviewer may ask these. If you can answer them aloud, you have understood
your kernel.

1. What are the first five things that happen after QEMU jumps to your entry
   point, and what breaks if you swap any two of them?
2. A timer interrupt arrives while a task is in the middle of `println!`.
   Follow the CPU from that instruction to the next instruction of the same
   task. Which stack is used at each step?
3. Why does the context switch save so few registers, and why does the
   exception entry save so many?
4. Where is the stack of a task freed, by whom, and why not earlier?
5. A client sends a request and the timer fires before the client blocks.
   Why is no wake-up lost in your kernel?
6. Which of your `unsafe impl Sync` would become wrong on a machine with two
   cores, and what would you add?
7. What would have to change to run the client of M6 at EL0?
