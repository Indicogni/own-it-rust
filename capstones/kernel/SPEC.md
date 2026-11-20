# Capstone "kernel": specification

You write a small operating system kernel in Rust. It boots on the `virt`
machine of QEMU, an emulated 64-bit Arm (AArch64) computer, with no operating
system and no standard library underneath it.

This file says exactly what the kernel must do. It does not say how. The
design, the file layout and almost every name are your decision.
[CONCEPTS.md](CONCEPTS.md) teaches the background, [HINTS.md](HINTS.md) gives
design hints, and [RUBRIC.md](RUBRIC.md) says how to judge the result.

Words used in a fixed sense: **must** marks a requirement that a test or the
rubric checks. **May** marks a free choice.

Contents:

1. [What you build](#1-what-you-build)
2. [The machine](#2-the-machine)
3. [Rules for the kernel](#3-rules-for-the-kernel)
4. [Milestones and the transcript](#4-milestones-and-the-transcript)
5. [The `kcore` facade](#5-the-kcore-facade)
6. [The tests](#6-the-tests)
7. [Stretch goals](#7-stretch-goals)
8. [Non-goals](#8-non-goals)
9. [Evidence to keep](#9-evidence-to-keep)

## 1. What you build

Two crates:

| Crate | Directory | Runs on | Content |
| --- | --- | --- | --- |
| `kcore` | `kcore/` | host and bare metal | Everything that does not need hardware: a ring buffer, two allocators, the scheduling policy, the IPC state machine and register arithmetic. `#![no_std]`, no `alloc`. |
| `kernel` | `kernel/` | bare metal only | The kernel binary: entry code, linker script, drivers, exception handling, context switch. It should be thin: logic belongs in `kcore`, where it can be tested. |

The starter gives you only this:

- `kcore/src/lib.rs`: the public names and signatures of section 5 with
  `todo!()` bodies, so that the host tests compile.
- `kernel/src/main.rs`: an empty `#![no_std]` `#![no_main]` program with a
  panic handler that loops. It has no entry point and no linker script.
- `tests/` and `harness/`: the acceptance tests. Do not change them.

The work has six milestones (section 4). After each one, more tests pass.

## 2. The machine

### 2.1 The QEMU command line

Every acceptance run uses this command (one line), with the path of your
kernel image at the end:

```text
qemu-system-aarch64 -machine virt,gic-version=2 -accel tcg -cpu cortex-a72
    -smp 1 -m 128M -display none -monitor none -serial stdio -no-reboot
    -kernel <path to the ELF file>
```

| Option | Meaning |
| --- | --- |
| `-machine virt,gic-version=2` | The generic Arm virtual board, with interrupt controller version 2. |
| `-accel tcg` | Pure software emulation, the same on every host. |
| `-cpu cortex-a72` | A 64-bit Armv8-A CPU. (The default CPU of `virt` is 32-bit.) |
| `-smp 1` | One CPU core. |
| `-m 128M` | 128 MiB of RAM. |
| `-display none -monitor none` | No window and no QEMU console. |
| `-serial stdio` | The first UART is connected to QEMU's standard input and output. |
| `-no-reboot` | A reset request ends QEMU instead of restarting the guest. |
| `-kernel` | Load this file into memory and start it. |

### 2.2 The state of the CPU when your code starts

QEMU loads each loadable segment of the ELF file at the physical address in
its program header and jumps to the ELF entry address. At that moment (checked
with QEMU 11.1.2):

| Fact | Value |
| --- | --- |
| Exception level | EL1, using `SP_EL1` (`CurrentEL` reads `0x4`) |
| Interrupts | all masked (`DAIF` = `0b1111`) |
| MMU and caches | off: every address is a physical address |
| Stack pointer | not set |
| General registers | not specified; do not rely on them (in particular, x0 does not hold a device tree address) |
| Floating point and SIMD | trapped: `CPACR_EL1.FPEN` is `0b00`. Any instruction that touches a floating-point or SIMD register raises an exception until you set `FPEN` to `0b11`. The Rust compiler uses these registers in ordinary code on `aarch64-unknown-none`. |
| `.bss` | not specified; your entry code must clear it |
| Unaligned access | With the MMU off the CPU treats all memory like device memory: a load or store whose address is not a multiple of its size raises a data abort with the cause "alignment fault" (QEMU 11.1.2 reports `ESR_EL1` = `0x9600_0021` for a load and `0x9600_0061` for a store). Compiled Rust code never does this on `aarch64-unknown-none` (the target sets `strict-align`), but hand-written assembly can, for example through a stack pointer that is not a multiple of 8. |
| Stack pointer alignment | `SCTLR_EL1` is `0x00c5_0838` at reset. Its bit 3 (`SA`) tells the CPU to raise an "SP alignment fault" (exception class `0x26`) whenever a stack pointer that is not a multiple of 16 is the base address of a load or store. Real CPUs make this check. QEMU 11.1.2 does not: a stack pointer that is off by 8 goes unnoticed here. Keep the stack pointer a multiple of 16 at all times anyway. |

Device tree: for a bare-metal ELF image QEMU can place a description of the
machine (a "device tree blob", 1 MiB) at the start of RAM, `0x4000_0000`. It
does so only if your image starts at least 1 MiB above that address. This
capstone does not use the device tree; all addresses you need are in section
2.3. Linking the kernel at `0x4010_0000` or higher keeps the option open.

### 2.3 Memory map

Only the regions that matter here. All devices are memory-mapped.

| Address | Size | Device | Used in |
| --- | --- | --- | --- |
| `0x0800_0000` | 64 KiB reserved, 4 KiB used | GICv2 distributor | M4 |
| `0x0801_0000` | 64 KiB reserved, 8 KiB used | GICv2 CPU interface | M4 |
| `0x0900_0000` | 4 KiB | PL011 UART (the serial port); its reference clock `UARTCLK` is 24 MHz | M1 |
| `0x0901_0000` | 4 KiB | PL031 real-time clock | not required |
| `0x0903_0000` | 4 KiB | PL061 GPIO | not required |
| `0x4000_0000` | 128 MiB | RAM (ends at `0x4800_0000`) | M1 |

Interrupt numbers as the GIC reports them (INTID):

| INTID | Source | Used in |
| --- | --- | --- |
| 27 | EL1 virtual timer (private peripheral interrupt 11; private peripheral interrupts start at 16) | M4 |
| 30 | EL1 physical timer (private peripheral interrupt 14) | not required |
| 33 | PL011 UART (shared peripheral interrupt 1; shared interrupts start at 32) | not required |

Sources, all from QEMU: the tables `base_memmap` (addresses) and `a15irqmap`
(shared interrupt numbers) in
[`hw/arm/virt.c`](https://gitlab.com/qemu-project/qemu/-/blob/master/hw/arm/virt.c),
the timer interrupt numbers in
[`include/hw/arm/bsa.h`](https://gitlab.com/qemu-project/qemu/-/blob/master/include/hw/arm/bsa.h),
and the manual page
[`virt` generic virtual platform](https://www.qemu.org/docs/master/system/arm/virt.html).
The addresses and the sizes in use were also read from a running QEMU 11.1.2
with the monitor command `info mtree`.

### 2.4 Register facts

These are the facts you need. The linked documents are the authority; read the
relevant pages before you write a driver.

**PL011 UART** ([PrimeCell UART (PL011) Technical Reference Manual, Arm DDI 0183](https://developer.arm.com/documentation/ddi0183/latest/), chapter 3). All registers are 32 bits wide.

| Offset | Name | Facts |
| --- | --- | --- |
| `0x00` | `UARTDR` | Write: send one byte. Read: receive one byte. |
| `0x18` | `UARTFR` | Read only. Bit 3 `BUSY`: a byte is being sent. Bit 5 `TXFF`: the transmit FIFO is full. |
| `0x24` | `UARTIBRD` | Integer part of the baud rate divisor (16 bits). |
| `0x28` | `UARTFBRD` | Fractional part of the divisor (6 bits, units of 1/64). |
| `0x2c` | `UARTLCR_H` | Bit 4 `FEN`: enable FIFOs. Bits 6:5 `WLEN`: `0b11` means 8 data bits. A write to this register also latches `UARTIBRD` and `UARTFBRD`, so write it after them. |
| `0x30` | `UARTCR` | Bit 0 `UARTEN`: enable. Bit 8 `TXE`: transmit enable. Bit 9 `RXE`: receive enable. Disable the UART before you change the other registers. |
| `0x38` | `UARTIMSC` | Interrupt mask. `0` masks all UART interrupts. |
| `0x44` | `UARTICR` | Write only. Writing `0x7ff` clears all pending UART interrupts. |

The divisor is `UARTCLK / (16 × baud)`. Section 5.5 defines how it is split
into the two registers.

**GICv2** ([Arm Generic Interrupt Controller Architecture Specification, version 2, Arm IHI 0048](https://developer.arm.com/documentation/ihi0048/latest/), chapter 4). All registers are 32 bits wide.

| Block | Offset | Name | Facts |
| --- | --- | --- | --- |
| Distributor | `0x000` | `GICD_CTLR` | Bit 0: forward interrupts to the CPU interface. |
| Distributor | `0x100 + 4n` | `GICD_ISENABLERn` | One bit per interrupt. Writing 1 enables the interrupt; writing 0 does nothing. |
| Distributor | `0x180 + 4n` | `GICD_ICENABLERn` | Writing 1 disables the interrupt. |
| Distributor | `0x400 + 4n` | `GICD_IPRIORITYRn` | One byte per interrupt. A lower value is more urgent. |
| Distributor | `0x800 + 4n` | `GICD_ITARGETSRn` | One byte per interrupt: a bit mask of CPUs. Read-only for INTIDs 0 to 31. |
| CPU interface | `0x00` | `GICC_CTLR` | Bit 0: signal interrupts to this CPU. |
| CPU interface | `0x04` | `GICC_PMR` | Priority mask. Only interrupts with a priority value lower than this one are signalled. `0xff` lets every priority below `0xff` through. |
| CPU interface | `0x0c` | `GICC_IAR` | Read: acknowledge. Bits 9:0 give the INTID of the most urgent pending interrupt and mark it active. 1023 means that nothing is pending. |
| CPU interface | `0x10` | `GICC_EOIR` | Write the value read from `GICC_IAR` to end the interrupt. Until then, no interrupt of the same or lower urgency is signalled. |

**Generic timer** ([Arm Architecture Reference Manual for A-profile, Arm DDI 0487](https://developer.arm.com/documentation/ddi0487/latest/), chapter "The Generic Timer in AArch64 state"). These are system registers, read with `mrs` and written with `msr`.

| Register | Facts |
| --- | --- |
| `CNTFRQ_EL0` | Counter frequency in Hz. Read it; do not hard-code it. (QEMU 11.1.2 reports 62 500 000 for `cortex-a72` and 1 000 000 000 for `-cpu max`.) |
| `CNTVCT_EL0` | The virtual counter: a 64-bit value that only grows. |
| `CNTV_CVAL_EL0` | Compare value. The timer condition is true while `CNTVCT_EL0 >= CNTV_CVAL_EL0`. |
| `CNTV_TVAL_EL0` | A 32-bit signed view: writing `t` sets the compare value to "now + `t`". |
| `CNTV_CTL_EL0` | Bit 0 `ENABLE`. Bit 1 `IMASK`: 1 hides the interrupt. Bit 2 `ISTATUS` (read only): the condition is true. The interrupt line (INTID 27) is high while `ENABLE` = 1, `IMASK` = 0 and the condition is true. It is a level: it stays high until you move the compare value into the future or disable the timer. |

**Exceptions** (same manual, chapter "The AArch64 System Level Programmers' Model", and the shorter guide [Learn the architecture: AArch64 Exception Model](https://developer.arm.com/documentation/102412/latest/)).

| Item | Facts |
| --- | --- |
| `VBAR_EL1` | Address of the vector table. The address must be a multiple of 2048. |
| Vector table | 16 entries of 128 bytes (32 instructions) each. Entry offset = group + kind. Groups: `0x000` taken from EL1 while using `SP_EL0`; `0x200` taken from EL1 while using `SP_EL1`; `0x400` taken from a lower level in AArch64 state; `0x600` taken from a lower level in AArch32 state. Kinds: `0x000` synchronous, `0x080` IRQ, `0x100` FIQ, `0x180` SError. |
| On an exception to EL1 the CPU | writes the return address to `ELR_EL1` and the processor state to `SPSR_EL1`, sets all four `DAIF` mask bits, selects `SP_EL1` and jumps to the entry. It saves no general register. |
| `eret` | Restores the program counter from `ELR_EL1` and the processor state from `SPSR_EL1`. |
| `ESR_EL1` | Cause of a synchronous exception. Bits 31:26 `EC`: the exception class. `0x15` is an `svc` instruction in AArch64 state; then bits 15:0 hold the immediate of that instruction. Other classes: section 5.5. |
| `svc #imm` | For this exception `ELR_EL1` holds the address of the instruction *after* the `svc`. |
| `FAR_EL1` | The faulting address of an abort. |
| `DAIF` | Bit 7 `I` masks IRQs. `msr daifset, #2` sets it, `msr daifclr, #2` clears it. |
| `CurrentEL` | Bits 3:2 hold the exception level. |
| `CPACR_EL1` | Bits 21:20 `FPEN`: `0b11` means that floating-point and SIMD registers are not trapped. |

**Calling convention** ([Procedure Call Standard for the Arm 64-bit Architecture (AAPCS64)](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst)): a function must preserve x19 to x28, x29 (the frame pointer), the stack pointer, and the low 64 bits of v8 to v15 (named d8 to d15). All other general and SIMD registers, and the condition flags, may be changed by any call. x30 (the link register) holds the return address when a function is entered. A function may overwrite it, so a caller does not rely on it after the call; but a context switch must save and restore it, because it says where the task continues. The stack pointer is a multiple of 16 at every call.

**Power off** ([Arm Power State Coordination Interface, Arm DEN 0022](https://developer.arm.com/documentation/den0022/latest/), function `SYSTEM_OFF`): put `0x8400_0008` in x0 and execute `hvc #0`. On this machine QEMU itself answers the call and exits with status 0. The call does not return. It may change x0 to x3.

## 3. Rules for the kernel

### 3.1 Build

1. `kernel/` must build with the pinned stable toolchain (`rust-toolchain.toml`)
   using `cargo build --target aarch64-unknown-none`, with and without
   `--release`, started inside `kernel/`. The tests use exactly these
   commands. How the linker script reaches the linker is your choice.
2. The package and its binary must keep the name `kernel`.
3. No nightly features, no `-Z` flags and no crates other than `kcore`, `core`
   and `alloc`.
4. The package must keep the Cargo feature `selftest-panic` (section 4.3).

### 3.2 Output

5. All output goes to the PL011 UART at `0x0900_0000`.
6. Output is a sequence of lines of printable ASCII. A line ends with `\n`;
   a `\r` directly before the `\n` is allowed and ignored.
7. A line must never be mixed with another line, even when an interrupt
   arrives or the timer switches tasks while the line is being printed.

### 3.3 Unsafe code

8. Every `unsafe` block and every `unsafe impl` has a `// SAFETY:` comment
   that says why the operation is sound at that place. Every `unsafe fn` has
   a `# Safety` section that states what the caller must guarantee.
9. Device registers are accessed only with volatile reads and writes through
   raw pointers. No Rust reference (`&` or `&mut`) to device memory is created.
10. `static mut` is not used for state that both a task and an interrupt
    handler touch. Such state sits behind a type that makes the access rule
    explicit (for example, a lock that masks interrupts).
11. `kcore` passes its tests under Miri (section 6).

### 3.4 Structure

12. Code outside the drivers reaches a device only through a trait (M2).
13. Addresses and interrupt numbers of section 2.3 appear in one place.
14. The kernel uses the `kcore` types for its heap, run queue and IPC
    endpoints. It does not carry a second copy of that logic.

### 3.5 Ending a run

15. The kernel ends every run by itself: it prints `[m1] power off` as its
    last line, waits until the UART has sent it, and powers the machine off
    with PSCI. QEMU must exit with status 0.
16. A complete run must end within the time limit of the tests (10 seconds by
    default; the reference needs under half a second). The first byte of
    output must appear within a third of the limit. A run may print at most
    256 KiB.

## 4. Milestones and the transcript

### 4.1 The transcript

A kernel that has finished all six milestones prints exactly these lines, in
this order, and nothing else. The same text is in
[`tests/transcript.txt`](tests/transcript.txt), which is what the tests read.

```text
[m1] boot: EL1
[m1] bss: zeroed
[m2] console: pl011
[m2] timer: arm generic timer
[m2] fmt: 42 0x2a kernel
[m2] delay: 10 ms
[m3] bump: ok
[m3] box: 42
[m3] vec: 1000 items, sum 499500
[m3] reuse: ok
[m4] vectors: installed
[m4] svc: 0x42
[m4] tick 1
[m4] tick 2
[m4] tick 3
[m5] coop: A0
[m5] coop: B0
[m5] coop: C0
[m5] coop: A1
[m5] coop: B1
[m5] coop: C1
[m5] coop: joined
[m5] preempt: A>B B>C C>A A>B B>C C>A
[m5] preempt: joined
[m6] math: 2 + 3 = 5
[m6] math: 6 * 7 = 42
[m6] client: bye
[m6] ipc: 3 console messages
[m1] power off
```

Each line starts with the tag of the milestone that introduces it. A line is a
claim about your kernel. Print it only at the moment the rules below allow,
and only from real state: the number in `[m3] vec` comes from a real `Vec`, the
trace in `[m5] preempt` from switches that really happened.

### 4.2 Working in stages

The test for milestone N compares only the lines tagged `[mN]` with the lines
of the transcript that carry the same tag. So a kernel that has finished M1 to
M3 prints the `[m1]`, `[m2]` and `[m3]` lines, the tests `m1_boot`, `m2_drivers`
and `m3_heap` pass, and the later ones fail. Lines without a tag (your own
debug output) are ignored by the milestone tests. The test `full_transcript`
accepts no extra line.

The last line `[m1] power off` belongs to M1, so M1 fails whenever the kernel
does not end the run properly.

#### M1: boot

Goal: get from the first instruction to Rust code, print, and end the run.

Requirements:

- A linker script places the image in RAM and defines the entry address.
- A short assembly entry routine (`global_asm!`) sets the stack pointer,
  enables floating-point access (section 2.2), clears `.bss` and calls Rust.
- A PL011 driver initialises the UART for 115200 baud, 8 data bits, no
  parity, 1 stop bit, with FIFOs on, and can send bytes. It describes the
  registers as a typed block (a `#[repr(C)]` struct, or one typed accessor
  per register) and waits for room in the FIFO before each byte.
- A panic handler (section 4.3).

Lines:

| Line | Print it when |
| --- | --- |
| `[m1] boot: EL1` | First output of the kernel. The digit is the value read from `CurrentEL` at run time. |
| `[m1] bss: zeroed` | The first thing your Rust entry function does is to check that every byte of `.bss` is zero. Print the line only if it is. |
| `[m1] power off` | Section 3.5. |

#### M2: drivers behind traits

Goal: a driver layer in the style of firmware. The kernel proper talks to a
console and a timer through traits; which device implements them is decided
in one place.

Requirements:

- A console trait, implemented by the PL011 driver, and a timer trait,
  implemented by a driver for the generic timer (no interrupts yet: read the
  counter and its frequency). Each driver reports a name.
- `print!` and `println!` macros with the usual `format_args!` syntax that
  write through the console trait.

Lines:

| Line | Print it when |
| --- | --- |
| `[m2] console: pl011` | The text after the colon is the name that the console driver returns through the console trait. |
| `[m2] timer: arm generic timer` | The same for the timer driver. |
| `[m2] fmt: 42 0x2a kernel` | Printed by one `println!` whose arguments are the number 42 twice (formatted with `{}` and with `{:#x}`) and the string `"kernel"`. |
| `[m2] delay: 10 ms` | After a busy-wait that used the timer trait and during which the counter advanced by at least `frequency / 100`. |

#### M3: memory

Goal: a kernel heap, so that `alloc::boxed::Box`, `alloc::vec::Vec` and
`alloc::string::String` work.

Requirements:

- `kcore::Bump` and `kcore::FreeList` (section 5.2) pass the host tests.
- The kernel has a `#[global_allocator]` that wraps `kcore::FreeList`, manages
  at least 1 MiB of RAM taken from the linker script, and is safe to use from
  tasks and from interrupt handlers.

Lines:

| Line | Print it when |
| --- | --- |
| `[m3] bump: ok` | After running `kcore::Bump` inside the kernel on a region of 4096 bytes: three blocks were allocated (one of them with an alignment of 64) and are aligned and disjoint, a request for 8192 bytes returned null, and after freeing the three blocks `used()` is 0. |
| `[m3] box: 42` | Prints the content of a `Box<u64>` that holds 42. |
| `[m3] vec: 1000 items, sum 499500` | Prints `len()` and the sum of a `Vec<u64>` that was collected from `0..1000`. |
| `[m3] reuse: ok` | After this check: note the free bytes of the heap; allocate at least 64 separate boxes; drop them in an order that is neither first-to-last nor last-to-first; the free bytes of the heap are the same as before. |

#### M4: exceptions and interrupts

Goal: the kernel handles a synchronous exception and a timer interrupt and
returns from both.

Requirements:

- A vector table with all 16 entries. Entries that the kernel does not expect
  must end in the panic path with the cause in the message, not in silence.
- The entry code saves every register that the interrupted code could be
  using, including the floating-point and SIMD registers, and restores them
  before `eret`.
- The synchronous handler decodes `ESR_EL1` with `kcore::reg`. For an `svc`
  taken from EL1 it returns the 16-bit immediate in the caller's x0.
- A GICv2 driver and a periodic timer interrupt at 100 Hz (INTID 27), both
  behind traits. The handler counts ticks.

Lines:

| Line | Print it when |
| --- | --- |
| `[m4] vectors: installed` | After `VBAR_EL1` points at your table. |
| `[m4] svc: 0x42` | The kernel executed `svc #0x42`; the value is what the handler left in x0, formatted with `{:#x}`. |
| `[m4] tick 1`, `[m4] tick 2`, `[m4] tick 3` | Printed by the boot code, not by the interrupt handler, each time it sees that the tick counter has reached the next number. Between checks the boot code sleeps with `wfi`. After the third line the kernel stops the timer interrupt. |

#### M5: tasks

Goal: several flows of control inside the kernel, each with its own stack,
switched first by their own request and then by the timer.

Requirements:

- A context switch that saves and restores what the calling convention
  requires (section 2.4), including d8 to d15.
- Tasks are created with a stack from the kernel heap. The code that booted
  the kernel is task 0. The scheduling policy is `kcore::Scheduler`.
- `yield`, `exit`, and `join` (wait until a given task has exited, then free
  its stack).
- Pre-emption: the timer interrupt (100 Hz) switches to the next ready task.

Lines, cooperative part (the timer interrupt is off):

| Line | Print it when |
| --- | --- |
| `[m5] coop: A0` to `[m5] coop: C1` | The boot task creates three tasks named A, B and C, in this order, and then joins them. Each task runs two rounds; in round `r` it prints `[m5] coop: <name><r>` and then yields. After the second round it exits. |
| `[m5] coop: joined` | Printed by the boot task after it has joined all three tasks. |

Lines, pre-emptive part (the timer interrupt is on again and stays on):

| Line | Print it when |
| --- | --- |
| `[m5] preempt: A>B B>C C>A A>B B>C C>A` | The boot task creates three new tasks named A, B and C and joins them. These tasks never yield and never block: each one loops until the timer has taken the CPU away from *it* at least twice, then returns. The kernel records every switch that the timer causes as "from>to". After the joins, the boot task prints the first six recorded switches. The boot task must already be blocked when the first tick arrives; otherwise the first entry would name the boot task. |
| `[m5] preempt: joined` | Printed by the boot task after it has joined all three tasks. |

#### M6: message passing

Goal: the microkernel idea in small. Tasks do not call each other's functions;
they exchange messages through the kernel.

Requirements:

- Kernel operations `send` (send a request and wait for the reply), `receive`
  (wait for a request, learn who sent it) and `reply`, built on
  `kcore::Endpoint` and on blocking and waking tasks.
- A message is a `kcore::Message`: it is copied from sender to receiver. No
  pointer or reference crosses from one task to another.
- Pre-emption stays on during this milestone.
- A *console server* task. It is the only code that prints the three lines
  below that come from requests. Request "write": the message carries a
  length and up to 24 bytes of text; the server checks that the length is in
  range and the bytes are UTF-8, prints `[m6] ` followed by the text, counts
  the request and replies. Request "count": the server replies with the
  number of texts it has printed.
- A *math server* task with the requests "add" and "multiply" on two numbers.
- A *client* task that never touches the UART.

Lines:

| Line | Print it when |
| --- | --- |
| `[m6] math: 2 + 3 = 5` | The client asked the math server to add 2 and 3, built the text from the reply and sent it to the console server. |
| `[m6] math: 6 * 7 = 42` | The same with "multiply", 6 and 7. |
| `[m6] client: bye` | The client's last request to the console server. Then the client exits. |
| `[m6] ipc: 3 console messages` | The boot task joined the client, sent "count" to the console server, and prints the number from the reply itself. |

The two servers never exit. They are blocked in `receive` when the machine is
powered off.

### 4.3 The panic path

A panic must leave a trace and must end the run. The panic handler:

1. masks interrupts;
2. prints one line that starts with `[panic] ` and contains the panic message
   (a file and line number in the same line are welcome);
3. waits until the UART has sent the line and powers the machine off.

It must work when the panic happens inside a critical section, and it must
not print forever if the handler itself panics.

The Cargo feature `selftest-panic` exists to test this. With the feature
enabled, the kernel calls `panic!("selftest")` directly after it has printed
`[m1] boot: EL1`. The complete output of such a build is two lines:

```text
[m1] boot: EL1
[panic] <anything that contains "selftest">
```

and QEMU exits with status 0.

## 5. The `kcore` facade

The host tests use exactly the public items of this section, with the
signatures in `kcore/src/lib.rs`. Everything else in the crate is yours: add
fields, private functions and modules as you like, and re-export the names
from the crate root (and from `kcore::reg`).

Rules for the whole crate: `#![no_std]`; no `alloc`; no dependencies; every
constructor named `new` and every function in `reg` is a `const fn`; no
function may loop forever or panic for the inputs described here. Each of the
five types also implements `Default` with the same result as `new()`.

### 5.1 `RingBuffer<T, const N: usize>`

A first-in, first-out queue that stores at most `N` values inside itself.
`N` may be 0.

| Item | Contract |
| --- | --- |
| `const fn new() -> Self` | An empty queue. |
| `fn capacity(&self) -> usize` | `N`. |
| `fn len(&self) -> usize` | The number of values in the queue. |
| `fn is_empty(&self) -> bool` | `len() == 0`. |
| `fn is_full(&self) -> bool` | `len() == N`. |
| `fn push(&mut self, value: T) -> Result<(), T>` | Adds `value` at the back. If the queue is full, changes nothing and returns `Err(value)`. |
| `fn pop(&mut self) -> Option<T>` | Removes and returns the value that was pushed first, or `None`. |
| dropping the queue | Drops every value that is still in it, exactly once. |

### 5.2 `Bump` and `FreeList`

Both manage one region of memory that the caller hands over with `init`.
`alloc` and `dealloc` have the meaning of the same methods of
[`core::alloc::GlobalAlloc`](https://doc.rust-lang.org/core/alloc/trait.GlobalAlloc.html),
except that they take `&mut self`. Layouts with size 0 are never passed.

Common contract:

| Item | Contract |
| --- | --- |
| `const fn new() -> Self` | An allocator without memory. `alloc` returns null. |
| `unsafe fn init(&mut self, start: *mut u8, len: usize)` | Takes over the region `start .. start + len`. The caller guarantees that the region is valid for reads and writes, is used by nothing else while the allocator or one of its blocks is in use, and that `start + len` does not overflow. Any `start` and `len` must work, including values that are not aligned. |
| `fn alloc(&mut self, layout: Layout) -> *mut u8` | Returns a block of at least `layout.size()` bytes, aligned to `layout.align()`, that lies inside the region and overlaps no other live block; or null if no such block is available. A request that fails changes nothing. A size that cannot fit (up to `isize::MAX`) gives null, not an arithmetic overflow. |
| `unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout)` | The caller guarantees that `ptr` came from `alloc` on this allocator with the same `layout` and has not been freed. |

The allocator must never write into a live block (its bookkeeping lives
elsewhere or in free memory). All pointers it returns, and all pointers it
keeps, must be derived from the pointer given to `init`. This also holds in
`dealloc`: use only the *address* of `ptr`. A `Box<u8>` gives its block back
through a pointer that is valid for one byte, while the block that the
allocator takes back may be larger (a rounded size) and may later be merged
with its neighbours. The host tests check both rules under Miri.

`Bump` in addition:

| Item | Contract |
| --- | --- |
| `fn used(&self) -> usize` | The distance from the start of the region to the first byte that was never handed out since the last reset. 0 after `init`. At least the sum of the sizes of the blocks allocated since the last reset. |
| `dealloc` | Freeing a block while other blocks are live changes nothing visible (`used()` stays). Freeing the last live block resets the allocator: `used()` is 0 and the whole region can be allocated again. |

`FreeList` in addition:

| Item | Contract |
| --- | --- |
| `fn free_bytes(&self) -> usize` | The number of bytes available for allocation. 0 before `init`. After `init` at most `len`, and at least `len - 64`. |
| `alloc` | Supports every alignment that is a power of two up to 4096. A successful call lowers `free_bytes()` by at least `layout.size()`. |
| `dealloc` | The block becomes available again and is merged with free neighbours. After *every* block has been freed, in any order: `free_bytes()` equals its value directly after `init`, and one block of three quarters of the region can be allocated. |

### 5.3 `Scheduler<const N: usize>`

The round-robin policy for at most `N` tasks, counted over the whole life of
the scheduler (`N >= 1`). It decides; it does not switch stacks.

`TaskId(pub usize)` names a task. `TaskState` is `Ready`, `Running`,
`Blocked` or `Exited`. At every moment exactly one task is `Running`. The
*ready queue* holds the `Ready` tasks in the order in which they became ready.

| Item | Contract |
| --- | --- |
| `const fn new() -> Self` | Task 0 exists and is `Running`: it stands for the code that created the scheduler. |
| `fn spawn(&mut self) -> Option<TaskId>` | Creates a task with the next unused id (1, 2, ...; ids are never reused), in state `Ready`, at the back of the ready queue. `None` once `N` ids are used. Does not change the running task. |
| `fn current(&self) -> TaskId` | The running task. |
| `fn state(&self, id: TaskId) -> Option<TaskState>` | The state of the task, or `None` if no task with this id was ever created (for any `id`, also beyond `N`). |
| `fn yield_current(&mut self) -> Option<TaskId>` | If the ready queue is empty: changes nothing, returns `None`. Otherwise the running task becomes `Ready` at the back of the queue, the task at the front becomes `Running`, and its id is returned. |
| `fn block_current(&mut self) -> Option<TaskId>` | If the ready queue is empty: changes nothing, returns `None` (the caller has found a deadlock). Otherwise the running task becomes `Blocked`, the task at the front becomes `Running`, and its id is returned. |
| `fn exit_current(&mut self) -> Option<TaskId>` | Like `block_current`, but the running task becomes `Exited`. |
| `fn wake(&mut self, id: TaskId) -> bool` | If the task is `Blocked`: it becomes `Ready` at the back of the queue; returns `true`. In every other case (also for unknown ids): changes nothing, returns `false`. Never changes the running task. |

### 5.4 `Endpoint<const N: usize>`, `Message`, `IpcError`

`Message` has the public fields `label: u64` and `words: [u64; 4]`, is `Copy`
and its `Default` is all zeros.

An `Endpoint` is the state machine of one meeting point between clients and a
server. It never blocks or wakes a task itself. Each method tells the caller
(your kernel) what happened, and the kernel then blocks or wakes tasks. A
*call* is in progress from a successful `send` until `take_reply` has returned
its reply. A call is in one of three stages: *queued* (sent, not received),
*awaiting reply* (received, not replied) and *replied* (reply stored, not
collected).

| Item | Contract |
| --- | --- |
| `const fn new() -> Self` | No calls, nobody waiting. |
| `fn send(&mut self, from: TaskId, msg: Message) -> Result<Option<TaskId>, IpcError>` | `Err(CallInProgress)` if `from` already has a call in progress on this endpoint; else `Err(Full)` if `N` calls are in progress. Otherwise the call is queued behind all queued calls. If a task is recorded as waiting to receive, the record is cleared and `Ok(Some(that task))` is returned: the kernel must wake it. Else `Ok(None)`. After `Ok`, the kernel blocks `from` until `take_reply` gives it a reply. |
| `fn receive(&mut self, me: TaskId) -> Result<Option<(TaskId, Message)>, IpcError>` | `Err(ReceiverBusy)` if a *different* task is recorded as waiting to receive. Otherwise, if a call is queued: the oldest queued call moves to *awaiting reply*, any waiting record is cleared, and `Ok(Some((sender, message)))` is returned. If none is queued: `me` is recorded as waiting and `Ok(None)` is returned; the kernel blocks `me`, and `me` calls `receive` again after it was woken. |
| `fn reply(&mut self, to: TaskId, msg: Message) -> Result<(), IpcError>` | If `to` has a call in the stage *awaiting reply*: the reply is stored and the call moves to *replied*; the kernel must wake `to`. Otherwise `Err(NotAwaitingReply)` and nothing changes. |
| `fn take_reply(&mut self, me: TaskId) -> Option<Message>` | If `me` has a call in the stage *replied*: returns the reply and ends the call. Otherwise `None`. |

Errors never change the state.

### 5.5 `kcore::reg`

Pure functions on numbers. All are `const fn`.

| Item | Contract |
| --- | --- |
| `get_field(value: u64, hi: u32, lo: u32) -> u64` | Bits `lo ..= hi` of `value`, moved down so that bit `lo` becomes bit 0. Requires `lo <= hi <= 63`; other inputs are not tested. |
| `set_field(value: u64, hi: u32, lo: u32, field: u64) -> u64` | `value` with bits `lo ..= hi` replaced by the low `hi - lo + 1` bits of `field`. All other bits of `value` are kept; higher bits of `field` are ignored. Same requirement on `hi` and `lo`. |
| `pl011_baud_divisors(uartclk_hz: u32, baud: u32) -> (u32, u32)` | Let `d = uartclk_hz / (16 × baud)` as an exact fraction, and `q = d × 64` rounded to the nearest integer (halves round up). Returns `(q / 64, q % 64)` using integer division: the values for `UARTIBRD` and `UARTFBRD`. Must be correct for every `uartclk_hz` and every `baud >= 1`. Example: 24 000 000 and 115 200 give `(13, 1)`. |
| `gic_bit_slot(intid: u32) -> (usize, u32)` | For the GIC registers with one bit per interrupt: `(intid / 32, 1 << (intid % 32))`, the register index `n` and the bit mask. |
| `gic_byte_slot(intid: u32) -> (usize, u32)` | For the GIC registers with one byte per interrupt: `(intid / 4, 8 × (intid % 4))`, the register index `n` and the shift of the byte. |
| `timer_interval(counter_hz: u64, tick_hz: u64) -> u64` | `counter_hz / tick_hz` (integer division): the counter ticks between two interrupts. Requires `tick_hz >= 1`. |
| `exception_class(esr: u64) -> ExceptionClass` | Decodes bits 31:26 of `esr` and ignores all other bits: `0x00` → `Unknown`, `0x15` → `Svc`, `0x20` → `InstructionAbortLower`, `0x21` → `InstructionAbortSame`, `0x24` → `DataAbortLower`, `0x25` → `DataAbortSame`, `0x3c` → `Brk`, any other value `v` → `Other(v)`. |
| `svc_number(esr: u64) -> Option<u16>` | `Some(bits 15:0 of esr)` if the class is `Svc`, else `None`. |

## 6. The tests

Run everything from this directory.

| Command | What it checks |
| --- | --- |
| `cargo test --test host_ring` (also `host_heap`, `host_sched`, `host_ipc`, `host_reg`) | Section 5, on your own computer, in milliseconds. |
| `cargo +nightly-2026-11-20 miri test` | The same host tests under Miri, which detects undefined behaviour in the unsafe code of `kcore`. The QEMU tests are skipped automatically. |
| `cargo test --test qemu` | Builds `kernel/`, boots it in QEMU and checks section 4. `cargo test --release --test qemu` does the same with an optimised kernel. Both must pass. |
| `cargo test` | All of the above except Miri. |

The QEMU tests:

| Test | Passes when |
| --- | --- |
| `m1_boot` to `m6_ipc` | The lines with that milestone's tag equal the transcript lines with the same tag (section 4.2). |
| `powers_off_cleanly` | QEMU exited by itself with status 0, the last line is `[m1] power off`, and no line starts with `[panic] `. |
| `full_transcript` | The whole output equals section 4.1. |
| `panic_selftest` | A second kernel, built with `--features selftest-panic`, behaves as in section 4.3. |

If `qemu-system-aarch64` or the Rust target `aarch64-unknown-none` is not
installed, the QEMU tests print a line that starts with `SKIPPED` and pass.
A skipped test proves nothing. Set `KERNEL_REQUIRE_QEMU=1` to turn a skip into
a failure, for example in continuous integration. The other environment
variables are listed at the top of [`harness/lib.rs`](harness/lib.rs).

The harness always ends QEMU: when the kernel powers off, at the time limit,
or when a test fails.

## 7. Stretch goals

Not tested. Each one is a serious piece of work; pick what interests you. If
you add output, keep the transcript of section 4.1 intact (print extra lines
only when a Cargo feature is on).

1. **User tasks at EL0.** Run a task at exception level 0 with its own stack
   (`SP_EL0`). It enters the kernel with `svc`; the "lower EL, AArch64"
   vectors handle it. Give it system calls for `send`, `receive`, `reply`,
   `yield` and `exit`, and move the client of M6 to EL0.
2. **An address space.** Turn the MMU on: build translation tables, map the
   kernel and the devices, give each user task its own table, and add an
   unmapped guard page below every stack so that a stack overflow becomes a
   clean data abort.
3. **Interrupt-driven console input.** Enable the UART receive interrupt
   (INTID 33), put received bytes into a `kcore::RingBuffer`, and add a
   "read line" request to the console server.
4. **Sleeping.** A `sleep(ticks)` operation, a queue of sleeping tasks ordered
   by wake-up time, and an idle task that executes `wfi`.
5. **Read the device tree.** Find the UART and the size of RAM in the blob at
   `0x4000_0000` instead of using constants.
6. **A RISC-V port.** Add a second board (`qemu-system-riscv64 -machine virt`,
   target `riscv64gc-unknown-none-elf`): a 16550 UART, the CLINT timer and
   the PLIC interrupt controller behind the same traits. If only the drivers,
   the entry code, the trap code and the context switch change, your
   hardware boundary was drawn well.
7. **More cores.** Start the other CPUs with PSCI `CPU_ON` and replace the
   interrupt-masking lock by a spin lock that also masks interrupts.

## 8. Non-goals

The acceptance tests do not ask for these, and the reference does not have
them: virtual memory, user mode, more than one CPU core, file systems, block
or network devices, dynamic loading of programs, sleeping or timeouts,
priorities, task ids that are reused, freeing the stacks of tasks that are
never joined, protection against stack overflow, UART input, and running on
real hardware.

## 9. Evidence to keep

The rubric asks for evidence, not for claims. Keep these in a file
`EVIDENCE.md` next to your code:

1. The output of `cargo test` and of `cargo test --release` (the summary
   lines are enough), with the QEMU version printed by
   `qemu-system-aarch64 --version`.
2. The output of `cargo +nightly-2026-11-20 miri test`.
3. The output of `cargo clippy --all-targets` here and of
   `cargo clippy --target aarch64-unknown-none` in `kernel/`.
4. An inventory of your unsafe code: for each file, the number of `unsafe`
   blocks and one sentence on why that file needs them.
5. Half a page of design notes: where you drew the line between `kcore` and
   the binary, how interrupts and tasks share state, and one decision you
   would change next time.
