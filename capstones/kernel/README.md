# Capstone: a kernel for the QEMU `virt` machine

[All chapters](../../README.md) · [Specification](SPEC.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write a small operating system kernel in Rust that boots on an emulated
64-bit Arm computer. Nothing runs underneath it: no operating system, no
standard library, no runtime. Your code sets up its own stack, drives the
serial port, manages memory, handles interrupts, switches between tasks and
passes messages between them.

When you are done, your kernel boots in QEMU, prints this, and switches the
machine off:

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

This is a **blank-file project**. There are no lesson files and no function
bodies to fill in one by one. You get a precise specification, a small API
facade that the host tests call, and acceptance tests that boot your kernel
in QEMU and compare what it prints.

| File | What it is |
| --- | --- |
| [SPEC.md](SPEC.md) | The complete specification: the machine, the register facts, the transcript, the facade. Read all of it first. |
| [CONCEPTS.md](CONCEPTS.md) | The background that the course has not taught yet, with small examples that were compiled and run. |
| [HINTS.md](HINTS.md) | Design hints per milestone. No code. |
| [RUBRIC.md](RUBRIC.md) | How to judge your result, including what the tests cannot see. |
| `kcore/src/lib.rs` | The facade of the hardware-independent library, with `todo!()` bodies. |
| `kernel/src/main.rs` | A stub for the bare-metal binary. Everything else in `kernel/` is yours to create. |
| `tests/`, `harness/lib.rs` | The acceptance tests and the code that runs QEMU for them. |
| `tests/transcript.txt` | The required serial output, as the tests read it. |

The reference kernel, with a `DESIGN.md` that explains it, is in
`solutions/capstones/kernel/` of the course repository. Look at it after you
have finished, or when one problem has kept you stuck for a long time.

## Prerequisites

| From | You use it for |
| --- | --- |
| [Chapter 23, Unsafe Rust](../../exercises/chapter23/) | raw pointers, `unsafe` blocks and functions, `UnsafeCell`, `repr(C)`, writing `SAFETY` comments |
| [Chapter 24, Calling C](../../exercises/chapter24/) | `extern "C"`, symbols and linking, calling conventions |
| Mastery module [`m2_unsafe`](../../mastery/m2_unsafe/README.md) | `MaybeUninit`, building containers from raw memory, pointer provenance, Miri |
| Mastery module [`m3_concurrency`](../../mastery/m3_concurrency/README.md) | atomics, what a lock has to guarantee, lost wake-ups |
| Chapters [11](../../exercises/chapter11/README.md) and [13](../../exercises/chapter13/README.md) | traits and trait objects for the driver layer, `Drop` for guards |
| [Chapter 22](../../exercises/chapter22/README.md) | `macro_rules!` for `print!` and `println!` |

You do not need to know Arm assembly or operating system theory.
[CONCEPTS.md](CONCEPTS.md) explains the instructions and ideas you need.

Plan for 40 to 60 hours of work (an estimate for a first kernel).

## Install the tools

### 1. The Rust target

The file `rust-toolchain.toml` in this directory pins the toolchain and
names the bare-metal target, so `rustup` installs both the first time you run
`cargo` here. To do it by hand:

```text
rustup target add aarch64-unknown-none
```

### 2. QEMU

You need the program `qemu-system-aarch64` on your `PATH`.

| System | Command |
| --- | --- |
| Debian, Ubuntu | `sudo apt install qemu-system-arm` (this package contains `qemu-system-aarch64`) |
| Fedora | `sudo dnf install qemu-system-aarch64` |
| Arch Linux | `sudo pacman -S qemu-system-aarch64` |
| macOS with Homebrew | `brew install qemu` |
| Windows | Use the installer linked from the [QEMU download page](https://www.qemu.org/download/), then add the installation directory (usually `C:\Program Files\qemu`) to `PATH`. With MSYS2: `pacman -S mingw-w64-ucrt-x86_64-qemu`. |

If a package name has changed, the [QEMU download page](https://www.qemu.org/download/)
lists the current ones.

Check it:

```text
qemu-system-aarch64 --version
```

The capstone was developed and checked with QEMU 11.1.2 on macOS. The
machine options it uses are old and stable, but other versions were not
tested. The test harness uses only the Rust standard library and starts QEMU
directly, without a shell.

If QEMU is installed somewhere else, set `KERNEL_QEMU` to the full path of
the program.

### 3. Miri (for the unsafe code in `kcore`)

```text
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
```

### 4. A debugger (optional)

| System | Debugger |
| --- | --- |
| Debian, Ubuntu | `sudo apt install gdb-multiarch` |
| Fedora | `sudo dnf install gdb` |
| macOS | `lldb` is part of the Xcode command line tools; or `brew install aarch64-elf-gdb` |
| Windows | With MSYS2: `pacman -S mingw-w64-ucrt-x86_64-gdb-multiarch` |

## How to work

1. Read [SPEC.md](SPEC.md) completely. Read the topics of
   [CONCEPTS.md](CONCEPTS.md) for the milestone you are on.
2. Run the tests once, to see the starting point:

   ```text
   cargo test
   ```

   Every test fails: the host tests at `todo!()`, the QEMU tests with "the
   kernel printed nothing". If the QEMU tests say `SKIPPED` instead, a tool
   is missing; see above.
3. Work on one milestone at a time, in order. For M1 and M2 you work only in
   `kernel/`. From M3 on, each milestone starts in `kcore/` with the host
   tests and ends in `kernel/`.

   | Milestone | Host tests to turn green first | QEMU test |
   | --- | --- | --- |
   | M1 boot | `host_reg` (the baud rate divisor) | `m1_boot`, `powers_off_cleanly`, `panic_selftest` |
   | M2 drivers | `host_reg` (bit fields) | `m2_drivers` |
   | M3 memory | `host_heap` | `m3_heap` |
   | M4 interrupts | `host_reg` (the rest) | `m4_interrupts` |
   | M5 tasks | `host_ring`, `host_sched` | `m5_tasks` |
   | M6 message passing | `host_ipc` | `m6_ipc`, `full_transcript` |

4. Before you call a milestone done, run Miri and Clippy and read
   [RUBRIC.md](RUBRIC.md) for that milestone.

### Commands

Run these in this directory:

```text
cargo test --test host_heap             # one group of host tests
cargo test --test qemu m3               # one milestone in QEMU
cargo test                              # everything, debug kernel
cargo test --release                    # everything, optimised kernel
cargo +nightly-2026-11-20 miri test                # host tests under Miri (QEMU tests are left out)
cargo clippy --all-targets              # lints for kcore and the tests
```

Run these in `kernel/`:

```text
cargo build --target aarch64-unknown-none
cargo clippy --target aarch64-unknown-none
```

`cargo build` in `kernel/` without `--target` tries to build for your own
computer and fails at link time, because a `no_main` program has no `main`
for your operating system to call. A file `kernel/.cargo/config.toml` can
make the bare-metal target the default and can teach `cargo run` to start
QEMU; the [Cargo book on configuration](https://doc.rust-lang.org/cargo/reference/config.html)
describes the keys `build.target` and `target.<triple>.runner`.

The starter builds with one warning,
`rust-lld: cannot find entry symbol _start`. That is your first task.

## Run the kernel by hand

```text
qemu-system-aarch64 -machine virt,gic-version=2 -accel tcg -cpu cortex-a72 -smp 1 -m 128M -display none -monitor none -serial stdio -no-reboot -kernel kernel/target/aarch64-unknown-none/debug/kernel
```

Serial output appears in your terminal. If the kernel does not switch the
machine off, press Ctrl-C to end QEMU.

When a QEMU test fails, its message ends with the exact command that the
harness ran, including the path of the image it built.

## Debugging

A kernel that fails usually prints nothing. These tools show what happened.

### See every exception: `-d int`

Add `-d int` to the QEMU command line. QEMU then logs each exception and
interrupt that the guest takes. This is what a `brk` instruction looks like:

```text
Taking exception 7 [Breakpoint] on CPU 0
...from EL1 to EL1
...with ESR 0x3c/0xf2000007
...with SPSR 0x400003c5
...with ELR 0x40101168
...to EL1 PC 0x40100a00 PSTATE 0x3c5
```

`ELR` is the address of the instruction that was interrupted. If the same
exception repeats without end, your kernel faults inside its own exception
handler, or it has no vector table yet. Add `-D qemu.log` to send the log to
a file.

### Find an address in your code

After `rustup component add llvm-tools`, the LLVM tools are inside the
toolchain directory:

```text
BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc --print host-tuple)/bin"
"$BIN/llvm-objdump" -h kernel/target/aarch64-unknown-none/debug/kernel     # sections and addresses
"$BIN/llvm-nm" -n kernel/target/aarch64-unknown-none/debug/kernel          # symbols, sorted by address
"$BIN/llvm-objdump" -d kernel/target/aarch64-unknown-none/debug/kernel     # disassembly
```

Use the first command to check your linker script: `.text` must start at the
address you chose, and the entry symbol must be where you expect it.

### Step through the kernel with a debugger

Start QEMU with two more options. `-s` opens a debug port on TCP 1234, and
`-S` stops the CPU before the first instruction:

```text
qemu-system-aarch64 -machine virt,gic-version=2 -accel tcg -cpu cortex-a72 -smp 1 -m 128M -display none -monitor none -serial stdio -no-reboot -s -S -kernel kernel/target/aarch64-unknown-none/debug/kernel
```

In a second terminal, with gdb (`gdb-multiarch` on Debian and Ubuntu):

```text
gdb kernel/target/aarch64-unknown-none/debug/kernel
(gdb) target remote localhost:1234
(gdb) break kmain
(gdb) continue
(gdb) backtrace
(gdb) info registers
(gdb) stepi
```

or with lldb:

```text
lldb kernel/target/aarch64-unknown-none/debug/kernel
(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name kmain
(lldb) continue
(lldb) bt
(lldb) register read pc sp x0
(lldb) frame variable
```

`kmain` stands for the name of your Rust entry function. Useful breakpoints
are your entry symbol, your panic handler and your exception handler. Debug
builds are much easier to step through than release builds.

### Print early, print often

Once the UART works, lines without an `[mN] ` tag are your debugging tool.
The milestone tests ignore them. Remove them before `full_transcript`.

## When something goes wrong

| Symptom | Likely cause |
| --- | --- |
| QEMU tests print `SKIPPED` | `qemu-system-aarch64` is not on `PATH`, or the target `aarch64-unknown-none` is not installed. |
| "the kernel printed nothing" | No entry point, the image is not linked into RAM, the linker script is not passed to the linker, or the stack pointer is not set. Check with `llvm-objdump -h` and `-d int`. |
| Works in a debug build, hangs in a release build | A device register is read without `read_volatile`; or code runs before floating-point access is enabled; or a stack is too small in one profile. |
| Hangs as soon as a struct is copied or `{}` formatting is used | Floating-point and SIMD registers are still trapped (SPEC 2.2). |
| The first timer interrupt arrives, then nothing | The handler did not re-arm the timer or did not complete the interrupt at the controller. |
| Interrupts never arrive | One of the four places that can block them is still closed (CONCEPTS topic 7). |
| Garbled or interleaved lines | Printing is not protected against interrupts and task switches (SPEC 3.2). |
| "QEMU was still running after 10 s" | The kernel did not power the machine off, or a task waits for an event that never comes. |
| A QEMU test fails only sometimes | A race: something depends on *when* an interrupt arrives. See the hints for M5. |
| Miri stops in `host_heap` with "attempting a write access using <...> ..., but that tag does not exist in the borrow stack for this location" | The allocator used a pointer that is not allowed to access that memory. Usual cause: a pointer made from an integer, or the pointer that was passed to `dealloc`, instead of one derived from the region pointer (SPEC 5.2). |

## Rules of the project

- Stable Rust only, as pinned in `rust-toolchain.toml`. No external crates.
- Do not change `tests/` or `harness/`.
- Keep the names and signatures in `kcore/src/lib.rs`. Everything behind them
  is yours.
- Keep notes while you work. [SPEC.md](SPEC.md) section 9 lists the evidence
  that the rubric asks for.
