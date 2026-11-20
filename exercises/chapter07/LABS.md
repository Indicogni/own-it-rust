# Chapter 7 optional labs

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

These labs are optional. They cover behaviour that a unit test cannot check
well, because it belongs to a whole process: how the program ends, what it
prints on which stream, and which exit status it leaves behind. Nothing in
the chapter's tests depends on them.

Each lab gives you a complete program. Paste it into
[`examples/ch07_scratch.rs`](../../examples/ch07_scratch.rs), run the
commands, and compare what you see with the notes. The outputs below were
recorded with the pinned toolchain (Rust 1.99.0) on macOS. Thread ids differ
on every run, and paths differ on your machine.

**Reading the exit status.** Run this directly after the program:

| Shell | Command |
| --- | --- |
| sh, bash, zsh | `echo $?` |
| PowerShell | `echo $LASTEXITCODE` |
| Windows `cmd` | `echo %ERRORLEVEL%` |

When you finish a lab, put a program that compiles back into the scratch
file, so that the whole repository still builds.

## Lab 1 · Unwinding versus abort

*Goes with lesson 01.* **Predict first:** which of the three `eprintln!`
lines will run, and in what order?

```rust,should_panic
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        eprintln!("drop {}", self.0);
    }
}

fn inner() {
    let _inner = Noisy("inner guard");
    panic!("invariant broken in inner()");
}

fn main() {
    let _outer = Noisy("outer guard");
    inner();
    eprintln!("this line is never reached");
}
```

1. Run `cargo run --example ch07_scratch`, then read the exit status.

   ```text
   thread 'main' (122242982) panicked at examples/ch07_scratch.rs:11:5:
   invariant broken in inner()
   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
   drop inner guard
   drop outer guard
   ```

   The status was 101. Both guards were dropped, innermost first, after the
   panic message was printed.

2. Build the same program so that a panic aborts. Cargo reads profile
   settings from environment variables, so you do not need to edit
   `Cargo.toml`:

   ```sh
   CARGO_PROFILE_DEV_PANIC=abort cargo run --example ch07_scratch
   ```

   In PowerShell, set the variable first with
   `$env:CARGO_PROFILE_DEV_PANIC = "abort"`, and remove it afterwards with
   `Remove-Item Env:CARGO_PROFILE_DEV_PANIC`.

   The panic message was printed as before, but neither `drop` line
   appeared. The shell reported status 134, which is how a shell on macOS or
   Linux shows "killed by the abort signal". The number is different on
   Windows.

3. Run it once more with `RUST_BACKTRACE=1` in front and find `inner` and
   `main` in the backtrace.

**Explain in NOTES.md:** why may a library never assume that its
destructors run? Which kind of program would choose `panic = "abort"`, and
what does it give up?

## Lab 2 · The panic hook and the other panicking macros

*Goes with lesson 01.* The panic hook is the function that prints the
message. A program can replace it, for example to write panics to a log
file. The hook is global to the process, which is why the chapter tests
never touch it.

```rust
use std::panic;

/// Runs `f`, catches its panic, and says whether it panicked.
fn observe(name: &str, f: impl FnOnce() + panic::UnwindSafe) {
    let outcome = panic::catch_unwind(f);
    println!("{name}: panicked = {}", outcome.is_err());
}

fn main() {
    // At least 1 (the program name), so every check below fails.
    let n = std::env::args().count();

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let place = info.location().map(|l| format!("line {}", l.line()));
        let message = info.payload_as_str().unwrap_or("<not a string>");
        eprintln!("[hook] {message:?} at {place:?}");
    }));

    observe("panic!", || panic!("plain message"));
    observe("panic! with a value", move || panic!("n is {n}"));
    observe("assert!", move || assert!(n == 0, "n must be zero"));
    observe("assert_eq!", move || assert_eq!(n, 0));
    observe("unreachable!", || unreachable!("no such state"));
    observe("todo!", || todo!());
    observe("unimplemented!", || unimplemented!("later"));
    observe("debug_assert!", move || debug_assert!(n == 0, "debug builds only"));

    // Put the default hook back, as code that replaced it should.
    panic::set_hook(default_hook);
    observe("default hook", || panic!("printed by the default hook"));
}
```

1. Run `cargo run --example ch07_scratch`. Recorded output:

   ```text
   [hook] "plain message" at Some("line 20")
   panic!: panicked = true
   [hook] "n is 1" at Some("line 21")
   panic! with a value: panicked = true
   [hook] "n must be zero" at Some("line 22")
   assert!: panicked = true
   [hook] "assertion `left == right` failed\n  left: 1\n right: 0" at Some("line 23")
   assert_eq!: panicked = true
   [hook] "internal error: entered unreachable code: no such state" at Some("line 24")
   unreachable!: panicked = true
   [hook] "not yet implemented" at Some("line 25")
   todo!: panicked = true
   [hook] "not implemented: later" at Some("line 26")
   unimplemented!: panicked = true
   [hook] "debug builds only" at Some("line 27")
   debug_assert!: panicked = true

   thread 'main' (122273818) panicked at examples/ch07_scratch.rs:31:32:
   printed by the default hook
   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
   default hook: panicked = true
   ```

   The program exits with status 0: every panic was caught.

2. Run it again with `--release`. One line changes:

   ```text
   debug_assert!: panicked = false
   ```

   `debug_assert!` is compiled out of release builds; `assert!` is not.

**Explain in NOTES.md:** each macro states a different intention. Which one
would you use for "this `match` arm cannot happen", for "I have not written
this yet", and for "the caller broke the rules"?

## Lab 3 · Display, Debug, and the two output streams

*Goes with lesson 02.*

```rust
fn main() {
    let error = "12x".parse::<u8>().unwrap_err();
    println!("stdout, Display: {error}");
    println!("stdout, Debug:   {error:?}");
    eprintln!("stderr, Display: {error}");
}
```

1. Run `cargo run -q --example ch07_scratch`. All three lines appear.
2. Throw standard error away: add `2>/dev/null` to the command (in
   PowerShell, `2>$null`). Recorded output:

   ```text
   stdout, Display: invalid digit found in string
   stdout, Debug:   ParseIntError { kind: InvalidDigit }
   ```

3. Throw standard output away instead: `>/dev/null` (in PowerShell,
   `>$null`). Recorded output:

   ```text
   stderr, Display: invalid digit found in string
   ```

**Explain in NOTES.md:** a user runs `your_tool > result.txt`. Where should
a warning go so that it does not end up inside `result.txt`?

## Lab 4 · The anyhow crate, in a separate package

*Goes with lessons 05 and 09.* This lab needs network access once, to
download the crate. Do it **outside** the course repository; do not add
dependencies to the course package.

`anyhow::Error` is a boxed error with extras: any error that is
`Send + Sync + 'static` converts into it with `?`, and `with_context` adds a
message in front while keeping the original as the cause. This lab is a
first look. Lesson E01 of the course's
[`ecosystem` package](../../ecosystem/README.md) practises `anyhow` and
`thiserror` with tests.

```sh
cargo new ch07_anyhow
cd ch07_anyhow
cargo add anyhow
```

Put this in `src/main.rs` (it needs the `anyhow` crate, so it is not
compiled with this file):

```rust,ignore
use anyhow::{Context, Result};

fn read_limit(path: &str) -> Result<u32> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("could not read {path}"))?;
    let limit = text
        .trim()
        .parse()
        .with_context(|| format!("{path} does not hold a number"))?;
    Ok(limit)
}

fn main() -> Result<()> {
    let limit = read_limit("limit.txt")?;
    println!("limit = {limit}");
    Ok(())
}
```

1. `cargo run` with no `limit.txt`. Recorded with anyhow 1.0.104; the status
   was 1:

   ```text
   Error: could not read limit.txt

   Caused by:
       No such file or directory (os error 2)
   ```

2. Create `limit.txt` containing the word `ten` and run again:

   ```text
   Error: limit.txt does not hold a number

   Caused by:
       invalid digit found in string
   ```

3. In `main`, handle the error yourself: print it with `{error}`, with
   `{error:#}`, and loop over `error.chain()`. Then try
   `error.downcast_ref::<std::num::ParseIntError>()`.

**Explain in NOTES.md:** compare this with the hand-written `chain` function
in the Lesson 06 section of CONCEPTS.md. What did the crate do for you? What
would a caller of `read_limit` lose compared with a typed error enum?

## Lab 5 · A real `main`: messages and exit status

*Goes with lesson 09 and the capstone.*

```rust,no_run
use std::error::Error;

fn total(args: &[String]) -> Result<u32, Box<dyn Error>> {
    if args.is_empty() {
        return Err("give me at least one number".into());
    }
    let mut total = 0;
    for arg in args {
        total += u32::from(arg.parse::<u16>()?);
    }
    Ok(total)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", total(&args)?);
    Ok(())
}
```

1. Run it three ways and read the status after each run. Arguments for your
   program go after `--`:

   ```sh
   cargo run -q --example ch07_scratch -- 2 3
   cargo run -q --example ch07_scratch -- 2 x
   cargo run -q --example ch07_scratch
   ```

   Recorded output, with statuses 0, 1 and 1:

   ```text
   5
   Error: ParseIntError { kind: InvalidDigit }
   Error: "give me at least one number"
   ```

   `main` printed the `Debug` text of each error. Notice the quotation
   marks in the last line.

2. Change `main` to return `ExitCode`. Add
   `use std::process::ExitCode;`, and a guard so that you can see
   destructors run. Keep `total` as it is and replace `main` with this:

   ```rust,ignore
   struct Noisy(&'static str);

   impl Drop for Noisy {
       fn drop(&mut self) {
           eprintln!("drop {}", self.0);
       }
   }

   fn main() -> ExitCode {
       let _guard = Noisy("guard in main");
       let args: Vec<String> = std::env::args().skip(1).collect();
       match total(&args) {
           Ok(sum) => {
               println!("{sum}");
               ExitCode::SUCCESS
           }
           Err(error) => {
               eprintln!("total: {error}");
               ExitCode::from(2)
           }
       }
   }
   ```

   The same three runs, now with statuses 0, 2 and 2:

   ```text
   5
   drop guard in main
   total: invalid digit found in string
   drop guard in main
   total: give me at least one number
   drop guard in main
   ```

3. Replace `ExitCode::from(2)` with `std::process::exit(2)` and run the
   failing case again. The status is still 2, but `drop guard in main` is
   gone: `exit` ends the process without running destructors.

   ```text
   total: invalid digit found in string
   ```

4. **Transfer.** Copy `ReportError`, `parse_row` and `report` from your
   finished capstone into the scratch file. Write a `main` that passes the
   command-line arguments to `report`, prints one stable line per error
   variant on standard error (include the row number), and returns a
   different status for bad input than for overflow.

**Explain in NOTES.md:** when is `fn main() -> Result<(), Box<dyn Error>>`
good enough, and what does a user of your tool gain from the `ExitCode`
version?
