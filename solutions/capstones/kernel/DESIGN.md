# Reference kernel: design notes

This is the model answer for the capstone in `capstones/kernel/`. The files
in this directory mirror that directory: copy them over a fresh copy of the
capstone and every acceptance test passes.

It is one possible design. It is small on purpose: about 590 lines of code in
`kcore`, 1300 in the kernel binary, and 47 `unsafe` blocks (6 of them in a
unit test), each with its reason next to it. Where a decision was a
trade-off, this file says what was given up.

## Reading order

| File | Role |
| --- | --- |
| `kernel/link.ld` | Where everything goes in RAM. |
| `kernel/build.rs` | Passes the linker script to the linker. |
| `kernel/src/boot.rs` | The first instructions: stack, floating point, `.bss`, call `kmain`. |
| `kernel/src/main.rs` | `kmain`: runs the six milestones in order; the panic handler. |
| `kernel/src/cpu.rs` | One small safe function per special instruction. |
| `kernel/src/mmio.rs` | Typed, volatile register access. |
| `kernel/src/drivers/` | The traits (`mod.rs`) and three drivers. |
| `kernel/src/board.rs` | The only file with addresses and interrupt numbers. |
| `kernel/src/console.rs` | `print!` and `println!`. |
| `kernel/src/sync.rs` | `IrqLock`, the one locking primitive. |
| `kernel/src/heap.rs` | `#[global_allocator]` around `kcore::FreeList`. |
| `kernel/src/exception.rs` | Vector table, trap frame, handlers. |
| `kernel/src/task.rs` | Contexts, stacks, the switch, `spawn`/`yield`/`join`. |
| `kernel/src/ipc.rs` | `send`/`receive`/`reply` on top of `kcore::Endpoint`. |
| `kernel/src/milestones.rs` | The code that produces the transcript. |
| `kcore/src/*.rs` | Ring buffer, allocators, scheduler, endpoint, register arithmetic. |

## Memory layout

```text
0x4000_0000  (1 MiB left free; QEMU puts the device tree blob here)
0x4010_0000  .text      entry code first; the vector table is inside, 2 KiB aligned
             .rodata
             .data
             .bss       cleared by the entry code, 16-byte aligned at both ends
             .stack     64 KiB boot stack (becomes the stack of task 0)
             heap       4 MiB, between __heap_start and __heap_end
0x4800_0000  end of RAM
```

The stack is a section of its own and not part of `.bss`, so the entry code
does not clear the stack it is about to use, and boot time is not spent on
zeroing 64 KiB that nobody reads. Task stacks (64 KiB each) come from the
heap.

The linker script reaches the linker through `build.rs`
(`cargo::rustc-link-arg-bins`) with an absolute path. `rustflags` in
`.cargo/config.toml` would also work, but only when Cargo is started below
that directory, and it would replace any `RUSTFLAGS` the user has set.
`.cargo/config.toml` is still there for convenience: it makes the bare-metal
target the default inside `kernel/` and lets `cargo run` start QEMU.

## Boot

`_start` does five things, in an order that matters:

1. masks all interrupts (QEMU does this already; a real boot loader may not);
2. parks the CPU unless it is at EL1, because nothing later would work;
3. sets `CPACR_EL1.FPEN`, so that code may use SIMD registers. This has to
   come before the first compiled instruction;
4. sets the stack pointer;
5. clears `.bss` and calls `kmain`.

`kmain` first checks that `.bss` is zero (before any static is written),
then initialises the UART and prints.

## Drivers and the hardware boundary

`drivers/mod.rs` defines three traits: `Console`, `Timer` and
`InterruptController`. `board.rs` owns one static instance of each driver and
hands out `&'static dyn Trait`. No other file names a driver type, an
address or an interrupt number. A port to another board replaces `board.rs`,
the drivers, `boot.rs`, the assembly in `exception.rs` and `task.rs`, and
`cpu.rs`; `kcore`, `ipc.rs`, `heap.rs`, `console.rs` and `milestones.rs`
stay.

Register access goes through `Mmio<R>`, which stores the address of a
`#[repr(C)]` register block, and two macros. `read_reg!(self.regs, fr)`
expands to a volatile read through `&raw const (*block).fr`. No reference
to device memory is ever created. Constant assertions with `offset_of!`
compare each block with the offsets in the data sheet.

The single `unsafe impl Sync` for device access is on `Mmio`. Its argument:
an `Mmio` is an address, and volatile access through raw pointers cannot
break Rust's aliasing rules whoever does it. Whether the device tolerates
interleaved access is a separate, per-driver question. For the UART the
answer is that a byte written from an interrupt handler in the middle of a
task's line would be harmless to the device and is prevented anyway, because
`print!` masks IRQs for the whole formatted message.

Register values are computed in `kcore::reg` (`pl011_baud_divisors`,
`set_field`, `gic_bit_slot`, ...), so the bit arithmetic is covered by host
tests and the drivers only perform the writes.

Trade-off: the console is found through a function (`board::console()`),
not through a registration call at run time. That keeps `print!` free of
locks and of "is a console registered yet?" states, at the price that the
choice of console is made at compile time.

## One locking primitive

The kernel runs on one core. The only thing that can interleave with running
code is an IRQ, which may also switch tasks. `IrqLock<T>` therefore gives
exclusive access by masking IRQs around a closure. It restores the previous
mask afterwards, so locks nest, and it carries a flag that turns a second
`lock` on the same lock from the same path into a panic instead of two
`&mut T`.

Everything shared between tasks and interrupt handlers is either an atomic
(the tick counter, the timer interval, the panic flag) or sits in an
`IrqLock` (the heap, the task table, the endpoint table). There is no
`static mut` in the kernel.

`IrqLock` would be wrong on two cores. Its `unsafe impl Sync` says so, and
that comment is the place to start a multi-core port.

## The heap

`kcore::FreeList` is a first-fit allocator over a singly linked list of free
blocks, sorted by address. Every block starts at a multiple of 16 bytes and
has a size that is a multiple of 16, so each gap that splitting leaves is
either empty or large enough for a list node. The node lives inside the free
block; allocated blocks carry no header, because `dealloc` receives the
layout. Sorting by address makes `dealloc` a walk through the list, and
makes merging a look at the two neighbours.

The allocator keeps the pointer that `init` received and derives every
pointer it stores or returns from that one. `dealloc` takes only the
*address* of the pointer it is given. A block comes back from `Box` or `Vec`
through a pointer that went through a reference to the content, and such a
pointer may access `layout.size()` bytes only. The list node, the rounded
block and a later merged block are larger. The host test
`free_list_uses_only_the_address_of_a_freed_pointer` checks this under Miri.

`kernel/src/heap.rs` adds the two things the algorithm must not know: the
region from the linker script and an `IrqLock`. `heap::init` refuses a second
call, which would hand out memory that live blocks still use.

## Exceptions

`cpu::set_vector_table` is the only code that writes `VBAR_EL1`, and it sets
a flag. `cpu::irq_enable` and `cpu::svc` check the flag and panic without it.
So safe code cannot send the CPU through a vector table that is not there.

All sixteen vector entries share one path. An entry makes room for the trap
frame, stores x0 and x1, loads its own number into x0 and branches to
`__trap_entry`, which saves the rest: x2 to x30, `ELR_EL1`, `SPSR_EL1`, FPCR,
FPSR and q0 to q31. It then calls `trap_handler(vector, &mut TrapFrame)` and
restores everything.

The frame is 800 bytes. Most of it (512 bytes) is the SIMD state. That is
the price of the target `aarch64-unknown-none`, on which the compiler may
use SIMD registers in any function, including handlers. The alternatives
were the soft-float target (not the one this course installs) or lazy
saving on first use (more code than the rest of the exception path
together). Saving everything is the simple, obviously correct choice.

`trap_handler` handles two cases. A synchronous exception from the kernel
itself must be an SVC; the handler puts the immediate into the saved x0. An
IRQ from the kernel goes to `handle_irq`. Every other entry, and every other
synchronous cause, ends in `panic!` with the vector number, the decoded
exception class, `ESR_EL1`, `ELR_EL1` and `FAR_EL1`, so an unexpected fault
is one readable `[panic]` line.

`handle_irq` claims the interrupt, re-arms the timer, counts the tick,
completes the interrupt at the GIC and only then calls `task::on_tick()`,
which may switch tasks. The order "complete, then switch" matters: the task
that is switched to may not return here for a long time, and the GIC
delivers no new timer interrupt before the old one is completed.

The timer is re-armed relative to *now* (`CNTV_TVAL_EL0`), not relative to
the previous deadline. If the emulator stalls, ticks are lost instead of
arriving in a burst, and every task is guaranteed a full interval between
two ticks. A kernel that keeps wall-clock time would choose the other way.

## Tasks

Policy and mechanism are separate. `kcore::Scheduler` knows task ids and
states and answers "who runs next?". `task.rs` owns, per task, a saved
`Context`, a stack and a name, and performs the switch.

Every scheduling operation has the same shape, in one function
(`reschedule`): mask IRQs; inside the task-table lock ask the scheduler and
take raw pointers to the two contexts; leave the lock; call
`context_switch`. The lock is *not* held across the switch. IRQs stay
masked across it, and each task restores its own mask state when it
continues, because that state is in a guard on its own stack.

`context_switch` is a naked function that saves and loads x19 to x29, the
stack pointer and d8 to d15 (the registers that AAPCS64 makes a callee
preserve) and x30, the address at which the task continues. Pre-emption uses the same function, called from the IRQ
handler; the pre-empted task's caller-saved registers are already in its
trap frame.

A new task gets a hand-built context: x19 holds the entry function, x20 its
argument, and the return address points at `task_start`, three instructions
that move both into argument registers and jump to `task_main`. `task_main`
unmasks IRQs (a new task has no saved mask state), runs the entry function
and calls `exit`.

`exit` wakes the joiner, if there is one, and switches away for ever. `join`
is a loop of "exited? otherwise note me as joiner and block" under one IRQ
guard, which makes the check and the block atomic. The joiner frees the
stack of the exited task.

Known limits, all listed as non-goals in the SPEC: task ids are not reused
(16 tasks in total), a task that is never joined keeps its stack, there is
no idle task (if every task blocks, `block_current` panics with
"deadlock"), and a stack overflow is not detected.

### Why the pre-emption trace is deterministic

The spinner tasks do nothing but read their own pre-emption count, so the
only events are timer ticks, and each tick moves the CPU to the next task in
the queue. The boot task starts the timer and blocks in `join` under one IRQ
guard, so it cannot be pre-empted in between. From then on the first six
timer-driven switches are A>B, B>C, C>A, A>B, B>C, C>A whatever the timing,
and no task can finish before the sixth, because each needs two
pre-emptions. The handler records the switches in a `kcore::RingBuffer`; the
boot task prints them later. The transcript was also checked with QEMU's
`-icount` option at several speeds and with other CPU models.

## Message passing

`kcore::Endpoint` holds, per call, one of three stages (queued, awaiting
reply, replied) and remembers a server that waits in `receive`. Its methods
return what the kernel must do. `ipc.rs` adds the blocking: each of `send`,
`receive` and `reply` is "ask the state machine; wake whom it names; if there
is nothing yet, block and ask again", under an IRQ guard so that a wake-up
cannot fall between asking and blocking.

A woken task always asks the state machine again and does not assume that it
got what it waited for. That makes spurious wake-ups harmless and keeps the
kernel layer free of per-task "result" slots.

The demo protocol is in `milestones.rs`: labels are small integers, a
console "write" carries the length in word 0 and up to 24 bytes of text in
words 1 to 3. The console server validates the length and the UTF-8 before
it prints. The output is deterministic under pre-emption because each line
is printed by exactly one task before the reply that allows the next step.

## Unsafe inventory

| File | `unsafe` blocks | Why they are needed |
| --- | --- | --- |
| `kcore/src/heap.rs` | 15 (6 of them in the unit test) | List nodes live inside free memory; every access is a raw pointer read or write. Also 2 `unsafe impl Send`. |
| `kcore/src/ring.rs` | 1 | Reading a slot of `MaybeUninit`. |
| `kcore/src/{sched,ipc,reg}.rs` | 0 | Pure logic. |
| `kernel/src/cpu.rs` | 8 | One `asm!` per special instruction. |
| `kernel/src/drivers/generic_timer.rs` | 4 | System register access with `asm!`. |
| `kernel/src/exception.rs` | 3 | Installing the vector table; reading `ESR_EL1` and `FAR_EL1`. Plus the vector assembly. |
| `kernel/src/mmio.rs` | 2 | The volatile read and write inside the two macros. Also `unsafe impl Send`/`Sync` for `Mmio`. |
| `kernel/src/board.rs` | 2 | Constructing the UART and GIC drivers from their addresses. |
| `kernel/src/drivers/gicv2.rs`, `pl011.rs` | 2 + 1 | Forwarding the constructor contract to `Mmio::new`. |
| `kernel/src/heap.rs` | 2 | Handing the heap region to the allocator; forwarding `dealloc`. Also `unsafe impl GlobalAlloc`. |
| `kernel/src/task.rs` | 2 | The call of `context_switch`; turning the entry address back into a function pointer. Plus two naked functions. |
| `kernel/src/milestones.rs` | 2 | Driving `kcore::Bump` by hand for the M3 demo. |
| `kernel/src/sync.rs` | 1 | The `&mut T` inside `IrqLock::lock`. Also its `unsafe impl Sync`. |
| `kernel/src/boot.rs` | 1 | Reading `.bss` to check that it is zero. Plus the entry assembly. |
| `kernel/src/psci.rs` | 1 | The `hvc` call. |
| `kernel/src/{main,console,ipc}.rs`, `drivers/mod.rs` | 0 | `main.rs` has one unsafe attribute (`no_mangle`). |

Safe code that would be easy to get wrong without these boundaries (the
scheduler, the endpoint, the servers, interrupt dispatch) contains no
`unsafe` at all.

## How it was checked

On macOS (Apple silicon) with Rust 1.99.0 and QEMU 11.1.2:

- `cargo test` and `cargo test --release`, three runs each: all host tests
  and all nine QEMU tests pass.
- The kernel image was booted 60 times per profile under heavy CPU load and
  compared byte for byte with the transcript; no run differed.
- `cargo +nightly-2026-11-20 miri test`: the host tests and the unit test of the free
  list pass, also with `-Zmiri-tree-borrows` and `-Zmiri-strict-provenance`.
  A version of `FreeList::dealloc` that keeps the caller's pointer fails the
  host tests under Miri.
- `cargo clippy --all-targets` on the host and `cargo clippy` for
  `aarch64-unknown-none`: no warnings.
- 38 deliberate mistakes (for example "never merges free blocks", "wake
  ignores the task state", "switches tasks before completing the
  interrupt", "new tasks start with IRQs masked") were applied one at a
  time; each made at least one test fail.

`kcore` has one unit test of its own, in `heap.rs`: it walks the free list
after every operation of a random workload and checks the invariants that
the acceptance tests cannot see (sorted, never two touching free blocks,
sizes in whole granules, counter equal to the sum). Run it with
`cargo test -p kcore`.

## What a next version would change

- Give each exception group its own Rust entry instead of one function with
  a vector number. It becomes necessary as soon as EL0 tasks exist.
- Replace the first-fit free list by size classes in front of it. The task
  stacks and the trap frames are a few fixed sizes.
- Add an idle task, so that "nothing is ready" means `wfi` and not a panic.
  That is the first step towards sleeping and timeouts.
- Unmap a guard page under each stack once the MMU is on. Today a stack
  overflow silently overwrites the heap.
