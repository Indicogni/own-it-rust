# Chapter 1 concepts — Start here

[Chapter guide](README.md) · [Hints](HINTS.md) · [Your notes](NOTES.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every Rust code block in this file was compiled
and run with the course toolchain (Rust 1.99.0, edition 2024). Every piece of
compiler, Cargo or test output was copied from that toolchain and then
shortened: a line with `...` marks a cut. In test output, the number in
parentheses after a test's name is a thread id; yours will be different.
Directory names in the output were replaced by `/home/you/...`; you will see
your own.

## What you will be able to do

After this chapter you can:

- install Rust, check that the course uses the right version, and set up an
  editor that shows errors while you type;
- create, check, build, run and test a small project with Cargo, and say what
  `target/` and `Cargo.lock` are;
- read the report of a failed test and find the test, the line, and the
  expected and the actual value in it;
- read a compiler error: its code, its position, its labels and its help;
- work through a lesson of this course with the loop *predict, implement,
  explain*, and use the course tools `cargo progress` and `cargo drill`.

This chapter teaches very little of the Rust language. That is the job of
Chapter 2. Here you learn the workshop: the tools, the reports they print, and
the way every later lesson works.

## Before you start

You have written a little code in some programming
language: you know what a variable, a function and an `if` are. You do not
need to know any Rust. You do not need to have used a terminal before; the
first lesson tells you what to type.

## Time

These are estimates for a first pass. They include reading the
section, the predictions, the tasks and your notes. Installing Rust can take
longer on a slow connection.

| Lesson | Estimate |
| --- | --- |
| 01 · Install, verify, and a first green test | 45 minutes |
| 02 · Cargo: new, run, check, build, test | 35 minutes |
| 03 · Reading a failing test | 35 minutes |
| 04 · Reading a compiler error | 30 minutes |
| Compiler drills | 40 minutes |
| 05 · The lesson loop | 40 minutes |
| 06 · Capstone | 30 minutes |

That is about four and a half hours in total.

## Words used in this chapter

- **Terminal**: a window in which you type commands and read the text that
  programs print. On Windows it is called *PowerShell* or *Terminal*; on macOS
  it is the *Terminal* app; on Linux it has names such as *Terminal* or
  *Konsole*.
- **Directory**: a folder. The **course directory** is the folder that holds
  this course, the one with the file `Cargo.toml` at its top.
- **Compiler**: the program that translates Rust source text into a program
  that the computer can run. Rust's compiler is called `rustc`.
- **Toolchain**: one version of the compiler together with its helper
  programs.
- **Panic**: Rust's word for "the program stops here because something is
  wrong". A panic prints a message and the place in the source text where it
  happened.

## Lesson 01 · Install, verify, and a first green test

### The idea

You need three things on your computer: the Rust toolchain, a copy of the
course, and an editor. After that, one command runs the tests of one lesson.
A test that fails is printed as `FAILED`, and a test that passes is printed
as `ok`. Programmers say "red" and "green" for these two results. This lesson
takes one test from red to green, so that you know the whole path works
before the content gets harder.

### Install Rust with rustup

Rust is installed by a small program called **rustup**. It downloads
toolchains and keeps them up to date. Follow the official instructions at
<https://www.rust-lang.org/tools/install>. In short:

- **Linux and macOS**: the page shows one command to paste into a terminal.
  Rust also needs a *linker* from your system. On macOS, run
  `xcode-select --install` once. On Linux, install your distribution's C
  compiler package (on Debian and Ubuntu it is called `build-essential`).
- **Windows**: download and run `rustup-init.exe` from that page. It checks
  for the Microsoft C++ build tools and offers to install them if they are
  missing. Accept that offer.

Accept the default choices of the installer. When it has finished, **close the
terminal and open a new one**. The new terminal knows where the Rust programs
are.

You now have three commands:

| Command | What it is |
| --- | --- |
| `rustup` | Installs and updates toolchains. You rarely call it yourself. |
| `rustc` | The compiler. Cargo calls it for you. |
| `cargo` | Rust's build tool. You use it all the time: it compiles, runs and tests. |

### Get the course and check the version

The course lives in a Git repository at
<https://github.com/Indicogni/own-it-rust>. **Git** is a program that
copies a repository to your computer and records changes to files. There are
two ways to get the course.

**With Git (recommended).** Check whether Git is installed: type
`git --version` in a terminal. If the command is not found, install Git from
<https://git-scm.com/downloads> (on macOS, `xcode-select --install` from the
step above also installs it). Then go to the folder where you want the course,
for example your *Documents* folder, and copy the repository:

```sh
git clone https://github.com/Indicogni/own-it-rust.git
cd own-it-rust
```

The first command creates the folder `own-it-rust`. The second goes into
it: `cd` means "change directory". The [Setup section of the course
README](../../README.md#fresh-clone-setup) shows the same commands.

**Without Git.** On the repository page, click the green *Code* button, then
*Download ZIP*. Unpack the ZIP file. The folder inside is named after the
repository and the branch, for example `own-it-rust-main`; you may
rename it. A copy made this way works for the whole course. It cannot fetch
corrections to the course later; with Git, `git pull` does that.

Whichever way you chose, open a terminal and go into the course directory with
`cd`, for example:

```sh
cd Documents/own-it-rust
```

Use the path where *you* put it. Every command in this course is typed in the
course directory unless the text says something else.

The course works with one exact version of Rust, so that every message you see
is the same as in this text. The file `rust-toolchain.toml` in the course
directory names that version:

```toml
[toolchain]
channel = "1.99.0"
profile = "minimal"
# rust-src lets rust-analyzer show and analyse the standard library.
components = ["rustfmt", "clippy", "rust-src"]
```

You do not have to install this version yourself. The first time you run
`cargo` or `rustc` inside the course directory, rustup reads this file and
downloads the right toolchain. This needs an internet connection once. Check
it now:

```sh
rustc --version
cargo --version
```

```text
rustc 1.99.0 (b940084d7 2026-09-28)
cargo 1.99.0 (5f94df478 2026-08-27)
```

The three numbers after `rustc` are the version. If you run the same command
in another directory, you may see a newer version. That is correct: the file
`rust-toolchain.toml` only applies inside the course directory.

### An editor that understands Rust

You can write Rust in any text editor. An editor with **rust-analyzer** is
much more comfortable. rust-analyzer is the official helper program for
editors. It shows compiler errors while you type, shows the type of a value
when you point at it, and jumps to the place where a function is defined.

A free and common choice is Visual Studio Code with the extension named
"rust-analyzer". Open the **whole course directory** in the editor (in VS
Code: *File*, *Open Folder*), not a single file. rust-analyzer needs the file
`Cargo.toml` at the top to understand the project. The first start takes a
minute or two.

**Editor setup for the separate packages.** Most of the course (the chapters
and the drills) is one Cargo package, the one whose `Cargo.toml` is at the top
of the course directory. Some parts are separate packages with their own
`Cargo.toml`: `project/`, `ecosystem/`, every module under `mastery/`, every
capstone under `capstones/` and every checkpoint under `checkpoints/`. When you
open the course directory, rust-analyzer loads the top package, and then shows
no errors and no types in the files of the separate ones. (In VS Code, the
course's settings file `.vscode/settings.json` also loads `project/`.) When
you reach one of the others, do one of two things:

- open that package's directory as its own editor window (in VS Code: *File*,
  *Open Folder*, then choose for example `ecosystem`), or
- add the package's `Cargo.toml` to the rust-analyzer setting
  `rust-analyzer.linkedProjects`.

The [editor setup notes in the course README](../../README.md#editor-setup)
show both ways.

### Just enough Rust for this chapter

The tasks of this chapter are very small. You need to read one shape:

```rust
fn double(n: i32) -> i32 {
    n * 2
}

fn main() {
    assert_eq!(double(21), 42);
}
```

- `fn double` starts a function with the name `double`.
- `(n: i32)` is the list of parameters. Each has a name, a colon and a type.
  `i32` is a whole number that may be negative. `u32` is a whole number that
  is never negative. `bool` is `true` or `false`.
- `-> i32` is the type of the value that the function returns.
- The body stands between `{` and `}`. The value of the **last line, written
  without a semicolon**, is what the function returns. There is no `return`
  word here.

The block above is a complete program. `fn main()` is the function in which
every program starts. The line `assert_eq!(double(21), 42);` checks that the
two values are equal, and stops the program with a message when they are
not; lesson 03 is about that message. Most code blocks in this course are
written this way: a `main` that checks what the text claims, so that every
block can be compiled and run.

Text is written between double quotes: `"ready"`. In this chapter the type of
such a text is written `&'static str`. Read it as "a text that is written in
the program". Chapter 2 explains it.

Two values can travel together as a **pair**. A pair is written in
parentheses with a comma, `(3, 34)`, and its type is written the same way:
`(u32, u32)`. Some tasks and predictions of this chapter ask for a pair. You
will also meet the type `usize` in a few places. It is one more whole-number
type that is never negative; Rust uses it for counts and positions.

In a lesson file, a task looks like this before you start:

```rust,should_panic
fn motto() -> &'static str {
    todo!("return the text \"measure twice\"")
}

fn main() {
    println!("{}", motto());
}
```

`println!` prints one line of text. The `{}` inside the quotes is a place for
a value: the value after the comma is put there.

`todo!(...)` is a placeholder. It lets the file compile although the function
is not written yet. When the program reaches it, the program panics with the
message "not yet implemented". To solve the task, replace the whole
`todo!(...)` with the value:

```rust
fn motto() -> &'static str {
    "measure twice"
}

fn main() {
    assert_eq!(motto(), "measure twice");
}
```

### Run one lesson

```sh
cargo test --test chapter01 l01_
```

Read the command as: "Cargo, run tests. Use the group of tests called
`chapter01`. Of those, run only the tests with `l01_` in their name." The
first run compiles the course and takes a little while. Cargo first prints a
few lines of its own, which start with `Compiling`, `Finished` and `Running`;
lesson 02 explains them. Then it prints a report. For lesson 01, before you
change anything, the report looks like this:

```text
running 4 tests
test l01_first_test::tests::b_minor_version_matches_the_course_toolchain ... FAILED
test l01_first_test::tests::a_status_is_ready ... FAILED
test l01_first_test::tests::c_add_returns_the_sum ... FAILED
test l01_first_test::tests::p_predictions ... FAILED
...
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 27 filtered out; ...

error: test failed, to rerun pass `--test chapter01`
```

Four tests, all red. That is the normal state of a lesson that nobody has done
yet. It does not mean that your installation is broken. The last line starts
with the word `error`, and it means only this: at least one test failed.
Cargo prints it after every run that is not completely green. After you solve
task A and run the same command again, the line for `a_status_is_ready` ends
in `ok`, and the result line says `1 passed; 3 failed`. Lesson 03 teaches you
to read the middle part of the report.

One test in each lesson is called `p_predictions`. It contains lines such as
`let p1: i32 = guess();`. Replace `guess()` with the value that you expect,
**before** you run the test. Lesson 05 explains why this matters. Answer all
predictions of a lesson before the first run. The test stops at the first
wrong answer, so the report shows one wrong prediction at a time: correct it,
run again, and the next one appears.

### Mistakes people really make

**The terminal is in the wrong directory.** Cargo looks for `Cargo.toml` in
the directory where you are and in the directories above it:

```text
error: could not find `Cargo.toml` in `/home/you/empty` or any parent directory
```

Fix: `cd` into the course directory.

**A semicolon after the value.** A semicolon turns the last line into a
statement that returns nothing. This does not compile:

```rust,compile_fail,E0308
fn motto() -> &'static str {
    "measure twice";
}

fn main() {
    println!("{}", motto());
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:1:15
  |
1 | fn motto() -> &'static str {
  |    -----      ^^^^^^^^^^^^ expected `&str`, found `()`
  |    |
  |    implicitly returns `()` as its body has no tail or `return` expression
2 |     "measure twice";
  |                    - help: remove this semicolon to return this value
```

The last line of the message tells you the fix. Lesson 04 shows how to read
all of it.

**Text without quotes.** Without quotes, the compiler reads a word as the name
of a variable and cannot find one:

```rust,compile_fail,E0425
fn motto() -> &'static str {
    measure
}

fn main() {
    println!("{}", motto());
}
```

```text
error[E0425]: cannot find value `measure` in this scope
 --> src/main.rs:2:5
  |
2 |     measure
  |     ^^^^^^^ not found in this scope
```

**"command not found: cargo"** directly after installing. Close the terminal
and open a new one.

### Read more

- [Install Rust](https://www.rust-lang.org/tools/install), the official
  installation page.
- [The rustup book: overrides](https://rust-lang.github.io/rustup/overrides.html)
  explains `rust-toolchain.toml`.
- [`todo!` in the standard library](https://doc.rust-lang.org/std/macro.todo.html).

## Lesson 02 · Cargo: new, run, check, build, test

### The idea

**Cargo** is the tool that you talk to. It reads a description of your
project, calls the compiler with the right options, and puts the results in
one place. A project that Cargo can build is called a **package**. A package
is a directory with a file named `Cargo.toml` at the top and the source files
in a directory named `src`.

This course is one package. In this lesson you make a second, very small
package of your own, so that you see every file and every command once
without any course material around it.

### Lab: a project of your own

Do this lab in a directory **outside** the course directory, for example in
your home directory. Type each command, read what it prints, and look at the
directory afterwards. The tasks of the lesson ask what you saw.

Outside the course directory, the file `rust-toolchain.toml` does not apply,
so this lab runs with the newest Rust version that rustup installed. Its
output can differ in small details from the output printed here, such as the
times and the letters and digits after `hello-`. The commands and the files
are the same.

**Step 1: create the package.**

```sh
cargo new hello
cd hello
```

```text
    Creating binary (application) `hello` package
note: see more `Cargo.toml` keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
```

List the files (`ls -a` on Linux and macOS, `dir` on Windows). Cargo made two
things that matter now:

`Cargo.toml` is the **manifest**. It describes the package:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[dependencies]
```

`name` and `version` identify the package. `edition` says which edition of
the Rust language the source uses; this course uses 2024 everywhere. Under
`[dependencies]` you would list packages written by other people that your
program uses. This one uses none.

`src/main.rs` is the program. Cargo wrote a small one for you:

```rust
fn main() {
    println!("Hello, world!");
}
```

Every program starts in the function called `main`. `println!` prints one
line of text. Cargo normally also sets up version control (a `.git`
directory and a `.gitignore` file); you can ignore both for now.

**Step 2: check.**

```sh
cargo check
```

```text
    Checking hello v0.1.0 (/home/you/hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
```

`cargo check` asks the compiler one question: is this program correct Rust?
It reports every error, but it does not produce a program that can run. That
makes it the fastest command, and the one to use while you write.

List the directory again. Two new things are there: a directory and a file.
Look at their names before you read on.

**Step 3: build.**

```sh
cargo build
```

```text
   Compiling hello v0.1.0 (/home/you/hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
```

`cargo build` compiles the program completely. The result is an *executable*,
a file that the computer can run. It is at `target/debug/hello` (on Windows:
`target\debug\hello.exe`).

**Step 4: run.**

```sh
cargo run
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `target/debug/hello`
```

After these two lines comes one more line: the output of your program.
`cargo run` builds the program if something changed and then starts it. The
lines that start with a green word (`Finished`, `Running`) come from Cargo.
Everything after `Running` comes from the program.

**Step 5: test.**

```sh
cargo test
```

```text
   Compiling hello v0.1.0 (/home/you/hello)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running unittests src/main.rs (target/debug/deps/hello-261e14bad72d5a37)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test` builds a special version of the program that runs every function
marked as a test, and prints a report. This package has no tests yet, so the
report says `running 0 tests`.

### What the commands and files are

| Command | What it does | Use it when |
| --- | --- | --- |
| `cargo new NAME` | Creates a new package in a new directory. | You start a project. |
| `cargo check` | Looks for errors. Produces nothing that can run. | You are writing and want fast feedback. |
| `cargo build` | Compiles to an executable in `target/debug/`. | You need the executable itself. |
| `cargo run` | Builds if needed, then runs the program. | You want to see the program work. |
| `cargo test` | Builds and runs the tests, prints a report. | You want to know whether the code is right. |

Two things appear by themselves:

- **`target/`** is the directory where Cargo puts everything it builds. It can
  become large. You may delete it at any time; the next command builds
  everything again, which only costs time. It is never shared with other
  people, and `.gitignore` keeps it out of version control.
- **`Cargo.lock`** is a file that Cargo writes next to `Cargo.toml`. It
  records the exact version of every package that was used in the build.
  Cargo maintains it; you do not edit it. Keep it: with this file, the same
  source gives the same build on another computer and on another day.

The words in the output have fixed meanings. `Compiling` means the compiler
ran. `Finished` means the build is complete. `dev` is the name of a
**profile**, a set of compiler options. The `dev` profile compiles fast and
does not optimise ("unoptimized"), and it keeps information for debugging
("debuginfo"). `cargo build --release` uses the `release` profile: slower to
compile, faster to run, results in `target/release/`.

### A worked example: the edit and run cycle

Change the program so that it prints two lines. Open `src/main.rs` in your
lab package and write:

```rust
fn main() {
    let name = "Cargo";
    println!("Hello, {name}!");
    println!("This package was created with `cargo new`.");
}
```

`let name = "Cargo";` makes a variable. Inside the quotes of `println!`,
`{name}` is replaced by the value of the variable. Run it:

```text
   Compiling hello v0.1.0 (/home/you/hello)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
     Running `target/debug/hello`
Hello, Cargo!
This package was created with `cargo new`.
```

Cargo noticed that the source changed, so the first line says `Compiling`
again. Run `cargo run` once more without changing anything: the `Compiling`
line is gone, because nothing needs to be built.

### How the course uses Cargo

The course package has no single `main` program that matters. Its code lives
in tests, one group of tests for each chapter, and you choose the group with
`--test`:

```sh
cargo test --test chapter01 l02_
cargo run --example ch01_scratch
cargo progress
cargo drill ch01
```

The second command runs a small program from the `examples` directory. Each
chapter has one, as a place for your own experiments. `cargo progress` and
`cargo drill` are not built into Cargo. The course defines them in the file
`.cargo/config.toml` as short names for longer commands. Lessons 04 and 05
introduce them.

### Mistakes people really make

**Running Cargo outside a package.** You saw this message in lesson 01:
"could not find `Cargo.toml`". Cargo works on the package in which your
terminal stands. After `cargo new hello` you must `cd hello`.

**Expecting `cargo check` to run the program.** It never does. If you want to
see output, use `cargo run`.

**Creating the lab package inside the course directory.** Then the course
package and your package are nested, and commands may act on the wrong one.
Keep experiments of this kind outside.

**Editing `Cargo.lock` by hand, or deleting it to "clean up".** Leave it
alone. If you want a clean state, delete `target/`, or run `cargo clean`,
which does the same.

### Read more

- [The Cargo book: Creating a new package](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html)
- [The Cargo book: Cargo.toml vs Cargo.lock](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html)
- [The Cargo book: Build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html)
  describes what is inside `target/`.
- [The Cargo book: Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)

## Lesson 03 · Reading a failing test

### The idea

A **test** is a function that calls your code and checks the result. Cargo
runs every test and prints a report. A test passes when it runs to its end.
A test fails when it panics.

The checks are written with two macros (a macro is called like a function,
with a `!` after its name):

- `assert_eq!(a, b)` panics when `a` and `b` are not equal, and prints both.
- `assert!(condition, "message")` panics when the condition is false, and
  prints the message.

A failing test is not a problem to hide. It is the most precise description
of a mistake that you will get. This lesson trains you to take every piece of
information out of the report before you change any code.

### A worked example

Here are two functions with mistakes, and three tests.

```rust
/// Return the number of hours in `days` days.
fn hours_in(days: u32) -> u32 {
    days * 12
}

/// Return the short name of a working day: 0 is "Mon", 4 is "Fri".
fn day_name(day: usize) -> &'static str {
    let names = ["Mon", "Tue", "Wed", "Thu"];
    names[day]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_has_24_hours() {
        assert_eq!(hours_in(0), 0);
        assert_eq!(hours_in(2), 48);
    }

    #[test]
    fn friday_is_the_last_working_day() {
        assert_eq!(day_name(0), "Mon");
        assert_eq!(day_name(4), "Fri");
    }

    #[test]
    fn an_hour_count_is_never_odd() {
        assert!(hours_in(3) % 2 == 0, "3 days must give an even number of hours");
    }
}
```

The lines `#[cfg(test)]`, `mod tests` and `use super::*;` put the tests in a
separate section that can see the functions above it. `#[test]` marks a
function as a test. You will write such lines yourself in lesson 05.

A few small things in the example may be new. `["Mon", "Tue", "Wed", "Thu"]`
is a list of four texts, and `names[day]` reads the entry at the position
`day`. Positions are called indexes, and they start at 0. `%` gives the
remainder of a division; lesson 05 says more about it. `==` asks whether two
values are equal, and `!=` whether they differ. A `!` in front of a `bool`
means "not": `!true` is `false`.

The file is not in the repository. It was run inside the course under the
name `sample.rs`, and this is its report:

```text
running 3 tests
test sample::tests::an_hour_count_is_never_odd ... ok
test sample::tests::a_day_has_24_hours ... FAILED
test sample::tests::friday_is_the_last_working_day ... FAILED

failures:

---- sample::tests::a_day_has_24_hours stdout ----

thread 'sample::tests::a_day_has_24_hours' (123196818) panicked at exercises/chapter01/sample.rs:19:9:
assertion `left == right` failed
  left: 24
 right: 48

---- sample::tests::friday_is_the_last_working_day stdout ----

thread 'sample::tests::friday_is_the_last_working_day' (123196820) panicked at exercises/chapter01/sample.rs:9:5:
index out of bounds: the len is 4 but the index is 4
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    sample::tests::a_day_has_24_hours
    sample::tests::friday_is_the_last_working_day

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 31 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test chapter01`
```

### The parts of the report

**1. The list.** One line for each test: its full name and `ok` or `FAILED`.
The full name is *file*`::tests::`*function*. Tests run at the same time, so
the order of the lines changes from run to run.

**2. One block for each failed test**, after the word `failures:`. The block
starts with the name of the test between dashes. Then comes the important
line:

```text
... panicked at exercises/chapter01/sample.rs:19:9:
```

It gives the file, the **line** (19) and the column (9) where the panic
happened. Open the file at that line first.

**3. The message.** For a failed `assert_eq!` it is:

```text
assertion `left == right` failed
  left: 24
 right: 48
```

`left` is the first value inside `assert_eq!(...)` and `right` is the second.
The tests of this course always put the call of your function first and the
expected value second. So read it as: **left is what your code produced,
right is what it should produce.** Here, `hours_in(2)` produced 24 and should
produce 48. That is half of the right answer, and the body says `days * 12`.
The fix is `days * 24`.

Some assertions in the course carry their own text, which says what left and
right mean. In a prediction test the text is "left: your prediction, right:
actual".

**4. The second failure is of another kind.** Its location is line 9, and
line 9 is not inside a test. It is inside `day_name`. The test did not get as
far as comparing anything: the function itself panicked. The message says
why. The list `names` has 4 entries, with the indexes 0, 1, 2 and 3, and the
test asked for index 4. The fix is to add `"Fri"` to the list.

**5. The summary.** The names of the failed tests once more, and one line of
counts. `31 filtered out` means that 31 other tests exist and were not chosen
by your filter. The closing line, `error: test failed`, is the one you met in
lesson 01: Cargo's note that the run was not completely green.

So there are three kinds of failure, and you tell them apart by the message:

| The message says | What happened | Where the line points |
| --- | --- | --- |
| ``assertion `left == right` failed`` and two values | Your function returned the wrong value. | Into the test. |
| A sentence that the test author wrote | A check with `assert!` was false. | Into the test. |
| Something else, such as `index out of bounds` or `attempt to divide by zero` | Your function panicked before it returned. | Into your function. |

A function that is not written yet belongs to the third kind. Its message is
`not yet implemented`, followed by the text inside `todo!(...)`.

### Backtraces

The note at the end of a block offers more detail. A **backtrace** is the
chain of function calls that led to the panic. Set the variable
`RUST_BACKTRACE` to 1 for one command:

```sh
RUST_BACKTRACE=1 cargo test --test chapter01 l03_
```

In Windows PowerShell, type `$env:RUST_BACKTRACE=1` on one line and the cargo
command on the next. For the example above, the interesting part is:

```text
stack backtrace:
   ...
   2: core::panicking::panic_bounds_check
             at /rustc/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/core/src/panicking.rs:271:5
   3: chapter01::sample::day_name
             at ./exercises/chapter01/sample.rs:9:5
   4: chapter01::sample::tests::friday_is_the_last_working_day
             at ./exercises/chapter01/sample.rs:25:20
   ...
```

Read from the top and skip every entry whose path starts with `/rustc/` or
leads into a directory named `.rustup`; those are inside Rust's own library.
Entries in your files have a path that starts with `./`. The first of them
is where the panic happened (line 9, in `day_name`). The entry below it is
the caller (line 25, in the test: `day_name(4)`).

### Running fewer tests

The word after the group name is a filter. Cargo runs the tests whose full
name contains it:

```sh
cargo test --test chapter01 l03_
cargo test --test chapter01 l03_failing_test::tests::a_
cargo test --test chapter01 -- --list
```

The first command runs the whole lesson, the second only the tests of task A,
and the third prints the names of all tests in the chapter without running
them.

### Just enough Rust for the tasks: `if` and `else`

Two tasks of this lesson need a function that gives one value in one case and
another value in the other case. In Rust, `if` and `else` do that directly,
because an `if` with an `else` produces a value:

```rust
/// Postage in cents: a letter of up to 20 grams costs 85, a heavier one 160.
fn postage(grams: u32) -> u32 {
    if grams <= 20 { 85 } else { 160 }
}

fn main() {
    assert_eq!(postage(5), 85);
    assert_eq!(postage(20), 85);
    assert_eq!(postage(21), 160);
}
```

The condition has no parentheses. Each branch is a block in braces, and the
value of the branch that runs becomes the value of the whole `if`. Because
the `if` is the last line of the function, without a semicolon, that value
is what the function returns. The comparisons are `<` and `>`, `<=` ("at
most") and `>=` ("at least"), `==` and `!=`.

### Mistakes people really make

**Changing the test until it passes.** The test states what is required. If
you edit `assert_eq!(hours_in(2), 48)` into `24`, the report turns green and
the function is still wrong. In this course, never change a supplied test.

**Repairing only the case that the test names.** A body such as
`if days == 2 { 48 } else { days * 12 }` passes this test and is still wrong
for every other number. Find the rule, not the case.

**Reading only the last line.** `1 passed; 2 failed` tells you that something
is wrong. The line with `panicked at` tells you where, and the message tells
you what.

**Mixing up the two sides.** If you forget which side is yours, look at the
test line: the value that comes from the call of your function is `left`.

### Read more

- [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html) and
  [`assert!`](https://doc.rust-lang.org/std/macro.assert.html) in the
  standard library.
- [The Cargo book: cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [The rustc book: Tests](https://doc.rust-lang.org/rustc/tests/index.html)
  lists the options that can follow `--`.
- [`std::backtrace`](https://doc.rust-lang.org/std/backtrace/index.html)
  describes `RUST_BACKTRACE`.

## Lesson 04 · Reading a compiler error

### The idea

Before a Rust program runs, the compiler checks all of it. If it finds one
error anywhere, it builds nothing, and nothing runs. This is different from
languages that start the program and stop at the first broken line.

Two consequences matter for this course:

- One compile error in one lesson file stops **all** tests of that chapter
  from running, because a chapter is compiled as one unit. Repair compile
  errors first. `cargo check --test chapter01` shows them quickly.
- The compiler is strict, and it explains itself. Its messages name the
  place, show the source lines, and often propose the repair. People who
  read the messages completely learn Rust much faster than people who read
  only the first line.

### A worked example

```rust,compile_fail,E0384
fn main() {
    let lives = 3;
    println!("lives: {lives}");
    lives = 2;
    println!("lives: {lives}");
}
```

```text
error[E0384]: cannot assign twice to immutable variable `lives`
 --> src/main.rs:4:5
  |
2 |     let lives = 3;
  |         ----- first assignment to `lives`
3 |     println!("lives: {lives}");
4 |     lives = 2;
  |     ^^^^^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable
  |
2 |     let mut lives = 3;
  |         +++

For more information about this error, try `rustc --explain E0384`.
error: could not compile `hello` (bin "hello") due to 1 previous error
```

### The parts of the message

**1. The headline**: `error[E0384]: cannot assign twice to immutable variable`.
The first word is the level. An `error` stops the build. A `warning` does
not: the program is built, and the compiler tells you about something that
is probably a mistake. The code in square brackets, `E0384`, names the kind
of error. The rest of the line says in one sentence what is wrong.

**2. The arrow**: `--> src/main.rs:4:5`. The file, the line and the column
where the problem is. Open the file there.

**3. The source lines.** The compiler prints the lines that matter, with
their line numbers on the left. Marks under the lines point at exact places:

- `^^^^` is the **primary** mark: the place where the error is.
- `----` is a **secondary** mark: another place that explains the error.

Each mark can have a label. Here the secondary mark says where `lives` got
its first value, and the primary mark says that the second assignment is not
allowed. Read the labels as two sentences of one explanation.

**4. `help:` and `note:`.** A `help` proposes a change, often with the
changed line printed and `+++` under the added text. A `note` gives
background. Here the help says: write `let mut lives`. In Rust a variable
cannot be changed unless it is declared with `mut`; Chapter 2 explains this.

**5. The closing lines.** `rustc --explain E0384` prints a longer
description of the error kind, with examples. The last line counts the
errors. It is a summary, not a new error.

A **statement**, by the way, is one step of a program that ends with a
semicolon, such as `let lives = 3;`.

### A method for many errors

A program with several mistakes produces a long output. Work like this:

1. Scroll up to the **first** message. Later messages are often caused by
   the first one.
2. Read all of it: headline, arrow, labels, help.
3. Repair that one error.
4. Compile again (`cargo check`). Do not try to repair the rest from the old
   output. It is out of date now.

The messages are not sorted by line number. The compiler works in several
passes: it first checks that all names exist, and later that all types fit.
An error from an early pass is printed before an error from a later pass,
even when it is further down in the file.

### Errors you will meet in the first weeks

| Code | Headline | Usual cause |
| --- | --- | --- |
| `E0425` | cannot find value `x` in this scope | A misspelled name, or a variable used outside the block where it was declared. |
| `E0308` | mismatched types | A value of one type where another type is required: text for a number, or a function that returns nothing where a value is required. |
| `E0384` | cannot assign twice to immutable variable | A variable is changed, but was declared without `mut`. |
| `E0061` | this function takes 2 arguments but 1 argument was supplied | A call with the wrong number of values. |
| (none) | expected `;`, found ... | A missing semicolon. Most errors in the basic shape of the text have no code. |
| `E0382` | borrow of moved value | A value was handed to another owner and then used again. Chapter 4 is about this rule. |

A missing semicolon shows something general: the compiler reports the place
where it **noticed** the problem, and the cause can be a little earlier.

```text
error: expected `;`, found `println`
 --> src/main.rs:2:18
  |
2 |     let lives = 3
  |                  ^ help: add `;` here
3 |     println!("lives: {lives}");
  |     ------- unexpected token
```

The secondary mark is on line 3, where the compiler met a word that it did
not expect. The primary mark and the help are at the end of line 2, where
the semicolon belongs.

### Practice: the compiler drills

A lesson file must always compile, so the tested tasks of this lesson can
only show you messages. The real practice is `cargo drill ch01`: ten small
programs in `drills/ch01/`. For each one:

1. Open the file and read the program. Do not compile it.
2. Decide what will happen, and write your answer on the line
   `//! EXPECT: ?` in place of the question mark. The possible answers are
   `ok` (it compiles and runs), `panic` (it compiles and then panics),
   `error` (a compile error without a code), or an error code such as
   `E0308`.
3. Run `cargo drill ch01`. The tool compiles the drill, shows the real
   result next to your prediction, and prints the compiler's message.
4. Repair the program so that it compiles and runs. Lines that end in
   `// KEEP` must stay as they are, so that you cannot repair a program by
   deleting the problem. A drill whose real result is `ok` needs no repair.
5. Run `cargo drill ch01` again. A repaired drill gets a check mark, and the
   tool moves on to the next one.

The tool calls the compiler `rustc` directly, without Cargo. So the closing
line of a message is `error: aborting due to 1 previous error` here, where
Cargo prints `error: could not compile ...`. Everything above that line is
the same.

When a verdict surprises you, read the note for that drill in the *Drills*
section of [HINTS.md](HINTS.md), and write one sentence in
[NOTES.md](NOTES.md).

### Mistakes people really make

**Starting at the bottom.** The terminal shows the end of the output. The
end is the summary and the last error. Scroll up.

**Reading only the headline.** "mismatched types" alone says little. The
label under the marks says which type was expected and which was found.

**Ignoring `help:`.** In the first weeks the proposed change is very often
exactly the right repair. Read it, understand why it helps, then apply it.

**Treating warnings as noise.** A warning does not stop the build, but it is
the compiler telling you that something looks wrong. In this course,
warnings about *your* code should be repaired. Warnings about parameters of
functions that still contain `todo!()` are hidden for you.

### Read more

- [The Rust error code index](https://doc.rust-lang.org/error_codes/error-index.html)
  has the text of every `rustc --explain`.
- [The rustc book: Lints](https://doc.rust-lang.org/rustc/lints/index.html)
  explains warnings and the levels `allow`, `warn` and `deny`.

## Lesson 05 · The lesson loop: predict, implement, explain

### The idea

Reading about a programming language feels like learning, and most of it is
gone a week later. What stays is what you had to **produce** yourself: an
answer that you committed to before you saw the result, code that you wrote
against a failing test, an explanation in your own words. Every lesson of
this course is built as a loop of such steps:

1. **Read** the lesson's section in CONCEPTS.md.
2. **Predict.** Answer the `p_predictions` test in the lesson file. Replace
   each `guess()` with the value that you expect. Do not run anything first.
3. **Implement.** Write the bodies that say `todo!()`, and run the lesson
   until the report is green.
4. **Explain.** Write in NOTES.md which predictions were wrong and why, and
   answer the lesson's EXPLAIN question. Add one test of your own. Then
   compare your text with *Check your explanation* in HINTS.md.

A wrong prediction is not a bad mark. It shows you exactly where your picture
of the language differs from the language. That is the most useful thing a
lesson can tell you, and you only get it if you answer before you run.

### A full worked lesson

Here is a complete small lesson, from the untouched file to the finished
notes. It is not in the repository; it is printed here so that you can see
every step once. Its file name in the outputs below is `demo.rs`.

**The file as you receive it.**

```rust
//! LESSON — A worked lesson: the car park
//! Run: cargo test --test chapter01 demo

/// A. Return the number of wheels on `cars` cars. A car has 4 wheels.
fn wheels(cars: u32) -> u32 {
    todo!("A: 4 wheels per car")
}

/// B. The speed limit in the car park is 10. Return true when `speed` is
/// above the limit.
fn too_fast(speed: u32) -> bool {
    todo!("B: compare speed with 10")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_four_wheels_per_car() {
        assert_eq!(wheels(3), 12);
        assert_eq!(wheels(0), 0);
    }

    #[test]
    fn b_only_speeds_above_the_limit_are_too_fast() {
        assert!(too_fast(11), "11 is above the limit");
        assert!(!too_fast(10), "exactly 10 is allowed");
        assert!(!too_fast(3), "3 is allowed");
    }

    #[test]
    fn p_predictions() {
        // P1. What is the value of this expression?
        let p1: u32 = guess();
        assert_eq!(
            p1,
            2 + 2 * 10,
            "P1: 2 + 2 * 10 (left: your prediction, right: actual)"
        );
    }
}
```

The text above each function, after `///`, is the **contract**: it says what
the function must do. The tests check the contract. (A comment that starts
with `///` is called a *doc comment*. A comment that starts with `//` is a
note for the reader of that line.) The line
`use practice::guess;` brings in `guess` from the course's small support
library. (The complete file was compiled and run inside the course package,
and the reports below are from those runs.)

**Step 1: predict.** The learner in this example reads P1 and thinks: "2 plus
2 is 4, times 10 is 40." She replaces `guess()`:

```rust
        let p1: u32 = 40;
```

**Step 2: implement.** She writes both bodies:

```rust
fn wheels(cars: u32) -> u32 {
    cars * 4
}

fn too_fast(speed: u32) -> bool {
    speed >= 10
}

fn main() {
    assert_eq!(wheels(3), 12);
    assert!(too_fast(10)); // The contract says this should be false.
}
```

**Step 3: run and read.**

```text
running 3 tests
test demo::tests::a_four_wheels_per_car ... ok
test demo::tests::b_only_speeds_above_the_limit_are_too_fast ... FAILED
test demo::tests::p_predictions ... FAILED

failures:

---- demo::tests::b_only_speeds_above_the_limit_are_too_fast stdout ----

thread 'demo::tests::b_only_speeds_above_the_limit_are_too_fast' (123194060) panicked at exercises/chapter01/demo.rs:29:9:
exactly 10 is allowed

---- demo::tests::p_predictions stdout ----

thread 'demo::tests::p_predictions' (123194061) panicked at exercises/chapter01/demo.rs:37:9:
assertion `left == right` failed: P1: 2 + 2 * 10 (left: your prediction, right: actual)
  left: 40
 right: 22
...
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 31 filtered out; finished in 0.00s
```

Task A is green. Two things are red, and she reads both blocks as lesson 03
taught.

*Task B*: the message is a sentence from the test, "exactly 10 is allowed",
at line 29: `assert!(!too_fast(10), ...)`. So `too_fast(10)` returned true
and must return false. The contract says "above the limit". Her comparison
`>=` means "above or equal". The repair is `speed > 10`.

*The prediction*: left is her 40, right is the real value, 22. She does not
simply type 22 and go on. She asks why: Rust multiplies before it adds, as
in school arithmetic, so the expression is 2 + 20.

**Step 4: repair, run again, explain.** With `speed > 10` and `p1 = 22`:

```text
running 3 tests
test demo::tests::a_four_wheels_per_car ... ok
test demo::tests::p_predictions ... ok
test demo::tests::b_only_speeds_above_the_limit_are_too_fast ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 31 filtered out; finished in 0.00s
```

Green is not the end of the lesson. She writes in NOTES.md:

```text
Predictions I got wrong, and why:
- P1: I said 40. I calculated from left to right. `*` is done before `+`,
  so 2 + 2 * 10 is 2 + 20 = 22.

Edge case I added:
- 1000 cars. The supplied test only had 0 and 3.
```

**Step 5: a test of her own.** She adds one function to the tests:

```rust
    #[test]
    fn a_car_park_with_a_thousand_cars() {
        assert_eq!(wheels(1000), 4000);
    }
```

A test has three parts: the line `#[test]`, a function without parameters
whose name says what is checked, and one or more assertions in the body. The
report now says `4 passed`.

In most lessons of the course you add your own test next to the supplied
ones, as here. A few lessons, among them lessons 05 and 06 of this chapter,
prepare a separate section called `mod my_tests` for it, because there a
supplied test checks that you really wrote one.

### Whole numbers: division and remainder

The tasks of this lesson calculate with whole numbers (`u32`). Two operators
behave differently from a pocket calculator:

```rust
fn main() {
    // `/` between whole numbers gives a whole number. The rest is dropped.
    assert_eq!(17 / 5, 3);
    // `%` gives the rest (the remainder) of that division.
    assert_eq!(17 % 5, 2);
    // Together they give back the number: 3 * 5 + 2.
    assert_eq!((17 / 5) * 5 + 17 % 5, 17);
    // `*` and `/` are done before `+` and `-`; equal ranks go left to right.
    assert_eq!(1 + 6 / 3, 3);
    assert_eq!(20 / 2 / 5, 2);
    // Comparisons give a bool. `&&` means "and", `||` means "or".
    assert_eq!(17 % 5 == 2 && 17 / 5 == 3, true);
}
```

Because the rest is dropped at every division, the **order** of the
operations can change the result. `seconds / 60 * 60` is not always
`seconds`: for 90 it is 60. When you mix `*` and `/`, think about which one
should come first.

### The course tools

**`cargo progress`** runs the tests of every chapter and prints one line for
each chapter. Before you have done anything, the output starts and ends like
this:

```text
Lessons: ✓ all tests pass   ◐ some tests pass   · no test passes yet

Building the lesson tests (the first run takes longer) ... done
ch01  0/6  ·l01 ·l02 ·l03 ·l04 ·l05 ·l06
ch02  0/11 ·l01 ·l02 ·l03 ·l04 ·l05 ·l06 ·l07 ·l08 ·l09 ·l10 ◐c01
...
Next lesson: cargo test --test chapter01 l01_
```

Each lesson has a mark, explained in the first line. The last line is the
command for your next unfinished lesson. When a whole chapter is green, the
tool starts a review plan for it: it asks you to come back to the chapter
after 1, 3, 7, 21 and 60 days. The plan is kept in the directory
`.progress/`, which belongs to you and is not shared. Run `cargo progress`
at the start of every session.

**`cargo drill ch01`** runs the compiler drills of a chapter, as described in
lesson 04. `cargo drill` without a chapter prints how many drills you have
predicted and how many of the predictions were right.

**HINTS.md** has help for each lesson, in steps. In chapters 1 and 2 there
are three hints for each lesson, and the third one is nearly the answer.
Open them one at a time, and only after you have really tried.

**Solutions** for every lesson are in the directory `solutions/` at the top
of the course, in files with the same names; `cargo progress solution ch01
l05` prints the path of one. They are there so that you are never stuck for
good. Use them after your own attempt and after the hints, and compare: what
did the model solution do differently, and why?

### Mistakes people really make

**Running first and filling in the predictions from the report.** The test
turns green and you have learned nothing from it. Answer first, also when
you are unsure. Especially when you are unsure.

**Correcting a prediction without asking why it was wrong.** The number in
the report is not the lesson. The reason is.

**Opening the solution when the first test fails.** Reading a solution is
easy and feels productive. The effort of finding it yourself is the part
that stays in memory.

**Stopping at green.** The explanation and the edge case take five minutes
and decide whether you still know this next month.

### Read more

- [The Rust Reference: Operator expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html)
  defines `/`, `%` and the comparison operators.
- [The Rust Reference: Expression precedence](https://doc.rust-lang.org/reference/expressions.html#expression-precedence)
  lists which operator is done first.

## Lesson 06 · Capstone: the parking meter

The capstone uses everything from this chapter in one small file, and the
text does not lead you through it. Two tasks have empty bodies. One task is
already written and wrong, as in lesson 03. You answer the predictions,
write one test of your own, and explain the loop as you really did it.

You need one piece of Rust again that lesson 03 introduced: `if` and `else`
produce a value.

```rust
/// A ticket costs 300 cents. Children under 6 travel free.
fn ticket_price(age: u32) -> u32 {
    if age < 6 { 0 } else { 300 }
}

fn main() {
    assert_eq!(ticket_price(4), 0);
    assert_eq!(ticket_price(6), 300);
    assert_eq!(ticket_price(40), 300);
}
```

The condition has no parentheses. Each branch is a block in braces, and the
value of the branch that runs becomes the value of the whole `if`. Because
the `if` is the last line of the function, without a semicolon, that value
is what the function returns.

And you need one fact about `u32`. A `u32` cannot be negative, so a
subtraction whose result would be below zero has no valid answer. In a
normal build (the `dev` and `test` profiles), the program panics there:

```rust,should_panic
fn minutes_left(used: u32) -> u32 {
    60 - used
}

fn main() {
    println!("{}", minutes_left(75));
}
```

```text
thread 'main' (123248377) panicked at src/main.rs:2:5:
attempt to subtract with overflow
```

In a `--release` build this check is switched off to save time, and the
result is a huge wrong number instead of a panic. Chapter 3 covers this
properly. For now the rule is simple: make sure that a subtraction on `u32`
runs only when the left side is at least as large as the right side. An
`if` does that.

When all tests of the capstone pass, run `cargo progress`. Chapter 1 should
show a check mark for every lesson.

### Read more

- [The Rust Reference: `if` expressions](https://doc.rust-lang.org/reference/expressions/if-expr.html)
- [`u32` in the standard library](https://doc.rust-lang.org/std/primitive.u32.html)
