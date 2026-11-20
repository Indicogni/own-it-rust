# Capstone "kernel": concepts

Read this before and during the capstone. The earlier parts of the course ran
every program on top of an operating system. Here there is none: your program
*is* the lowest layer of software. This file teaches what that changes.

**After the capstone you will be able to**

- build a Rust program that starts with no operating system and no standard
  library, and explain every step from the first instruction to `main`;
- read a device data sheet and write a driver with typed, volatile register
  access behind a trait;
- write an allocator, a scheduler and a message-passing layer as ordinary
  library code that is tested on your own computer and checked by Miri;
- handle exceptions and interrupts, and switch between tasks;
- say for every `unsafe` block in a kernel why it is there and why it is sound.

**What is assumed.** Chapters [23](../../exercises/chapter23/README.md)
(unsafe code) and [24](../../exercises/chapter24/README.md) (foreign
functions), and the mastery modules
[`m2_unsafe`](../../mastery/m2_unsafe/README.md) (raw pointers,
`MaybeUninit`, building `Box` and `Vec`, Miri) and
[`m3_concurrency`](../../mastery/m3_concurrency/README.md) (atomics, locks). You do not need
to know Arm assembly; the dozen instructions you need are explained here.

**Time.** These are estimates for a first attempt, including reading:

| Topic | Read | Milestone | Build |
| --- | --- | --- | --- |
| 1 Life without `std` | 30 min | M1 | 6 to 10 hours |
| 2 From power-on to Rust | 45 min | M1 | |
| 3 Device registers | 45 min | M1 | |
| 4 Drivers behind traits | 30 min | M2 | 3 to 5 hours |
| 5 A heap from nothing | 45 min | M3 | 6 to 10 hours |
| 6 Exceptions | 60 min | M4 | 8 to 14 hours |
| 7 Interrupts | 45 min | M4 | |
| 8 Context switching | 60 min | M5 | 8 to 14 hours |
| 9 Message passing | 30 min | M6 | 5 to 8 hours |
| 10 Where the unsafe code lives | 30 min | all | |

**About the code blocks.** Blocks marked `rust` are complete programs that run
on your computer; `rustdoc --edition 2024 --test CONCEPTS.md` checks them.
Blocks marked `rust,ignore` only make sense on the bare-metal target. Each of
them is taken from a small program that was built for `aarch64-unknown-none`
with the pinned toolchain and booted in QEMU 11.1.2; the output shown is what
it printed. None of the examples is a part of the capstone's solution: they
use other devices and leave out what the specification asks you to add.

---

## Topic 1 · Life without `std`

### The model

The standard library is three layers. `core` needs nothing from the outside:
integers, slices, `Option`, iterators, formatting, atomics, raw pointers.
`alloc` adds the types that need a heap (`Box`, `Vec`, `String`, `Rc`) and
asks for one thing: a *global allocator*. `std` adds everything that needs an
operating system: files, threads, sockets, time, `println!`.

`#![no_std]` at the top of a crate removes the automatic `use std`. `core` is
always there. `alloc` is there if you write `extern crate alloc;` and provide
an allocator (topic 5).

Three more things that `std` did silently are now yours:

1. **Starting.** A normal Rust program does not begin at `fn main`. The
   operating system starts a runtime that sets up the stack and arguments and
   then calls `main`. With `#![no_main]` you say that there is no such
   runtime; topic 2 shows what replaces it.
2. **Panicking.** `core` can detect a panic (an index out of range, an
   `unwrap` on `None`) but does not know what to do about it. You must supply
   one function marked `#[panic_handler]`. It receives the message and the
   location, and it must never return.
3. **Stopping.** There is no process to exit from. A kernel that is done has
   to ask the machine to power off, or sleep forever.

The compiler builds for a *target*. `aarch64-unknown-none` means: 64-bit Arm,
no vendor, no operating system. For this target Rust ships `core` and `alloc`
precompiled, so a stable toolchain is enough. Panics abort on this target;
there is no unwinding.

### Worked example: formatting without `std`

`println!` belongs to `std`, but the machinery behind it, `core::fmt`, does
not. Anything that implements `core::fmt::Write` can receive formatted text.
Here it is a fixed array; in your kernel it will be a serial port. This
example uses only `core`, so the same code compiles for bare metal.

```rust
use core::fmt::{self, Write};

/// Collects formatted text in a fixed array.
struct LineBuffer {
    bytes: [u8; 32],
    len: usize,
}

impl Write for LineBuffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        // `get_mut` returns `None` when the text does not fit.
        let free = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        free.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

let mut line = LineBuffer { bytes: [0; 32], len: 0 };
write!(line, "tick {} of {:#x}", 3, 255).unwrap();
assert_eq!(&line.bytes[..line.len], b"tick 3 of 0xff");

// The buffer has 18 bytes left; this text is longer.
assert!(write!(line, "{}", "and a text that is too long").is_err());
```

`write!` calls `write_str` once per piece (`"tick "`, `"3"`, `" of "`,
`"0xff"`), not once per line. Remember that when you make sure that a line of
output cannot be interrupted (topic 7).

### Mistakes people really make

**No panic handler.** A crate that contains only `#![no_std]` and `#![no_main]`
does not build:

```text
error: `#[panic_handler]` function required, but not found
```

Fix: write one. Its signature is `fn(&core::panic::PanicInfo) -> !`.

**Reaching for `std` out of habit.**

```text
error[E0433]: cannot find module or crate `std` in this scope
 --> src/main.rs:3:5
  |
3 | use std::collections::HashMap;
  |     ^^^ use of unresolved module or unlinked crate `std`
```

Fix: look for the item in `core` or `alloc` (`core::fmt`, `core::mem`,
`alloc::vec::Vec`, `alloc::collections::BTreeMap`). `HashMap` is not in
`alloc`, because its default hasher needs random numbers from the operating
system.

**Using `alloc` types before there is an allocator.** The program compiles
and then fails at link time:

```text
error: no global memory allocator found but one is required; link to std or add
       `#[global_allocator]` to a static item that implements the GlobalAlloc trait
```

Fix: topic 5.

### When to use which

Put code in a `no_std` *library* whenever it does not touch hardware. Such a
library builds for the bare-metal target and for your own computer, and on
your computer it has the normal test harness, a debugger and Miri. Keep the
`no_main` *binary* for the code that cannot exist anywhere else.

### Read more

- [`core`](https://doc.rust-lang.org/core/) and [`alloc`](https://doc.rust-lang.org/alloc/), the two libraries you have.
- [The Rust Reference: the `no_std` attribute](https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute) and [the `panic_handler` attribute](https://doc.rust-lang.org/reference/panic.html#the-panic_handler-attribute).
- [The rustc book: `aarch64-unknown-none`](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-none.html).
- [The Rustonomicon: Beneath `std`](https://doc.rust-lang.org/nomicon/beneath-std.html).

---

## Topic 2 · From power-on to Rust: boot flow and linker scripts

### The model

A compiled program is a set of *sections*:

| Section | Content | In the file? |
| --- | --- | --- |
| `.text` | machine code | yes |
| `.rodata` | constants, string literals | yes |
| `.data` | statics with a non-zero start value | yes |
| `.bss` | statics that start as zero | no: only its size is recorded |

Under an operating system the loader puts the sections somewhere, fills
`.bss` with zeros, creates a stack and jumps to the entry point. On bare
metal you decide the addresses, and your first instructions do the rest.

The **linker script** is the text file that tells the linker at which address
each section goes and which symbol is the entry point. It can also define
symbols of its own (an address with a name), which is how code learns where
`.bss` begins and ends or where the stack is.

What QEMU does with `-kernel file.elf` is small. It copies each loadable part
of the file to the address in its header, sets the program counter to the
entry address and lets the CPU run. The CPU is at exception level 1, the MMU
is off (addresses are physical), interrupts are masked, and the stack pointer
holds nothing useful. Rust code cannot run in that state: every function
needs a stack, and Rust assumes that zero-initialised statics are zero.

So the entry routine is assembly. It is short, and it is the only code in a
kernel that cannot be written in Rust.

```text
power on -> QEMU loads the ELF file -> _start (assembly)
                                         set the stack pointer
                                         (clear .bss, allow floating point)
                                         call the Rust entry function
```

### Worked example: the smallest program that boots

A linker script for the QEMU `virt` machine, whose RAM starts at `0x4000_0000`:

```text
ENTRY(_start)

SECTIONS
{
    . = 0x40100000;                 /* where the first byte of the image goes */

    .text : {
        KEEP(*(.text.boot))         /* the entry code first ... */
        *(.text .text.*)            /* ... then all other code */
    }
    .rodata : { *(.rodata .rodata.*) }
    .data   : { *(.data .data.*) }
    .bss    : { *(.bss .bss.*) }

    . = ALIGN(16);
    . += 0x4000;                    /* 16 KiB for the stack */
    __stack_top = .;                /* a symbol the entry code can use */
}
```

`.` is the *location counter*: the address at which the next thing is placed.
`*(.text .text.*)` means "these input sections from all object files". Rust
puts each function in its own section, such as `.text.main`, which is why the
pattern needs the `.*` part.

And the program:

```rust,ignore
#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

// The entry point. The linker script puts this section first and names
// `_start` as the address where execution begins.
global_asm!(
    r#"
    .section .text.boot, "ax"
    .global _start
_start:
    adrp    x0, __stack_top             // high part of the address ...
    add     x0, x0, :lo12:__stack_top   // ... plus the low 12 bits
    mov     sp, x0                      // now function calls work
    bl      main
1:  wfi                                 // main returned: sleep forever
    b       1b
"#
);

/// The transmit register of the UART on the QEMU `virt` machine.
const UART_DATA: *mut u8 = 0x0900_0000 as *mut u8;

#[unsafe(no_mangle)]
extern "C" fn main() {
    for &byte in b"hello from bare metal\n" {
        // SAFETY: on this machine a one-byte write to this address sends
        // the byte to the serial port and has no other effect.
        unsafe { UART_DATA.write_volatile(byte) };
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

Output in QEMU: `hello from bare metal`. Then QEMU keeps running, because
nothing powers the machine off.

Reading the assembly: `adrp` and `add` together load the address of a symbol
(one instruction cannot hold a 64-bit address). `mov sp, x0` sets the stack
pointer. `bl` is "branch with link", a function call. `wfi` is "wait for
interrupt", which puts the CPU to sleep. `1:` is a local label and `1b` means
"the label 1 before this line".

`#[unsafe(no_mangle)]` gives the function exactly the symbol name `main`, so
that the assembly can call it. `extern "C"` gives it the calling convention
that assembly code expects.

This example gets away with three shortcuts that the capstone does not allow:
it never clears `.bss` (it has no statics), it never enables floating-point
registers (it is too simple for the compiler to use them), and it writes to a
device through a bare address.

### Mistakes people really make

**The linker script is not used.** The build still succeeds, with a warning
that is easy to miss:

```text
warning: linker stderr: rust-lld: cannot find entry symbol _start; not setting start address
```

or with no warning at all, with the code linked at the linker's default
address where the machine has no RAM. QEMU then shows nothing. To check, run
`cargo build -v`: it prints the full compiler command, and your script must
appear in it as `-C link-arg=-T...`.

**The entry code is not first, or was dropped.** Without `KEEP`, the linker
may discard a section that nothing refers to. Without listing `.text.boot`
before `.text`, the entry code lands wherever the linker likes. In both
cases `ENTRY(_start)` is what counts: QEMU starts at the entry address
recorded in the ELF header, not at the first byte of the image.

**Floating point traps.** A slightly larger program hangs before `main`
prints anything. The compiler copied a struct with a SIMD register, and at
reset those registers are disabled, so the instruction raises an exception.
With no vector table (topic 6) the CPU jumps to a low address where there is
no code, and is lost. The fix is three instructions in the entry code; SPEC
section 2.2 names the register.

**Forgetting `.bss`.** Under QEMU, fresh RAM happens to be zero, so a kernel
that does not clear `.bss` appears to work. On real hardware, or after a
warm reset, `static COUNTER: AtomicU64 = AtomicU64::new(0)` then starts with
garbage. Clear it always.

### Read more

- [The GNU linker manual: Scripts](https://sourceware.org/binutils/docs/ld/Scripts.html) and [LLD's notes on linker scripts](https://lld.llvm.org/ELF/linker_script.html) (Rust links this target with LLD).
- [`core::arch::global_asm!`](https://doc.rust-lang.org/core/arch/macro.global_asm.html).
- [The Cargo book: build scripts, `rustc-link-arg`](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg), one way to pass `-T` to the linker.
- [QEMU: `virt` generic virtual platform](https://www.qemu.org/docs/master/system/arm/virt.html).

---

## Topic 3 · Talking to devices: MMIO and volatile access

### The model

A device is controlled through *registers*. With memory-mapped I/O (MMIO) the
registers appear at fixed addresses, and the CPU uses ordinary load and store
instructions on them. But a register is not memory:

- Reading can change the device. Reading a UART's data register removes a
  byte from its receive queue.
- Writing the same value twice can mean two actions (two bytes sent).
- The value can change by itself (a status flag, a clock).

The compiler knows none of this. For normal memory it may remove a read whose
result it already knows, merge two writes, or reorder accesses. For a device
each of these is a bug.

Two rules follow.

**Rule 1: every access is volatile.** `ptr.read_volatile()` and
`ptr.write_volatile(value)` tell the compiler: perform exactly this access,
exactly once, exactly here.

**Rule 2: no references to device memory.** A `&T` promises the compiler
that the memory behind it is readable at any time and does not change while
the reference lives. The compiler is allowed to read through a reference
earlier or more often than your code says. So device registers are reached
only through raw pointers. `&raw const (*ptr).field` and
`&raw mut (*ptr).field` compute the address of one field without creating a
reference.

To keep offsets out of the driver logic, describe the register block as a
`#[repr(C)]` struct. `repr(C)` fixes the field order and padding, so field
offsets match the data sheet, and a constant assertion lets the compiler
check that they do.

### Worked example: a sensor that exists only on paper

The driver below is for an invented temperature sensor. On your computer an
ordinary struct stands in for the device, which is also how a driver can be
tested without hardware.

```rust
use core::mem::offset_of;

/// Register block of the sensor, as its (invented) data sheet lists it.
#[repr(C)]
struct SensorRegisters {
    /// 0x00: write 1 to start a measurement.
    control: u32,
    /// 0x04: bit 0 is set when a value is ready.
    status: u32,
    _reserved: [u32; 2],
    /// 0x10: the measured value, in tenths of a degree.
    data: u32,
}

// The compiler checks the layout against the data sheet.
const _: () = assert!(offset_of!(SensorRegisters, data) == 0x10);

struct Sensor {
    regs: *mut SensorRegisters,
}

impl Sensor {
    /// # Safety
    ///
    /// `regs` must be the address of a sensor register block that stays
    /// valid, and is used by nothing else, for as long as the `Sensor` lives.
    unsafe fn new(regs: *mut SensorRegisters) -> Self {
        Self { regs }
    }

    fn start(&self) {
        // SAFETY: `new` promised a valid register block. `&raw mut` forms
        // the address of one register without creating a reference.
        unsafe { (&raw mut (*self.regs).control).write_volatile(1) }
    }

    fn ready(&self) -> bool {
        // SAFETY: as in `start`.
        unsafe { (&raw const (*self.regs).status).read_volatile() & 1 != 0 }
    }

    fn tenths_of_a_degree(&self) -> u32 {
        // SAFETY: as in `start`.
        unsafe { (&raw const (*self.regs).data).read_volatile() }
    }
}

// A fake device: plain memory with the values a real one would show.
let mut fake = SensorRegisters { control: 0, status: 1, _reserved: [0; 2], data: 215 };
// SAFETY: `fake` outlives `sensor` and is not used while `sensor` is.
let sensor = unsafe { Sensor::new(&raw mut fake) };
sensor.start();
assert!(sensor.ready());
assert_eq!(sensor.tenths_of_a_degree(), 215);
assert_eq!(fake.control, 1, "the driver wrote the start bit");
```

All unsafe code is inside three small methods. Code that uses a `Sensor`
needs no `unsafe` after construction.

The same pattern on a real device. The QEMU `virt` machine has a PL031
real-time clock at `0x0901_0000`; its first register holds the seconds since
1970. (The entry code and a `put` function for one byte are as in topic 2.)

```rust,ignore
/// The registers of the PL031 real-time clock, in data-sheet order.
#[repr(C)]
struct RtcRegisters {
    /// 0x00: seconds since 1970-01-01 (read only).
    data: u32,
    /// 0x04: alarm time.
    match_value: u32,
    /// 0x08: writing a value sets the clock.
    load: u32,
    /// 0x0c: bit 0 enables the clock.
    control: u32,
}

const _: () = assert!(offset_of!(RtcRegisters, control) == 0x0c);

/// The clock's address on the QEMU `virt` machine.
const RTC: *const RtcRegisters = 0x0901_0000 as *const RtcRegisters;

fn seconds_since_1970() -> u32 {
    // SAFETY: `RTC` is the address of a PL031 on this machine. `&raw const`
    // computes the address of one register without creating a reference,
    // and the volatile read happens exactly once.
    unsafe { (&raw const (*RTC).data).read_volatile() }
}
```

Booted in QEMU on 20 November 2026 it printed `6b0019fe`, which is
1 795 168 766 seconds after 1 January 1970.

### Mistakes people really make

**A wrong padding array.** One missing reserved word shifts every later
register. Without the assertion you write to the wrong register and nothing
tells you. With it:

```rust,compile_fail,E0080
use core::mem::offset_of;

#[repr(C)]
struct Registers {
    data: u32,
    status: u32,
    _reserved: [u32; 3], // the data sheet says the next register is at 0x18
    control: u32,
}

const _: () = assert!(offset_of!(Registers, control) == 0x18);
```

```text
error[E0080]: evaluation panicked: assertion failed: offset_of!(Registers, control) == 0x18
  --> src/main.rs:11:15
   |
11 | const _: () = assert!(offset_of!(Registers, control) == 0x18);
   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `_` failed here
```

Two words and three reserved words end at 0x14, not 0x18: the array needs
four elements.

**A plain read in a wait loop.** `while unsafe { *status } & BUSY != 0 {}`
may be compiled into one read followed by an endless loop, because nothing in
the loop writes to `*status`. In a debug build it works; in a release build
it hangs. `read_volatile` forces a fresh read in every round.

**Read-modify-write on a register that is not memory.** `reg |= bit` reads
the register and writes it back. On a "write 1 to clear" register this
clears every flag that happened to be set. Read the register description
before you choose between a plain write and read-modify-write.

### When to use which

- One register, used once (the first `hello`): a bare pointer is fine.
- A real driver: a `repr(C)` block with offset assertions, or one small
  accessor per register. Both give names and types to offsets.
- Bit arithmetic (which bit, which divisor): pure functions that you can test
  on your computer. In the capstone these live in `kcore::reg`.

### Read more

- [`core::ptr::read_volatile`](https://doc.rust-lang.org/core/ptr/fn.read_volatile.html) and [`write_volatile`](https://doc.rust-lang.org/core/ptr/fn.write_volatile.html).
- [The Rust Reference: raw borrow operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#raw-borrow-operators).
- [The Rust Reference: the `C` representation](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation).
- [The Embedded Rust Book: memory-mapped registers](https://doc.rust-lang.org/stable/embedded-book/peripherals/index.html).

---

## Topic 4 · Drivers behind traits

### The model

Firmware outlives hardware. The board changes, the UART chip changes, and the
logging code, the scheduler and the allocator should not notice. The tool for
that is the one you know from chapter 11: a trait says what a *kind* of device
can do, and each driver implements it for one concrete device.

```text
  kernel logic        print!, scheduler, heap, IPC
        |                 uses only traits
  ------+-------------------------------------------
  trait Console    trait Timer    trait InterruptController
        |               |                  |
  PL011 driver    generic timer       GICv2 driver        <- know registers
        |               |                  |
  ------+---------------+------------------+--------
  board file: addresses, interrupt numbers, which driver where
```

Three properties make this layering worth the effort:

1. **Portability.** A new board is a new board file and perhaps new drivers.
2. **Testability.** Logic that takes `&dyn Timer` can be run on your computer
   with a fake timer.
3. **A small unsafe surface.** Only the drivers touch registers.

A driver is usually stored in a `static`, because it exists for the whole
run and interrupt handlers need to reach it. Two consequences:

- Its methods take `&self`, not `&mut self`. Hardware state is changed
  through register writes, which need no `&mut`. State on the Rust side
  (a counter, a buffer) needs interior mutability: an atomic, or a lock.
- Its type must be `Sync`. A struct that holds a raw pointer is not `Sync`
  automatically. You have to state it with `unsafe impl Sync` and explain why
  sharing is sound (topic 10).

### Worked example: logic tested against a fake device

```rust
use std::cell::Cell;

/// What the rest of the system may ask of any clock.
trait Clock {
    /// Counter increments per second.
    fn frequency_hz(&self) -> u64;
    /// The counter value now.
    fn now(&self) -> u64;
}

/// Hardware-independent logic: works with every `Clock`.
fn elapsed_ms(clock: &dyn Clock, since: u64) -> u64 {
    (clock.now() - since) * 1000 / clock.frequency_hz()
}

/// A clock for tests. Every reading is 250 counts later than the last.
struct FakeClock {
    count: Cell<u64>,
}

impl Clock for FakeClock {
    fn frequency_hz(&self) -> u64 {
        1000
    }
    fn now(&self) -> u64 {
        self.count.set(self.count.get() + 250);
        self.count.get()
    }
}

let clock = FakeClock { count: Cell::new(0) };
let start = clock.now(); // 250
assert_eq!(elapsed_ms(&clock, start), 250); // the second reading is 500
assert_eq!(elapsed_ms(&clock, start), 500);
```

`FakeClock` uses `Cell`, which is fine in a single-threaded test. A real
driver in a `static` needs a `Sync` type instead.

### Mistakes people really make

**A driver with a raw pointer in a `static`.**

```rust,compile_fail,E0277
struct Uart {
    base: *mut u32,
}

static UART: Uart = Uart { base: 0x0900_0000 as *mut u32 };
```

```text
error[E0277]: `*mut u32` cannot be shared between threads safely
 --> src/main.rs:5:14
  |
5 | static UART: Uart = Uart { base: 0x0900_0000 as *mut u32 };
  |              ^^^^ `*mut u32` cannot be shared between threads safely
  |
  = help: within `Uart`, the trait `Sync` is not implemented for `*mut u32`
  = note: shared static variables must have a type that implements `Sync`
```

The compiler is right to ask. On bare metal "threads" means: tasks *and
interrupt handlers*. Fix: write `unsafe impl Sync for Uart {}` with a comment
that says what happens when a task and an interrupt handler use the device at
the same moment, and why that is acceptable. Do not reach for `static mut`
to silence the error; that removes the question instead of answering it.

**`static mut` and a reference to it.** In edition 2024 this is an error by
default:

```rust,compile_fail
static mut TICKS: u64 = 0;

fn ticks() -> u64 {
    let ticks = unsafe { &TICKS };
    *ticks
}
```

```text
error: creating a shared reference to mutable static
 --> src/main.rs:4:26
  |
4 |     let ticks = unsafe { &TICKS };
  |                          ^^^^^^ shared reference to mutable static
  |
  = note: shared references to mutable statics are dangerous; it's undefined behavior if the
          static is mutated or if a mutable reference is created for it while the shared
          reference lives
  = note: `#[deny(static_mut_refs)]` (part of `#[deny(rust_2024_compatibility)]`) on by default
```

A counter that an interrupt handler increments is exactly the case the note
describes. Use `AtomicU64`.

**A trait that leaks the device.** If the console trait has a method
`set_pl011_fifo_level`, it is not an abstraction. Put only what every device
of the kind can do in the trait; keep the rest as methods of the driver type,
called from the board file.

### When to use which

| Choice | Use it when |
| --- | --- |
| `&dyn Console` | The driver is chosen in one place and passed around. One copy of the code, a pointer-sized handle. The usual choice in a kernel. |
| `fn log<C: Console>(c: &C)` | The call is hot and you want it inlined. The cost is one copy of the code per driver type. |
| A plain function without a trait | There is only ever one implementation (for example "power off" on one board). Do not abstract what does not vary. |

### Read more

- [The Rust Reference: trait objects](https://doc.rust-lang.org/reference/types/trait-object.html).
- [`Sync`](https://doc.rust-lang.org/core/marker/trait.Sync.html) and [the Rustonomicon on `Send` and `Sync`](https://doc.rust-lang.org/nomicon/send-and-sync.html).
- [The Edition Guide: disallow references to `static mut`](https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html).
- [The Embedded Rust Book: portability](https://doc.rust-lang.org/stable/embedded-book/portability/index.html).

---

## Topic 5 · A heap from nothing

### The model

`Box::new(x)` compiles into a call of the *global allocator*: a `static`
marked `#[global_allocator]` whose type implements
`core::alloc::GlobalAlloc`. The trait has two required methods:

- `alloc(&self, layout) -> *mut u8` returns a block of `layout.size()` bytes
  whose address is a multiple of `layout.align()`, or null.
- `dealloc(&self, ptr, layout)` takes a block back. The caller passes the
  same layout again, so an allocator does not have to remember block sizes.

A kernel gets its heap memory from the linker script: a range of RAM that
nothing else uses. The allocator's job is to cut that range into blocks and
to take blocks back. Three designs, from simple to useful:

**Bump.** Keep one pointer. To allocate, round it up to the alignment, return
it, and move it forward by the size. Freeing does nothing. Very fast and only
a few lines, but memory comes back only when everything is freed at once.

**Pool.** Cut the range into blocks of one size and keep the free ones in a
list. Allocating and freeing are both a single list operation, and there is
no fragmentation. But it serves only one block size.

**Free list.** Keep a list of free ranges of any size. To allocate, search
for a range that is large enough and cut the block out of it (*splitting*).
To free, put the range back and join it with free neighbours (*merging*, also
called coalescing). Without merging the heap decays into crumbs that are
each too small for a large request:

```text
after freeing a and c, with b still live:
  [ free 32 ][  b: used  ][ free 32 ][ ... free ... ]

after freeing b, without merging:      a request for 80 bytes fails
  [ free 32 ][  free 32  ][ free 32 ][ ... ]

after freeing b, with merging:         one range again
  [             free 96             ][ ... ]
```

The trick that all list designs share: a free block is unused memory, so the
list node can be stored *inside the free block itself*. The allocator needs
no memory of its own.

### Worked example: a pool allocator

The capstone asks for a bump and a free-list allocator. This is the third
kind, small enough to read in one go. It runs on your computer on top of a
plain array.

```rust
use core::ptr;

/// Bytes per block. A block must be able to hold a `FreeBlock`.
const BLOCK: usize = 32;

/// What the allocator writes into a block while the block is free.
#[repr(C)]
struct FreeBlock {
    next: *mut FreeBlock,
}

/// A pool of equal blocks, linked through the free blocks themselves.
struct Pool {
    head: *mut FreeBlock,
}

impl Pool {
    /// # Safety
    ///
    /// `start .. start + len` must be valid for reads and writes, aligned
    /// for `FreeBlock`, and used by nothing else while the pool is in use.
    unsafe fn new(start: *mut u8, len: usize) -> Self {
        let mut head = ptr::null_mut();
        for index in 0..len / BLOCK {
            // SAFETY: block `index` lies inside the region (see `new`'s contract).
            let block = unsafe { start.add(index * BLOCK) }.cast::<FreeBlock>();
            // SAFETY: the block is free, aligned and large enough for a node.
            unsafe { block.write(FreeBlock { next: head }) };
            head = block;
        }
        Self { head }
    }

    /// Returns one block, or null when the pool is empty.
    fn alloc(&mut self) -> *mut u8 {
        let block = self.head;
        if !block.is_null() {
            // SAFETY: every pointer in the list refers to a free block that
            // holds an initialised node.
            self.head = unsafe { (*block).next };
        }
        block.cast()
    }

    /// # Safety
    ///
    /// `block` must come from `alloc` on this pool and must not be freed twice.
    unsafe fn free(&mut self, block: *mut u8) {
        let block = block.cast::<FreeBlock>();
        // SAFETY: the caller returns a whole block, so the node fits.
        unsafe { block.write(FreeBlock { next: self.head }) };
        self.head = block;
    }
}

let mut memory = [0_u64; 16]; // 128 bytes, aligned to 8
// SAFETY: the array is valid, aligned and not used directly while `pool` is.
let mut pool = unsafe { Pool::new(memory.as_mut_ptr().cast(), 128) };

let blocks = [pool.alloc(), pool.alloc(), pool.alloc(), pool.alloc()];
assert!(blocks.iter().all(|block| !block.is_null()), "128 / 32 = 4 blocks");
assert!(pool.alloc().is_null(), "the fifth request fails");

// SAFETY: `blocks[1]` came from this pool and is freed once.
unsafe { pool.free(blocks[1]) };
assert_eq!(pool.alloc(), blocks[1], "a freed block is handed out again");
```

Note where the pointers come from. Every block pointer is computed from
`start` with `add`. None is made from an integer. That keeps the *provenance*
of the original pointer (module [`m2_unsafe`](../../mastery/m2_unsafe/README.md)), and it is why Miri accepts the
code. When you need to change the address of a pointer, for example to round
it up to an alignment, use `ptr.with_addr(new_address)` or
`ptr.map_addr(|a| ...)`. A cast such as `address as *mut u8` creates a pointer
without that link, and Miri in strict mode rejects it.

### Mistakes people really make

**Ignoring alignment.** A bump allocator that returns its pointer without
rounding works for `u8` and breaks for `u64`. Round up with
`(addr + align - 1) & !(align - 1)`; the alignment is always a power of two.

**Overflow in the size check.** `if next + size > end` wraps around for a
huge `size` and then passes. Use `checked_add` and treat `None` as "does not
fit". The host tests ask for an absurd size on purpose.

**A remainder too small for a node.** After splitting, the unused rest of a
free range must be able to hold a list node. If it cannot, you either write a
node past the end of the range, or you lose the bytes for ever. A common cure
is a minimum block size: round every size and every alignment up to the size
of a node, so that every gap is either empty or large enough.

**Keeping the pointer that `dealloc` was given.** The pool above stores the
caller's pointer in its list. That is sound as long as the caller hands back
the very pointer that `alloc` returned, as the example does. A global
allocator cannot count on that. `Box<u8>` gives its block back through a
pointer that went through the box, and under the rules that Miri checks such
a pointer may access one byte: the `u8`. A list node written through it is
larger, and so is the free range after a merge with a neighbour. In `dealloc`,
take only the address from the caller's pointer and make your own pointer
from the region pointer: `start.with_addr(ptr.addr())`.

**Using the heap inside the allocator.** `alloc` must not allocate. A `Vec`
of free ranges inside the allocator calls `alloc` from inside `alloc`.

**Forgetting interrupts.** If an interrupt handler allocates while a task is
in the middle of changing the free list, the list is corrupt. The algorithm
(in `kcore`) takes `&mut self` and does not know about interrupts. The
`GlobalAlloc` wrapper in the kernel is the place to keep them out.

### When to use which

| Allocator | Use it when |
| --- | --- |
| Bump | Early boot, or work in phases that end by freeing everything. |
| Pool | Many objects of one size: task structures, message buffers, network packets. |
| Free list | A general heap for `Box`, `Vec` and `String`. |

Many real kernels use all three: a bump allocator until the heap is ready,
pools for the common sizes, and a general allocator behind them.

### Read more

- [`core::alloc::GlobalAlloc`](https://doc.rust-lang.org/core/alloc/trait.GlobalAlloc.html) and [`Layout`](https://doc.rust-lang.org/core/alloc/struct.Layout.html).
- [The `alloc` crate](https://doc.rust-lang.org/alloc/).
- [`core::ptr`: provenance](https://doc.rust-lang.org/core/ptr/index.html#provenance), with `addr`, `with_addr` and `map_addr`.
- [The Rustonomicon: `#[global_allocator]` and `no_std`](https://doc.rust-lang.org/nomicon/beneath-std.html).

---

## Topic 6 · Exception levels and the vector table

### The model

An AArch64 CPU runs at one of four **exception levels**. A higher level has
more rights:

| Level | Who runs there |
| --- | --- |
| EL0 | applications |
| EL1 | the operating system kernel (your code) |
| EL2 | a hypervisor |
| EL3 | secure firmware |

An **exception** is any event that makes the CPU stop what it is doing and
jump into more privileged code. There are four kinds:

- **Synchronous**: caused by the instruction that just ran. It can be asked
  for (`svc`, the system call instruction; `brk`, a breakpoint) or be an
  error (an access to an address that does not exist, an undefined
  instruction).
- **IRQ**: a device wants attention (topic 7).
- **FIQ**: a second interrupt line, not used here.
- **SError**: an asynchronous hardware error.

When an exception is taken to EL1, the CPU does very little, and all of it in
hardware:

1. It copies the processor state (flags, interrupt masks, the current level)
   to `SPSR_EL1`.
2. It copies the return address to `ELR_EL1`.
3. It masks interrupts.
4. For a synchronous exception it writes the cause to `ESR_EL1` (and, for a
   bad memory access, the address to `FAR_EL1`).
5. It jumps to an entry of the **vector table**.

The vector table is not a table of addresses. It is code: sixteen slots of
128 bytes each, starting at the address in `VBAR_EL1`. Which slot is used
depends on where the exception came from (four groups) and on its kind (four
kinds):

```text
offset  came from                          kind
0x000   EL1, while using stack SP_EL0      synchronous
0x080                                      IRQ
0x100                                      FIQ
0x180                                      SError
0x200   EL1, while using stack SP_EL1      synchronous   <- a kernel that takes
0x280                                      IRQ           <- exceptions in itself
0x300                                      FIQ
0x380                                      SError
0x400   a lower level, 64-bit code         synchronous   <- system calls from EL0
 ...                                       ...
0x600   a lower level, 32-bit code         ...
```

The instruction `eret` undoes steps 1 and 2: it restores the state from
`SPSR_EL1` and continues at `ELR_EL1`.

The CPU saves no general register. The interrupted code was in the middle of
something, with values in x0 to x30 and in the floating-point registers, and
it must find them unchanged when it continues. So a handler that wants to
*return* must first store every register it might change, and since it calls
compiled Rust code, that is every register the calling convention lets a
function change, plus `ELR_EL1` and `SPSR_EL1` if another exception can
happen meanwhile. The saved block is called a **trap frame**. Slots are only
32 instructions long, so each one usually just jumps to shared code.

### Worked example: making exceptions visible

Before a kernel can *handle* exceptions it should be able to *see* them.
Without a vector table, `VBAR_EL1` is 0 in QEMU at reset (on real hardware
its reset value is not defined), and any exception jumps into memory that
holds no code. That raises another exception, for ever. The
machine looks dead.

This table does the minimum: every slot calls one function that prints the
cause and stops. It saves nothing, so it cannot return. (Entry code and `put`
as in topic 2; `put_hex` prints 16 hexadecimal digits.)

```rust,ignore
// A vector table whose sixteen entries all do the same thing: call
// `report`, which never returns. Nothing is saved, so this table can only
// *show* an exception, not resume from it.
global_asm!(
    r#"
    .section .text.vectors, "ax"
    .balign 0x800               // the table address must be a multiple of 2048
    .global vector_table
vector_table:
    .rept 16
    .balign 0x80                // one entry every 128 bytes
    b       report
    .endr
"#
);

#[unsafe(no_mangle)]
extern "C" fn report() -> ! {
    let (syndrome, return_address): (u64, u64);
    // SAFETY: both registers are readable at EL1 and reading has no effect.
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) syndrome, options(nomem, nostack));
        asm!("mrs {}, elr_el1", out(reg) return_address, options(nomem, nostack));
    }
    put_hex(syndrome >> 26); // bits 31:26: the exception class
    put_hex(return_address);
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() {
    // SAFETY: `vector_table` is a 2 KiB-aligned table of sixteen valid
    // entries. `brk` then raises a synchronous exception on purpose.
    unsafe {
        asm!(
            "adrp {t}, vector_table",
            "add  {t}, {t}, :lo12:vector_table",
            "msr  vbar_el1, {t}",
            "isb",
            "brk  #7",
            t = out(reg) _,
            options(nomem, nostack),
        );
    }
}
```

Output of a debug build in QEMU:

```text
000000000000003c
00000000401010fc
```

Class `0x3c` is "breakpoint instruction", and the second number is the
address of the `brk` itself. `mrs` reads a system register into a general
register and `msr` writes one. `isb` makes sure the new `VBAR_EL1` is in
effect before the next instruction runs.

Note the return address. For `brk` it is the address of the instruction that
caused the exception. Returning with `eret` would run the `brk` again. For
`svc` the CPU stores the address of the *next* instruction, because a system
call should continue after the call. A handler must know which rule applies
to the exception it handles.

### Mistakes people really make

**A table that is not aligned.** `VBAR_EL1` ignores its low 11 bits. If the
table is at an address that is not a multiple of 2048, the CPU jumps into the
middle of something else. Use `.balign 0x800`, and check the address in the
linker map or with a debugger.

**Saving too little.** A handler written in Rust may use any caller-saved
register, including the SIMD registers: on this target the compiler uses them
for copies. A trap frame without them works for weeks, and then a `memcpy` in
a handler corrupts a float in a task. Either save them, or prove that no code
reached from the handler uses them.

**A stack pointer that is not a multiple of 16.** A frame of 33 registers is
264 bytes, which leaves the stack pointer 8 bytes off. A real CPU checks the
alignment of the stack pointer (SPEC 2.2) and raises an exception at the
first load or store through it. QEMU does not make that check, so there the
handler runs on until compiled code stores a 16-byte SIMD register on the
stack. With the MMU off that unaligned store raises a data abort inside the
exception handler (QEMU reports `ESR_EL1` = `0x96000061`). Round the frame
size up.

**Forgetting what the CPU masked.** On entry, interrupts are masked. If the
handler unmasks them (or calls code that does) before the frame is complete,
a second exception overwrites `ELR_EL1` and `SPSR_EL1`, and the first return
goes to the wrong place.

### Read more

- [Learn the architecture: AArch64 Exception Model](https://developer.arm.com/documentation/102412/latest/) (Arm; short and readable).
- [Arm Architecture Reference Manual for A-profile](https://developer.arm.com/documentation/ddi0487/latest/): the register descriptions of `ESR_EL1`, `VBAR_EL1`, `SPSR_EL1`.
- [The Rust Reference: inline assembly](https://doc.rust-lang.org/reference/inline-assembly.html).
- [QEMU invocation: `-d int`](https://www.qemu.org/docs/master/system/invocation.html) logs every exception the guest takes.

---

## Topic 7 · Interrupts, the GIC and the timer

### The model

An interrupt is a device saying "something happened". The path has three
stations:

```text
device                interrupt controller (GIC)             CPU
timer   --line 27-->  distributor: enabled? priority?  -->  IRQ input
UART    --line 33-->  CPU interface: more urgent than       masked by
 ...                  the priority mask?                     the I bit?
```

1. The **device** raises its line. The timer does so when its counter reaches
   the compare value.
2. The **interrupt controller** collects all lines. For each interrupt number
   (INTID) it knows whether the interrupt is enabled and how urgent it is. If
   an enabled interrupt is pending and urgent enough, it raises the CPU's IRQ
   input.
3. The **CPU** takes an IRQ exception (topic 6), unless the I bit in its
   state masks IRQs. Then the interrupt simply stays pending.

The handler then walks the path backwards:

1. **Claim.** Read the acknowledge register of the controller. It returns the
   INTID and marks that interrupt as "being handled".
2. **Serve the device**, so that it lowers its line. This step is the one
   people forget.
3. **Complete.** Write the INTID to the end-of-interrupt register. Now the
   controller may signal the next interrupt of the same urgency.

Step 2 matters because most lines are **level-triggered**: the line means "the
condition is true *now*", not "an event happened once". The timer's line is
high as long as the counter is at or past the compare value. If the handler
completes the interrupt without moving the compare value, the line is still
high, and the same interrupt fires again the moment the handler returns.

### Worked example: a level-triggered line, simulated

This model has the same rules as the generic timer: a counter that only
grows, a compare value, and a line that is high while `counter >= compare`.
It shows the difference between a handler that only counts and one that
serves the device.

```rust
struct TimerModel {
    counter: u64,
    compare: u64,
}

impl TimerModel {
    /// The interrupt line: a level, not an event.
    fn line_is_high(&self) -> bool {
        self.counter >= self.compare
    }
}

/// Runs the "CPU" for `steps` counter increments and returns how often the
/// handler ran. The handler runs whenever the line is high.
fn run(steps: u64, mut handler: impl FnMut(&mut TimerModel)) -> u64 {
    let mut timer = TimerModel { counter: 0, compare: 10 };
    let mut handled = 0;
    for _ in 0..steps {
        timer.counter += 1;
        if timer.line_is_high() {
            handler(&mut timer);
            handled += 1;
        }
    }
    handled
}

// A handler that forgets the device: the line never goes low again.
let storm = run(100, |_timer| {});
assert_eq!(storm, 91, "from count 10 on, the handler runs at every step");

// A handler that moves the compare value one interval into the future.
let periodic = run(100, |timer| timer.compare = timer.counter + 10);
assert_eq!(periodic, 10, "one interrupt every 10 counts");
```

On real hardware the "storm" is worse than in the model: the CPU does nothing
but enter and leave the handler, and the interrupted code never runs again.

### Shared state and critical sections

An interrupt can arrive between any two instructions of a task. If both the
task and the handler use the same data, the handler may see it half-changed.
There are two tools:

- **Atomics** for single values. A tick counter is an `AtomicU64`; the handler
  adds, tasks read.
- A **critical section** for anything larger: mask IRQs, do the work, restore
  the previous mask. On one CPU core nothing else can run in between, so the
  code has exclusive access. *Restore*, not *unmask*: the code may have been
  called with interrupts already masked, and unmasking would break its
  caller's critical section. A guard object that saves the old state when it
  is created and writes it back in `Drop` gets this right even on early
  returns.

Keep critical sections short. While one runs, the machine is deaf.

Printing is shared state too. A line that a task is half-way through, mixed
with a line from an interrupt handler, is the first symptom most people see.

### Mistakes people really make

**The interrupt is enabled in only one of four places.** A timer interrupt
arrives only if (1) the timer is enabled and its interrupt is not masked in
the timer's own control register, (2) the INTID is enabled in the
distributor, (3) the distributor and the CPU interface are switched on and
the priority mask lets the interrupt through, and (4) the CPU's I bit is
clear. If nothing happens, check all four. QEMU's `-d int` log shows whether
the CPU took an exception at all.

**Sleeping after the wake-up.** This loop can sleep one period too long:

```text
if ticks == seen {        <- tick arrives here, after the check
    wfi                   <- and the CPU sleeps until the next one
}
```

For a periodic timer that is only a delay. For a one-time event it is a hang.
The robust form is: mask IRQs, check, `wfi`, unmask. `wfi` wakes up for a
pending interrupt even when it is masked, and the handler runs right after
the unmask.

**Completing the interrupt too late.** If the handler switches to another
task (topic 8) before it has completed the interrupt at the controller, the
controller waits for a completion that may not come for a long time, and
delivers no further timer interrupt meanwhile. Complete first, then switch.

**Hard-coding the timer frequency.** The counter frequency is a property of
the machine. Read it from `CNTFRQ_EL0`. QEMU reports different values for
different CPU models.

### Read more

- [Arm Generic Interrupt Controller Architecture Specification, version 2](https://developer.arm.com/documentation/ihi0048/latest/): chapter 3 (interrupt handling and prioritization) and chapter 4 (the registers).
- [Arm Architecture Reference Manual for A-profile](https://developer.arm.com/documentation/ddi0487/latest/): the Generic Timer chapter.
- [`core::sync::atomic`](https://doc.rust-lang.org/core/sync/atomic/).
- [The Embedded Rust Book: concurrency](https://doc.rust-lang.org/stable/embedded-book/concurrency/index.html) (written for microcontrollers; the reasoning about interrupts and critical sections is the same).

---

## Topic 8 · Context switching

### The model

A **task** is a flow of control with its own stack. On one CPU core only one
task runs; the others are frozen. A frozen task is nothing more than

- its stack, with all the local variables and return addresses on it, and
- a small block of saved registers, its **context**.

A **context switch** freezes the running task and thaws another. The
surprising part is how few registers that takes when the switch is an
ordinary function call. The calling convention (AAPCS64) divides the
registers in two groups:

| Group | Registers | Rule |
| --- | --- | --- |
| caller-saved | x0 to x18, most of the SIMD registers | Any call may change them. Code that needs them after a call has already saved them on its stack. |
| callee-saved | x19 to x28, x29 (frame pointer), the stack pointer, d8 to d15 | A function must leave them as it found them. |
| return address | x30 (link register) | The call instruction `bl` puts the return address here, and `ret` jumps to it. The caller does not expect x30 to survive the call. |

A task that calls `switch(from, to)` expects, like after any call, that the
caller-saved registers are gone and the callee-saved ones are intact. So
`switch` needs to store only the callee-saved group in `*from`, and x30: the
address at which this task will continue. It then loads the same registers
from `*to`, which include the other task's stack pointer and return address,
and returns. That `ret` does not return to the caller. It
returns into the other task, at the point where *that* task once called
`switch`.

```text
task A                         task B (frozen inside its own call of switch)
  ...
  switch(&A, &B) --+
                   | store x19..x30, sp in A
                   | load  x19..x30, sp from B
                   +---- ret ----> ... continues after B's call of switch
                                   ...
        <---- ret ---------------- switch(&B, &A)
  continues here
```

A **new task** has never called `switch`, so there is nothing to return into.
The creator fakes it: it allocates a stack and writes a context whose stack
pointer is the top of that stack and whose return address is the task's entry
function. The first switch to the task "returns" into the entry function.

**Pre-emption** is the same switch, called from the timer interrupt handler.
The interrupted task did not expect to lose any register, and that is why the
exception entry saved the full trap frame on the task's stack (topic 6). The
handler then calls `switch` like any other function. The task is now frozen
*inside the interrupt handler*. When it is thawed, `switch` returns, the
handler returns, the entry code restores the trap frame and `eret` continues
the task where the timer struck. Two layers, each saving what its caller did
not expect to lose.

### Worked example: two flows of control taking turns

No scheduler, no interrupts: `main` and one worker hand the CPU to each other.
(Entry code as in topic 2; `put` prints a string.)

```rust,ignore
/// What a suspended flow of control needs to continue: the registers that
/// a function must preserve for its caller, and the stack pointer.
#[repr(C)]
struct Context {
    /// x19..x28, then x29 (frame pointer) and x30 (return address).
    callee_saved: [u64; 12],
    sp: u64,
}

/// Stores the running context in `*from` and continues the one in `*to`.
///
/// # Safety
///
/// Both pointers must be valid, and `*to` must hold a context that was
/// stored by this function or prepared like `WORKER` below.
#[unsafe(naked)]
unsafe extern "C" fn switch(from: *mut Context, to: *const Context) {
    naked_asm!(
        "stp x19, x20, [x0, #0]",
        "stp x21, x22, [x0, #16]",
        "stp x23, x24, [x0, #32]",
        "stp x25, x26, [x0, #48]",
        "stp x27, x28, [x0, #64]",
        "stp x29, x30, [x0, #80]",
        "mov x9, sp",
        "str x9, [x0, #96]",
        "ldp x19, x20, [x1, #0]",
        "ldp x21, x22, [x1, #16]",
        "ldp x23, x24, [x1, #32]",
        "ldp x25, x26, [x1, #48]",
        "ldp x27, x28, [x1, #64]",
        "ldp x29, x30, [x1, #80]",
        "ldr x9, [x1, #96]",
        "mov sp, x9",
        "ret", // jumps to the x30 that was just loaded
    )
}

const EMPTY: Context = Context {
    callee_saved: [0; 12],
    sp: 0,
};
static mut MAIN: Context = EMPTY;
static mut WORKER: Context = EMPTY;
static mut WORKER_STACK: [u8; 4096] = [0; 4096];

extern "C" fn worker() -> ! {
    loop {
        put("  worker runs\n");
        // SAFETY: both contexts are valid statics; `MAIN` was stored by the
        // switch that brought us here. Only one flow of control runs at a time.
        unsafe { switch(&raw mut WORKER, &raw const MAIN) };
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() {
    // SAFETY: nothing else runs yet, so nothing else uses these statics.
    // The prepared context makes the first switch "return" into `worker`
    // with an empty, 16-byte aligned stack of its own.
    unsafe {
        let stack_top = (&raw mut WORKER_STACK).cast::<u8>().add(4096);
        WORKER.callee_saved[11] = worker as *const () as u64; // x30
        WORKER.sp = stack_top as u64 & !0xf;
    }
    for _ in 0..3 {
        put("main runs\n");
        // SAFETY: as in `worker`.
        unsafe { switch(&raw mut MAIN, &raw const WORKER) };
    }
    put("done\n");
}
```

Output in QEMU:

```text
main runs
  worker runs
main runs
  worker runs
main runs
  worker runs
done
```

How to read the assembly: `stp a, b, [x0, #n]` stores the pair of registers
`a` and `b` at address `x0 + n`; `ldp` loads a pair. The first two arguments
of a function arrive in x0 and x1. The stack pointer cannot be stored
directly, so it goes through x9, a caller-saved scratch register. A *naked*
function is one for which the compiler generates no code of its own; the body
is exactly the assembly you write. That is essential here, because compiler
code before the first `stp` could already have changed the stack pointer.

`worker` is "in the middle of its loop" between switches. Its loop counter,
if it had one, would live on `WORKER_STACK` or in a callee-saved register,
and it would survive.

This example is deliberately incomplete: it does not save d8 to d15, it has
exactly two contexts in `static mut` variables, a worker that may never
return, and no interrupts anywhere. The capstone asks for the general
version.

### Mistakes people really make

**The lock that travels.** If a task takes a lock, switches away, and the
next task wants the same lock, the system is stuck: the owner is frozen. With
interrupt-masking locks the bug looks different. The lock is "released" by a
task that never took it, and the mask state of one task leaks into another.
Decide what is held across a switch (as little as possible) and write it
down.

**New tasks start with the wrong interrupt mask.** A switch is made with
interrupts masked. An existing task restores its own mask state when it
continues, because that state is in its saved registers or on its stack. A
new task has no saved state: unless its first instructions unmask interrupts,
it runs with them masked for ever and can never be pre-empted.

**Freeing a stack that is in use.** A task cannot free its own stack while it
is still running on it. Another task has to do it after the task has ended.

**A lost wake-up.** "Check a condition, then block" must be one step with
respect to whoever sets the condition. If the waker runs between the check
and the block, it wakes a task that is not blocked yet, and the task then
blocks for ever.

### Read more

- [Procedure Call Standard for the Arm 64-bit Architecture (AAPCS64)](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst): section "General-purpose Registers" and "SIMD and Floating-Point Registers".
- [The Rust Reference: the `naked` attribute](https://doc.rust-lang.org/reference/attributes/codegen.html#the-naked-attribute).
- [`core::arch::naked_asm!`](https://doc.rust-lang.org/core/arch/macro.naked_asm.html).

---

## Topic 9 · Message passing

### The model

In a *monolithic* kernel the file system, the network stack and the drivers
are all kernel code, and they call each other's functions. In a *microkernel*
the kernel keeps only what must be privileged: address spaces, tasks, and
**inter-process communication (IPC)**. Everything else runs as ordinary
tasks, called **servers**, and other tasks reach a server by sending it a
message. A bug in a server cannot corrupt the kernel, and a crashed server
can be restarted. The price is that every request crosses the kernel twice.

The classic IPC design is **synchronous** and has three operations:

| Operation | Called by | Meaning |
| --- | --- | --- |
| send | client | "Here is a request." The client stops until the reply is there. |
| receive | server | "Give me the next request." The server stops until one arrives. It learns who sent it. |
| reply | server | "This is the answer for that client." The client continues. |

Synchronous means that there is no queue of messages that grows without
bound. A client that sends is blocked, so it cannot send a second request.
The message is *copied* from the client to the server. No pointer crosses,
so the server never has to trust the client's memory.

From the client's point of view a call goes through these stages:

```text
          send                 server receives             server replies
running ---------> send-blocked ----------------> reply-blocked ---------> running
        (request is queued)       (request is with the server)
```

The meeting point is called an **endpoint**. Clients know the endpoint, not
the server's identity.

### State machines as plain code

The kernel side of IPC is bookkeeping: who waits for what. That can be
written as a data structure with no blocking in it. Each method changes the
state and *returns what the caller must do next*: block this task, wake that
one. The code that really blocks and wakes, the part that needs a scheduler
and a context switch, is a thin layer on top. The pure part can be tested on
your computer with every order of events you can think of, including the
orders that are hard to produce on a real machine.

### Worked example: a mailbox for one number

This is not the endpoint of the capstone. It is the smallest thing with the
same shape: a state machine whose methods return instructions.

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
struct Task(u32);

/// What the kernel must do after an operation on the mailbox.
#[derive(Debug, PartialEq)]
enum Action {
    /// Nothing; the calling task keeps running.
    Continue,
    /// Block the calling task.
    Block,
    /// Make this task ready again.
    Wake(Task),
}

/// Holds at most one letter. At most one task waits for it.
#[derive(Default)]
struct Mailbox {
    letter: Option<u64>,
    waiting: Option<Task>,
}

impl Mailbox {
    fn post(&mut self, letter: u64) -> Action {
        self.letter = Some(letter);
        match self.waiting.take() {
            Some(task) => Action::Wake(task),
            None => Action::Continue,
        }
    }

    /// The caller tries again after it was woken.
    fn fetch(&mut self, me: Task) -> (Option<u64>, Action) {
        match self.letter.take() {
            Some(letter) => (Some(letter), Action::Continue),
            None => {
                self.waiting = Some(me);
                (None, Action::Block)
            }
        }
    }
}

let reader = Task(7);
let mut mailbox = Mailbox::default();

// The reader comes first and finds nothing: the kernel has to block it.
assert_eq!(mailbox.fetch(reader), (None, Action::Block));
// A letter arrives: the kernel has to wake the reader ...
assert_eq!(mailbox.post(99), Action::Wake(reader));
// ... which tries again and now finds the letter.
assert_eq!(mailbox.fetch(reader), (Some(99), Action::Continue));

// The other order: the letter comes first, nobody has to wait.
assert_eq!(mailbox.post(5), Action::Continue);
assert_eq!(mailbox.fetch(reader), (Some(5), Action::Continue));
```

The pattern "try; if nothing, register and block; when woken, try again" is
worth remembering. The woken task does not assume that it got what it waited
for. It checks.

### Mistakes people really make

**Passing a pointer in a message.** Between kernel tasks that share one
address space this works, which is what makes it tempting. It stops working
the day the server gets its own address space, and it lets a client make the
server read anything. Copy the bytes.

**Trusting the request.** A server must check every field that comes from a
client: lengths, indices, labels. "The client is my own code" is true only
until it is not.

**Blocking after the wake-up.** The client queues its request (which may wake
the server) and then blocks. If the server runs in between, replies and
"wakes" a client that is not blocked yet, the wake-up is lost and the client
sleeps for ever. This is the lost wake-up of topic 8, and the cure is the
same.

**Deadlock by design.** Server A calls server B while B calls A. Each waits
for a reply that the other can never send. Draw the "who calls whom" graph;
it must have no cycle.

### When to use which

| Style | Use it when |
| --- | --- |
| Synchronous send/receive/reply | Request and response belong together, and you want simple flow control. |
| Asynchronous messages with a queue | The sender must not wait (an interrupt handler telling a driver task that data arrived). Needs a bounded queue and an answer to "what if it is full?". |
| Shared memory plus a notification | Large data. The message only says where the data is. |

### Read more

- [The seL4 Reference Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf), chapter "Message Passing (IPC)": how a production microkernel defines endpoints, send, receive and reply.
- [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/), the asynchronous-queue style in the standard library, for comparison.
- Mastery module [`m4_async`](../../mastery/m4_async/README.md) builds a scheduler for futures with the same "ready queue and wake-up" structure.

---

## Topic 10 · Where the unsafe code lives

### The model

A kernel cannot avoid `unsafe`. It executes instructions that Rust has no
syntax for, writes to addresses that no Rust object owns, and switches
stacks. The goal is not zero unsafe code. The goal is that every unsafe
operation is in a small place with a name, a contract and a reason, and
that everything else is safe code that cannot break the kernel's memory
whatever it does.

A useful map of where unsafe code is *necessary*:

| Place | Why unsafe is needed | What makes it sound |
| --- | --- | --- |
| Entry code, vector table | The CPU's own entry conventions; must be assembly. | The linker script and the hardware manual. Keep it tiny. |
| Register access | Writes to raw addresses. | The address is a real device (a fact about the board), the access is volatile, and no reference to device memory exists. |
| System registers, `wfi`, `svc` | Instructions without Rust syntax (`asm!`). | Each wrapper does one thing and declares the registers it changes. |
| Context switch | Replaces the stack pointer. | Both contexts are valid, and nothing observes the half-switched state. |
| Allocator | Hands out raw memory; stores list nodes in free memory. | The region belongs to the allocator alone. Tested on the host, checked by Miri. |
| `unsafe impl Sync`, `unsafe impl GlobalAlloc` | A promise the compiler cannot check. | A written argument: who can access the value, and what excludes them from each other. |
| Symbols (`no_mangle`, `extern` statics from the linker script) | Names that cross the boundary of the Rust type system. | The name is unique; the type declared for an extern symbol matches how it is used. |

And where it is *not* necessary: the scheduler's queue, the IPC state machine,
register arithmetic, formatting, the code that decides what to do in an
interrupt, and the servers. If you find `unsafe` there, a safe abstraction is
missing one level below.

Edition 2024 marks more of the boundary than earlier editions did:
`unsafe extern "C" { ... }` for declarations, `#[unsafe(no_mangle)]` for
attributes that can break the program at link time, and the lint
`unsafe_op_in_unsafe_fn`, which makes you write an `unsafe` block (and so a
`SAFETY` comment) even inside an `unsafe fn`.

### Worked example: the comment is the proof

A `SAFETY` comment answers one question: *why is this particular operation
sound here?* It names the facts that the code relies on. Compare:

```rust
/// A cell for a single-threaded program that hands out its value by copy.
struct Slot {
    value: core::cell::UnsafeCell<u32>,
}

impl Slot {
    fn get(&self) -> u32 {
        // SAFETY: `Slot` is not `Sync` (`UnsafeCell` is not), so no other
        // thread can access the value. This method creates no reference to
        // it, and no method of `Slot` hands one out, so nothing can be
        // reading or writing the value during this read.
        unsafe { *self.value.get() }
    }

    fn set(&self, new: u32) {
        // SAFETY: as in `get`: no other thread, and no reference exists.
        unsafe { *self.value.get() = new }
    }
}

let slot = Slot { value: core::cell::UnsafeCell::new(1) };
slot.set(slot.get() + 1);
assert_eq!(slot.get(), 2);
```

The comment in `get` would be *wrong* for a `Slot` that also had
`fn borrow(&self) -> &u32`. A reviewer can check that by reading the type's
methods, and that is the value of a comment that names its facts. "SAFETY:
this is fine" or "SAFETY: we know the pointer is valid" give a reviewer
nothing to check.

In a kernel the facts are often about the machine and about time: "IRQs are
masked", "this runs before any task exists", "only this driver uses this
address", "single core". Write them down exactly. When the kernel later gains
a second core or a new caller, these comments are the list of what to
re-check.

### Mistakes people really make

**An unsafe attribute without `unsafe`.**

```rust,compile_fail
#[no_mangle]
extern "C" fn kmain() {}
```

```text
error: unsafe attribute used without unsafe
 --> src/main.rs:1:3
  |
1 | #[no_mangle]
  |   ^^^^^^^^^ usage of unsafe attribute
  |
help: wrap the attribute in `unsafe(...)`
  |
1 | #[unsafe(no_mangle)]
  |   +++++++         +
```

It is unsafe because two symbols with one name, or a symbol that shadows one
from another library, give undefined behaviour at link time.

**An `extern` block without `unsafe`.** Writing
`extern "C" { static __bss_start: u8; }` in edition 2024 gives:

```text
error: extern blocks must be unsafe
 --> src/main.rs:3:1
  |
3 | extern "C" { static __bss_start: u8; }
  | -^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  | |
  | help: needs `unsafe` before the extern keyword: `unsafe`
```

The compiler cannot check that the symbol exists or has this type. For a
symbol from a linker script the type is a fiction anyway: only its *address*
has meaning. Take the address with `&raw const` and never read the "value".

**Inline assembly outside an unsafe block.**

```rust,compile_fail,E0133
core::arch::asm!("nop");
```

```text
error[E0133]: use of inline assembly is unsafe and requires unsafe block
 --> src/main.rs:2:1
  |
2 | core::arch::asm!("nop");
  | ^^^^^^^^^^^^^^^^^^^^^^^ use of inline assembly
  |
  = note: inline assembly is entirely unchecked and can cause undefined behavior
```

Wrap each instruction you need in one small safe function, with the argument
for its soundness next to it. The rest of the kernel then calls
`cpu::wait_for_interrupt()` and contains no assembly.

**Declaring too little in `asm!`.** The compiler believes the operand list.
An instruction that changes a register you did not declare as an output
corrupts whatever the compiler kept there. Declare every register the
instruction writes, and do not claim `nomem` or `nostack` unless it is true.

**One big `unsafe` block.** A block around twenty lines hides which of them
needs it. Keep one unsafe operation, or one tight group, per block, each with
its own comment.

### When to use which

| Tool | Use it for |
| --- | --- |
| Host tests | Logic. Fast, with a debugger. |
| Miri (`cargo +nightly-2026-11-20 miri test`) | Unsafe code that is not machine-specific: allocators, `MaybeUninit` containers. It finds out-of-bounds access, use of uninitialised memory, aliasing violations and leaks. It cannot run `asm!` or talk to devices. |
| QEMU tests | Everything that needs the machine: boot, registers, interrupts, switching. They show that it works once on one emulator, not that it is sound. |
| Review against the `SAFETY` comments | The rest. For entry code, vectors and the context switch there is no tool, only careful reading. |

### Read more

- [The Rust Reference: unsafety](https://doc.rust-lang.org/reference/unsafety.html) and [behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).
- [The Edition Guide: unsafe attributes](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html), [unsafe extern blocks](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html) and [`unsafe_op_in_unsafe_fn`](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html).
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/), in particular "Working with Unsafe".
- [Miri](https://github.com/rust-lang/miri), the interpreter that checks unsafe code.
