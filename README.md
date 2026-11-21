# Own It: Hands-On Mastery of Rust

An original, repository-based course by Indicogni LLC, published on [indicogni.com](https://indicogni.com).

**Mastery of Rust by doing.** This course takes you from your first Rust program to the level where you can design, build, review and defend real Rust software: libraries, services, concurrent and unsafe code, foreign-function bindings, and tools. Everything you need is in this repository. Every chapter teaches its own material in a `CONCEPTS.md` text written for this course, every exercise is checked by tests on your machine, and every model solution ships with the course and is verified by the same checks. You do not need any other resource; links to the free official documentation are there when you want to read further.

## Contents

- [Who it is for](#who-it-is-for)
- [How the method works](#how-the-method-works)
- [Setup](#setup)
- [The path](#the-path)
- [How to know you have mastered Rust](#how-to-know-you-have-mastered-rust)
- [Solutions](#solutions)
- [Tools](#tools)
- [Guides](#guides)
- [The course in numbers](#the-course-in-numbers)
- [Licence, trademarks and errors](#licence-trademarks-and-errors)

## Who it is for

You have written some code in another language: variables, functions, `if` and loops. That is all Chapter 1 assumes. The course suits:

- programmers who are new to Rust and want to become fluent, not just able to read it;
- Rust users who can make code compile but want to know *why* it compiles, and want to write libraries, unsafe abstractions, async services and FFI with confidence;
- teams who want a shared, tested path for onboarding.

**You need:** a computer with Linux, macOS or Windows (see [Supported platforms](#supported-platforms)), or only a web browser and a free GitHub account if you use [GitHub Codespaces](#way-2-github-codespaces-in-your-browser-optional); Git; and a few gigabytes of free disk space: the pinned toolchain takes under 1 GB, and a debug build of all chapters about 0.5 GB. Some mastery modules and capstones download crates and need more; their READMEs say so. A few units need extra tools, such as a C compiler, a [nightly toolchain](#the-nightly-toolchain) with Miri, or QEMU; their READMEs list them before you start.

If you know C, C++, Go, Python, Java, C#, JavaScript or TypeScript, read the matching [guide](#guides) first. It tells you which chapters to give the most time.

## How the method works

Recognising Rust when you read it is not the same as writing it under pressure. The course is built on the techniques that make skills last: predicting before you see the answer, retrieving from memory, immediate feedback, explaining in writing, and reviewing on a spaced schedule. Every lesson follows the same loop:

1. **Read** the lesson's section in the chapter's `CONCEPTS.md`. It explains the mental model, works through an example that is different from the exercise, shows the mistakes people really make with the compiler's real message, and links to the official documentation.
2. **Predict.** Each lesson file ends with a `p_predictions` test. Replace each `guess()` with what you think the code produces, *before you run anything*. A wrong guess is the most useful result you can get: write down why in the chapter's `NOTES.md`. (When a file's predictions are all answered, `guess` becomes an unused import. Every chapter allows that, so the compiler stays quiet; delete `guess` from the `use` line anyway, as the solutions do.)
3. **Implement** the `todo!()` bodies from their written contracts, and run only that lesson: `cargo test --test chapter04 l01_`.
4. **Check your technique.** Some tests count heap allocations, compare pointers or count threads, so a solution that clones everything or runs sequentially fails even when its output is right. Run `cargo clippy --test chapter04` and fix its warnings in your code.
5. **Drill the compiler.** `cargo drill ch04` gives you short programs. Predict whether each compiles (and with which error code) or panics, see the verdict, then fix it. Do them where the chapter's README places them in the study order.
6. **Explain.** Answer the lesson's EXPLAIN prompt in `NOTES.md`, add a test for an edge case, then compare with *Check your explanation* in the chapter's `HINTS.md`.
7. **Review on a schedule.** When a whole chapter passes, `cargo progress` schedules five short reviews, each one different (see [Spaced review](#spaced-review)).

Every chapter from 2 on has **C01**, a code review of a teammate's code that compiles but has bugs and smells. Every chapter from 4 on also has **R01**, a review lesson that mixes earlier chapters into new problems, done from memory. Every chapter ends with a **capstone** that combines its lessons. Chapters 1 and 2 give three hints per lesson instead of two, and Chapter 1 teaches the loop itself.

When you finish a chapter, also fill in its row in [DECISIONS.md](DECISIONS.md), which condenses the course into the judgement calls you make while writing real code.

## Setup

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/Indicogni/own-it-rust)

You can set up the course in three ways. All three give you the same tools and the same results.

| Way | What you need | Good for |
| --- | --- | --- |
| [1. A dev container on your computer](#way-1-a-dev-container-on-your-computer-recommended) (recommended) | Docker or a free alternative, and VS Code | Everyone. Every tool is ready, nothing is installed on your computer itself, and it works offline after the first start. |
| [2. GitHub Codespaces in your browser](#way-2-github-codespaces-in-your-browser-optional) (optional) | A free GitHub account and a web browser | Trying the course without installing anything, or working from a computer that is not yours. |
| [3. Install the tools yourself](#way-3-install-the-tools-yourself) | Git, rustup, and a few extra tools for some units | People who prefer their own setup, and the units that need your real machine (Docker labs, the game window at full speed). |

A **dev container** (development container) is a Linux system that runs in a box on your computer. The file `.devcontainer/devcontainer.json` in this repository describes it: which Rust version, which tools and which editor extensions it has. VS Code reads that file, builds the container, and then works inside it. GitHub Codespaces uses the same file, but runs the container on a GitHub computer instead of yours.

**Your own copy.** Your work is safe only when it is saved in a repository that belongs to you. This repository is a *template*: on its GitHub page, click **Use this template**. You then have two choices:

- **Create a new repository**: you get your own copy on GitHub, with your own name on it, and you can make it private. For Way 1 and Way 3, do this first, and use your copy in the steps below. (You can also clone the course directly; then you cannot push your work to GitHub until you make your own repository.)
- **Open in a codespace**: this starts a codespace (Way 2) at once, and you publish it as your own repository later.

For Way 2, choose one of the two; you do not need both (see the steps below).

Your copy contains the course's GitHub Actions workflows (the automatic checks in `.github/workflows/`). They run only in the course repository itself, so in your copy they are skipped and use none of your Actions minutes.

### Way 1: A dev container on your computer (recommended)

1. **Install a container tool.** [Docker Desktop](https://www.docker.com/products/docker-desktop/) works on macOS, Windows and Linux. Check its licence terms if you use it at work: large companies need a paid plan. Free alternatives are [Podman](https://podman.io/), [Rancher Desktop](https://rancherdesktop.io/) and, on macOS, [Colima](https://github.com/abiosoft/colima). [OrbStack](https://orbstack.dev/) (macOS) is free for personal use. VS Code supports Docker officially; the others usually work, but are not officially supported. On Linux, Docker Engine is enough. On Windows, Docker Desktop uses WSL 2 (the Windows Subsystem for Linux).
2. **Give the container enough room.** Allow at least 4 CPUs and 8 GB of memory. Less works, but builds are slow. On macOS, and on Windows without WSL 2, set this in your container tool's settings. On Windows with WSL 2, Docker uses the limits of WSL (by default about half of your memory and all CPUs); change them in the *WSL Settings* app (in the Start menu) or in the file `.wslconfig` in your user folder if needed. On Linux there is nothing to set.
3. **Install [VS Code](https://code.visualstudio.com/)** and its **Dev Containers** extension (extension ID `ms-vscode-remote.remote-containers`).
4. **Open the course in a container.** In VS Code, open the Command Palette (F1, or Ctrl+Shift+P; Cmd+Shift+P on macOS) and run **Dev Containers: Clone Repository in Container Volume...**. Paste the URL of your copy, for example `https://github.com/YOUR-NAME/own-it-rust`. On macOS and Windows this is the faster choice, because the files stay inside the container's own storage (a *volume*) instead of being shared with your computer's disk.
   - Or clone the repository in the usual way, open the folder in VS Code (*File*, *Open Folder*), and choose **Reopen in Container** when VS Code offers it (or run **Dev Containers: Reopen in Container**). Your files then stay in a normal folder on your disk. This is simple, but builds are slower on macOS and Windows.
5. **Wait for the first start.** The first time, the container tool downloads and builds the container, installs Rust 1.99.0, and compiles the tests of every chapter (without running them), so that your first lesson starts quickly. This can take ten minutes or more. Later starts take seconds.
6. **Open a terminal inside the container**: *Terminal*, *New Terminal* in VS Code. Every command in this course is typed there. Go on with [Check your setup](#check-your-setup).

Inside the container you already have Rust 1.99.0 with rustfmt, clippy, rust-src and the `aarch64-unknown-none` target, a C compiler, QEMU for the kernel capstone, gdb (with `rust-gdb`) for the M9 debugger lab, the libraries of the game capstone's window, Python 3 and Git. VS Code inside the container has the rust-analyzer, CodeLLDB and Even Better TOML extensions. Two things are not included: the [nightly toolchain](#the-nightly-toolchain) with Miri (one command installs it when you need it), and Docker itself, so do the Docker labs of M9 and M11 on your own computer, outside the container.

If you used **Clone Repository in Container Volume**, your files live in that volume, not in a folder you can see on your disk. If you delete the volume, your work is gone unless you pushed it (see [Save your work](#save-your-work)).

Other editors: JetBrains IDEs and the [Dev Container CLI](https://github.com/devcontainers/cli) can also use the same `.devcontainer/devcontainer.json`. With the CLI, `devcontainer up --workspace-folder .` starts the container and `devcontainer exec --workspace-folder . bash` opens a shell in it. Use a current version of the CLI: older versions (0.80, for example) wait forever after "Container started" with Docker 29, because they do not recognise its newer event format. Version 0.91.0 works.

### Way 2: GitHub Codespaces in your browser (optional)

A **codespace** is a dev container that runs on a GitHub computer. You use VS Code in your browser, and nothing is installed on your computer. It runs on your own GitHub account and uses your free monthly allowance (see below).

1. Sign in to GitHub. A free account is enough.
2. Start the codespace in one of two ways. They are alternatives; choose one.
   - **(a)** Open the course repository, <https://github.com/Indicogni/own-it-rust>, click **Use this template**, then **Open in a codespace**. You publish your work as your own repository later (step 5).
   - **(b)** If you already made your own copy with **Create a new repository**, open your copy, click **Code**, open the **Codespaces** tab, and click **Create codespace**. Your work then goes to your copy from the start.
   - The **Open in GitHub Codespaces** button at the top of this section opens a codespace on the course repository itself. You cannot push to the course repository. When you commit there, GitHub links your codespace to a copy (a *fork*) under your account.
3. Wait for the first start (several minutes; it builds the container and compiles the chapters' tests). VS Code opens in the browser with this README and Chapter 1's `CONCEPTS.md`.
4. Open a terminal (*Terminal*, *New Terminal*) and go on with [Check your setup](#check-your-setup).
5. **Publish your work.** If you chose (a), **Open in a codespace**, your work is not in any repository yet. In the Source Control view, commit your changes and click **Publish Branch**; VS Code then creates a repository under your account (choose private or public). Do this early. If you delete a codespace that you never published, your work is deleted with it.

**What Codespaces costs, and how long your work stays.** These are GitHub's rules on 20 November 2026; check [GitHub's billing page for Codespaces](https://docs.github.com/en/billing/concepts/product-billing/github-codespaces) for the current numbers.

- **Your free allowance.** A free personal GitHub account includes 120 core hours of compute time and 15 GB-month of storage each month (GitHub Pro: 180 core hours and 20 GB-month). A "core hour" is one processor core for one hour. This course asks for a machine with 4 cores and 8 GB of memory, so one hour of work uses 4 core hours. 120 core hours are therefore **about 30 hours of work a month**. If you have not added a payment method to your account, GitHub does not charge you: when the allowance is used up, you cannot use Codespaces again until the next month.
- **Storage counts while a codespace exists,** even when it is stopped. Delete codespaces you no longer need (after you pushed your work) at <https://github.com/codespaces>.
- **Idle timeout.** A codespace stops after 30 minutes without activity (you can change this between 5 and 240 minutes in your GitHub settings). A stopped codespace keeps your files, and you can start it again from <https://github.com/codespaces>.
- **Automatic deletion.** A codespace that has been stopped and unused for 30 days is deleted automatically, **even if it has changes you did not push.**

So: **your work is safe only when it is committed and pushed** to your repository on GitHub. Push at the end of every session.

To see the game window of the [game capstone](capstones/game/README.md) in a dev container or a codespace, see its README. The game runs more smoothly on your own computer.

### Way 3: Install the tools yourself

<a id="fresh-clone-setup"></a>
Install Git and [Rust through rustup](https://www.rust-lang.org/tools/install). The repository pins Rust **1.99.0** with rustfmt, clippy and rust-src in `rust-toolchain.toml`, and uses edition 2024; rustup installs the pinned toolchain the first time you run Cargo in the directory. The course was last verified against Rust 1.99.0. Rust 1.100.0, released on 12 November 2026, is newer than the pin; where it changes what a lesson teaches (the never type `!` and `Infallible`, in [Chapter 6](exercises/chapter06/CONCEPTS.md) and [M1](mastery/m1_types/CONCEPTS.md) lesson 06), the lesson has a marked note. Keep the pin while you work through the course, so that your compiler's messages match the text.

```sh
git clone https://github.com/Indicogni/own-it-rust.git
cd own-it-rust
cargo test --locked --offline --all-targets --no-run
cargo progress
```

- If you made your own copy from the template, clone that copy instead: put its URL after `git clone`, and `cd` into the folder it creates.
- The third command compiles every chapter without running any unfinished code. The chapters use only the standard library, so no network is needed after rustup has installed the toolchain.
- The first build prints one `warning[E0133]` for `exercises/chapter23/c01_code_review.rs`. That warning is part of that code-review lesson.
- Run `cargo progress` once before you edit anything: it saves a copy of every untouched lesson, so that you can redo a lesson later.
- Every exercise starts unfinished. Tests fail at `todo!()` and at `guess()` until you fill them in. That is the starting state, not a broken setup.

Run `cargo drill` and `cargo progress` from the repository root or from any directory of the root package (`exercises/`, `drills/`, `solutions/`, ...). The project, the ecosystem track, the mastery modules, the capstones and the checkpoints are standalone Cargo packages: work inside their directories, and change back to the repository root to use the coach.

`src/main.rs` is a playground for your own experiments: `cargo run --features playground`. Each chapter also has a scratch file, for example `cargo run --example ch04_scratch`.

Some units need extra tools: a C compiler (Chapter 24, M6, M11), QEMU and the `aarch64-unknown-none` target (the kernel capstone), a debugger (the M9 debugger lab), and the [nightly toolchain](#the-nightly-toolchain) with Miri. The dev container has all of them except the nightly toolchain. On your own computer, each unit's README says how to install what it needs.

### Check your setup

Whichever way you chose, run this in the course directory:

```sh
cargo progress doctor
```

It prints where it runs (a codespace, a dev container, or your own machine) and checks your tools. The required checks are Rust 1.99.0 (the version this directory pins), clippy and rustfmt; if one of them fails, the command says how to fix it and ends with an error. The other checks are optional and only report: rust-src, Git, the `aarch64-unknown-none` target and QEMU (kernel capstone), a C compiler (Chapter 24, M6), `rust-gdb` or `rust-lldb` (M9 debugger lab), `nightly-2026-11-20` with Miri (Miri labs) and Docker (the Docker labs of M9 and M11). A missing optional tool matters only when you reach the unit that needs it.

Then run `cargo progress` once, before you edit anything (see above), and start with [Chapter 1](exercises/chapter01/README.md).

### Save your work

Your solutions, your `NOTES.md` files and your history in `.progress/` are all files in your copy of the repository. Save them with Git: a **commit** records a snapshot of your files in your copy, and a **push** sends your commits to your repository on GitHub.

- **In VS Code:** open the Source Control view (the branch icon on the left). Enter a short message, such as `Chapter 1, lessons 01 to 03`, and choose **Commit** (VS Code asks whether to stage all changes; say yes). Then click **Sync Changes** or **Push**. In the dropdown next to **Commit** there is also **Commit & Push**.
- **In a terminal:**

  ```sh
  git add -A
  git commit -m "Chapter 1, lessons 01 to 03"
  git push
  ```

Commit and push at the end of every session. In a codespace this matters most: a codespace is deleted after 30 days without use, and anything you did not push is deleted with it. On your own computer, a push is also your backup.

### Supported platforms

The course has been verified on macOS on Apple silicon. The "Course checks" workflow also runs every check on Linux and Windows; treat those platforms as verified only once that workflow is green for the version you cloned.

The dev container and Codespaces run Linux (on any computer). The "Dev container" workflow ([`.github/workflows/devcontainer.yml`](.github/workflows/devcontainer.yml)) builds the dev container and checks the tools in it, so it counts as verified when that workflow is green too. The Docker labs of M9 and M11 cannot run inside the container; do them on your own computer.

### Editor setup

**In the dev container or a codespace,** VS Code already has rust-analyzer, CodeLLDB (a debugger front end) and Even Better TOML (for `Cargo.toml` files), and rust-analyzer works as soon as the first build has finished. The notes below about the separate packages still apply.

**On your own computer,** any text editor works. An editor with rust-analyzer, the official Rust language server (in VS Code, the "rust-analyzer" extension), shows errors, types and completions while you type. Open the repository folder, not a single file.

rust-analyzer loads the Cargo package of the folder you open, here the root package with all chapters, and does not look for other packages inside it. The project, the ecosystem track, the mastery modules, the capstones and the checkpoints are standalone packages, so their files get no errors, completion or go-to-definition in a window opened on the repository root. When you work on one of them, do one of two things:

- Open that package's folder (for example `mastery/m2_unsafe`) in its own editor window. This is the simplest way.
- Or list the package's `Cargo.toml` in the setting [`rust-analyzer.linkedProjects`](https://rust-analyzer.github.io/book/configuration.html#linkedProjects). The setting turns off automatic discovery, so list the root `Cargo.toml` too. Relative paths are relative to the folder you opened. The repository's `.vscode/settings.json` already lists the root package and the running project; add the package you work on:

  ```json
  {
    "rust-analyzer.linkedProjects": ["Cargo.toml", "project/Cargo.toml", "ecosystem/Cargo.toml"]
  }
  ```

  Keep the list short: rust-analyzer checks every package you list, and some of them download crates.

## The path

The core path is 24 chapters in five parts, with four checkpoints along the way. Beside it run a project that you grow across the chapters and a track of production crates. After the core come eleven mastery modules and seven capstone projects, the last of which is the final assessment.

```text
Part 0  ch01-02  on-ramp
Part 1  ch03-08  foundations ............ cp1 after ch07
Part 2  ch09-15  types and abstraction .. cp2 after ch11
Part 3  ch16-21  data, text, I/O, concurrency .. cp3 after ch16, cp4 after ch21
Part 4  ch22-24  macros, unsafe, FFI
        loglens running project (milestones from ch03 to ch23)
        ecosystem track (E01 after ch07, the rest as their chapters are done)
Then    mastery modules M1-M11, in any order their prerequisites allow
        capstones, then the final assessment
```

All times below are estimates for a careful first pass, including reading, predictions and notes. They come from each unit's own README or CONCEPTS.md. Your times will differ. Sessions of 45 to 60 minutes work well.

### Part 0 · On-ramp (about 15 hours)

| Chapter | You learn | Time |
| --- | --- | --- |
| [01 · Start here](exercises/chapter01/README.md) | the tools, reading test failures and compiler errors, the lesson loop | about 4.5 h |
| [02 · A first tour](exercises/chapter02/README.md) | the whole everyday language once: types, functions, loops, text, vectors, structs, enums, `match`, `Option`, `Result`, tests | about 10 h |

### Part 1 · Foundations (about 53 to 62 hours)

| Chapter | You learn | Time |
| --- | --- | --- |
| [03 · Numbers, text and arrays](exercises/chapter03/README.md) | integers, overflow, floats, conversions, arrays, slices, text and Unicode | about 11 h |
| [04 · Ownership and moves](exercises/chapter04/README.md) | owners, moves, copies, clones, `Rc` and `Arc` | 7 to 8 h |
| [05 · Borrowing with references](exercises/chapter05/README.md) | borrowing, lifetimes, shared versus exclusive access | 9 to 11 h |
| [06 · Expressions and control flow](exercises/chapter06/README.md) | blocks, branches, `let else`, loops, casts, temporaries | 9 to 11 h |
| [07 · Handling errors](exercises/chapter07/README.md) | panics, `Result`, `?`, error types, `main` and exit codes | 8 to 10 h |
| **[Checkpoint 1](checkpoints/README.md)** | `splitpot`, from a specification, closed notes | at most 4 h |
| [08 · Modules, crates and Cargo](exercises/chapter08/README.md) | packages, modules, visibility, tests, features, a checked workspace lab | 9 to 11 h |

### Part 2 · Types and abstraction (about 60 to 67 hours)

| Chapter | You learn | Time |
| --- | --- | --- |
| [09 · Structs and methods](exercises/chapter09/README.md) | methods, generics, derives, `Cell`, `RefCell`, `Weak` parent links | 12 to 13 h |
| [10 · Enums and pattern matching](exercises/chapter10/README.md) | data-carrying enums, recursive types, every kind of pattern | about 8 h |
| [11 · Generic code with traits](exercises/chapter11/README.md) | traits, bounds, `dyn` versus generics, associated items | about 9 h |
| **[Checkpoint 2](checkpoints/README.md)** | `postage`, from a specification, closed notes | at most 4 h |
| [12 · Operators for your own types](exercises/chapter12/README.md) | arithmetic, equality, ordering, indexing, and their contracts | 5 to 6 h |
| [13 · Everyday traits](exercises/chapter13/README.md) | `Drop`, `Clone`, `Deref`, `AsRef`, `Borrow`, conversions, `Cow` | 8 to 9 h |
| [14 · Closures and captures](exercises/chapter14/README.md) | captures, `Fn`/`FnMut`/`FnOnce`, returning and storing closures | 7 to 9 h |
| [15 · Iterators and adapters](exercises/chapter15/README.md) | adapters, laziness, consumers, writing your own iterators | 11 to 13 h |

### Part 3 · Data, text, I/O and concurrency (about 55 to 61 hours)

| Chapter | You learn | Time |
| --- | --- | --- |
| [16 · Standard collections](exercises/chapter16/README.md) | `Vec`, `VecDeque`, heaps, maps, sets, hashing | 10 to 11 h |
| **[Checkpoint 3](checkpoints/README.md)** | `matchday`, from a specification, closed notes | at most 4 h |
| [17 · Working with text](exercises/chapter17/README.md) | UTF-8, editing, searching, parsing, formatting, a regex lab | 10 to 11 h |
| [18 · Files, streams and processes](exercises/chapter18/README.md) | `Read`, `Write`, buffering, binary data, paths, TCP, child processes | 10 to 12 h |
| [19 · Threads and shared state](exercises/chapter19/README.md) | scoped threads, channels, locks, condition variables, atomics, orderings | about 10 h |
| [20 · Writing async code](exercises/chapter20/README.md) | futures, borrowing across `.await`, async traits and closures, cancellation | 7 to 8 h |
| [21 · Futures by hand](exercises/chapter21/README.md) | `poll`, wakers, an executor, a reactor pattern, pinning | 8 to 9 h |
| **[Checkpoint 4](checkpoints/README.md)** | `seal`, from a specification, closed notes | at most 4 h |

### Part 4 · Macros, unsafe code and FFI (about 28 to 31 hours)

| Chapter | You learn | Time |
| --- | --- | --- |
| [22 · Writing macros](exercises/chapter22/README.md) | `macro_rules!`, repetition, fragments, recursion, exporting, tt-munchers | 9 to 10 h |
| [23 · Unsafe Rust](exercises/chapter23/README.md) | raw pointers, safe interfaces over unsafe code, `UnsafeCell`, layout, Miri | 11 to 12 h |
| [24 · Calling C](exercises/chapter24/README.md) | `repr(C)`, C strings, callbacks, handles, calling the real C library | 8 to 9 h |

The five parts add up to about 210 to 235 hours, plus up to 16 hours for the checkpoints. Check a chapter off only when its lessons, predictions, drills, R01, C01, notes and capstone are complete.

### Checkpoints

A [checkpoint](checkpoints/README.md) is a test of what you can do without help: a specification, an almost empty file, behaviour and technique tests, and a rubric that maps every failed test back to the lessons behind it. Take each one in one sitting of at most four hours, with closed notes, at least two days after finishing the chapters it covers. Reaching "proficient" in all four means that you can design and build small Rust programs by yourself.

### The running project and the ecosystem track

| Track | When | What it practises | Time |
| --- | --- | --- | --- |
| [loglens, the running project](project/README.md) | start after Chapter 3; each milestone follows its chapter | one real command-line log analyser built in 15 milestones (16 test files; milestone 18 has two): zero-copy parsing, error types, traits, a filter language compiled to closures, iterator pipelines, streaming I/O, parallel files, macros, an optional async reader and an optional unsafe fast path. Later milestones rewrite earlier code while the earlier tests keep you safe. Standard library only. | 35 to 50 h |
| [Ecosystem track](ecosystem/README.md) | E01 after Chapter 7; the rest as their chapters are done | nine lessons on the crates and tools that professional Rust uses almost everywhere: thiserror and anyhow, serde, clap, tracing, proptest, tokio, a capstone job runner, a tooling workday, and an HTTP service with axum and reqwest. Each lesson ends with judgement: why this crate, and when you would choose differently. Pinned dependencies (one download, then offline). | 33 to 44 h |

### Mastery modules

Take the mastery modules after the core path, in any order their prerequisites allow. Each is a standalone package with a `CONCEPTS.md`, lessons with predictions and technique tests, hints, a notes workbook, model solutions, and (for M1 to M8) a drill set: `cargo drill m1` from the repository root.

| Module | What you build or practise | Prerequisites | Time |
| --- | --- | --- | --- |
| [M1 · The type system in depth](mastery/m1_types/README.md) | variance, lifetime bounds, higher-ranked bounds, GATs and lending iterators, `PhantomData`, trait objects, sealed traits, typestate, coherence | Chapters 5, 8 to 11, 13 to 15; 19 and 23 for one lesson | about 20 h |
| [M2 · Unsafe Rust by rebuilding the standard library](mastery/m2_unsafe/README.md) | `Cell`, `RefCell`, `Box`, `Vec` and its iterators, `Rc`/`Weak`, a queue, a spin lock, an arena; the aliasing model; Miri | Chapters 4, 5, 9, 13, 15, 19, 23; nightly with Miri | about 40 h |
| [M3 · Concurrency by building the primitives](mastery/m3_concurrency/README.md) | memory orderings, a spin lock, a one-shot channel, `Arc`, a parking mutex, a channel, lock-free structures; loom | Chapters 9, 13, 19, 23 (M2 helps) | 30 to 40 h |
| [M4 · An async runtime from scratch](mastery/m4_async/README.md) | a raw waker, tasks and `spawn`, timers, a reactor, pin projection, streams, `select`, a chat server, then the same on tokio | Chapters 18 to 21 and 23; ecosystem E06 for the last lesson | 26 to 29 h |
| [M5 · Procedural macros](mastery/m5_proc_macros/README.md) | derives, attribute and function-like macros, `syn` and `quote`, error spans, compile-fail tests, a `#[derive(Builder)]` | Chapters 7 to 11, 13, 22 | about 17 h |
| [M6 · FFI across a real language boundary](mastery/m6_ffi/README.md) | a build script that compiles C, hand-written and generated bindings, safe wrappers, callbacks, exporting Rust to C | Chapters 5, 8, 13, 14, 17, 19, 23, 24; a C compiler; reading simple C | about 27 h |
| [M7 · Performance engineering](mastery/m7_perf/README.md) | benchmark design, profiles, allocation budgets, data layout, bounds checks, dispatch costs, I/O; tests that count work instead of reading clocks | Chapters 1 to 18 and 23 | 22 to 25 h |
| [M8 · Library design, semver and stewardship](mastery/m8_api/README.md) | breaking changes, trait and auto-trait promises, future-proofing, library errors, features, docs, MSRV | Chapters 7 to 11, 13, 15, 19 | about 18 h |
| [M9 · The professional toolbox](mastery/m9_toolbox/README.md) | Cargo in depth, editor setup, lints as policy, testing at scale, fuzzing, debugging, documentation, supply chain, CI and release | Chapters 1 to 22; ecosystem E05 and E08 | about 32 to 35 h with labs |
| [M10 · Working in real codebases and contributing](mastery/m10_contributing/README.md) | orienting in a strange project, reproducing bugs, `git bisect`, changes maintainers accept, code review, soundness reports, upgrades, contributing to the Rust project | Chapters 1 to 24; Git | 26 to 28 h |
| [M11 · Rust as a service](mastery/m11_services/README.md) | an HTTP API on axum and tower, SQL with sqlx, authentication, CPU work beside async I/O, a binary protocol, observability, testing services | Chapters 7, 11, 13, 18 to 20; ecosystem E01, E02, E04, E06; basic SQL; a C compiler | 45 to 50 h |

### Capstones and the final assessment

The capstones are blank-file projects: a specification, acceptance tests per milestone, a `CONCEPTS.md` for the ideas outside Rust, design hints, and a rubric. You design every type and module yourself.

| Capstone | You build | Prerequisites | Time |
| --- | --- | --- | --- |
| [Tansy, an interpreter](capstones/interpreter/README.md) | a scripting language with a REPL: lexer, parser, tree-walking evaluator, closures, diagnostics | Chapters 3 to 11, 13 to 19; M1, M7, M9 | 30 to 50 h |
| [kvstore](capstones/kvstore/README.md) | a key-value server that survives crashes: checksummed log, recovery, compaction, a TCP protocol, concurrency, graceful shutdown | Chapters 3 to 5, 7 to 11, 16 to 19; M3, M8 | 25 to 45 h |
| [httpd](capstones/httpd/README.md) | an HTTP/1.1 file server with its own thread pool, limits and a JSON endpoint, standard library only | Chapters 7 to 10, 13 to 19; M3, M7 | 25 to 40 h |
| [nostd collections](capstones/nostd_collections/README.md) | `ArrayVec`, `RingBuffer` and an arena for `#![no_std]` programs, checked with Miri | Chapters 4, 5, 13, 15, 16, 23; M1, M2, M8; nightly with Miri | 20 to 30 h |
| [Underkiln, a graphical game](capstones/game/README.md) | a real-time 2D game with its own rasteriser, fixed-timestep loop, collisions, replays and a headless mode | Chapters 3 to 5, 7 to 11, 15 to 18; M7, M8 | 40 to 70 h |
| [A kernel for QEMU `virt`](capstones/kernel/README.md) | a bare-metal Arm kernel: boot, serial port, memory, interrupts, tasks, message passing | Chapters 11, 13, 23, 24; M2, M3; QEMU | 40 to 60 h |
| **[Final assessment: rgx](capstones/final_assessment/README.md)** | a regular-expression engine with guaranteed linear-time matching, and a parallel, deterministic `grep`-like tool | Chapters 1 to 21 and 23; all four checkpoints; M1, M3, M7, M8 | 35 to 60 h |

You do not need every capstone. A good plan is two capstones that match your work (for example kvstore and httpd for services, nostd collections and the kernel for systems work), then the final assessment.

## How to know you have mastered Rust

Passing tests is necessary, not sufficient. The course gives you three increasingly demanding pieces of evidence:

1. **The core path.** Every chapter's tests pass in debug and release builds, clippy is quiet, your NOTES.md explains every lesson in your own words, and you have done all five spaced reviews. This shows that you know the language and its standard library and can explain them.
2. **The four checkpoints at "proficient".** You built four programs from a specification alone, within four hours each, with closed notes, and they are correct for reasons you chose. This shows that you can work without guidance.
3. **The final assessment at "mastery".** You wrote a non-trivial engine and tool with no hints, no concepts text and no dependencies. Its [rubric](capstones/final_assessment/RUBRIC.md) first requires the tests to pass three times in a row in both profiles with no warnings. The "mastery" level then asks for every row (design, correctness beyond the tests, idiom, your own tests, performance, unsafe hygiene) at "proficient" or better, at least half of them at "mastery", and all twenty design questions answered in writing. Reaching it shows work that you could publish and maintain, with every decision backed by a reason or a number.

What it does not prove: the grades are self-assessed, so they are only as honest as you are, and no course can certify you. The rubrics are written so that every cell can be checked by a command, by reading your code for a named thing, or by a small experiment. Apply them strictly, or ask an experienced Rust programmer to apply them to your work. The mastery modules add depth in the areas where your work will need it; M10 then takes you into other people's code and the Rust project itself.

## Solutions

Model solutions ship in [`solutions/`](solutions/), which mirrors the repository: `solutions/<path>` is the solved version of `<path>`. Drill solutions are under `solutions/drills/`, with the expected outcome of every starter drill in `expected.tsv`. Every solution is checked by the course's scripts and CI: it keeps the starter's tests unchanged and passes them in debug and release builds.

- **Attempt first.** A solution you only read is forgotten within days; one you struggled with first stays. Use the hints before the solution.
- `cargo progress solution ch04 l01` prints the path of a model solution, with a reminder to attempt first. It also accepts drills (`ch04 d01`), mastery lessons (`m1 l01`), the project (`project record`) and other units; without a lesson it lists the unit's solutions.
- For the **checkpoints** and the **final assessment**, read the solution only after you have graded your own work against the rubric. Reading it first turns an assessment into a reading exercise.
- Solutions are model answers, not the only answers. If yours is different and passes, compare the trade-offs.

## Tools

### The coach

| Command | What it does |
| --- | --- |
| `cargo test --test chapter04 l01_` | Run one lesson (`r01_`, `c01_` for the review and the code review). |
| `cargo clippy --test chapter04` | Lint a chapter; fix the warnings in your code. |
| `cargo drill` | Summary of every drill set. |
| `cargo drill ch04` / `cargo drill m1` | Work through a drill set; keys are `chNN` for chapters and `mN` for mastery modules. |
| `cargo drill path/to/drill.rs` | Work on one drill; the path may be relative to the current directory. |
| `cargo drill reset ch04` | Restore a drill set to predict it again. |
| `cargo progress` | Core chapters: status, percent passing, next lesson, reviews due. Fast. |
| `cargo progress all` | The same, plus one row each for the project, the ecosystem track, the mastery modules, the capstones and the checkpoints. Slow; the first run downloads crates. |
| `cargo progress reviewed ch04` | Record a review when one is due; add `--early` to record it sooner. |
| `cargo progress redo ch04 l08` | Archive your solution and restore the lesson's starter. |
| `cargo progress solution ch04 l01` | Show where the model solution is. |
| `cargo progress doctor` | Check your setup: where it runs, the required tools (Rust 1.99.0, clippy, rustfmt) and the optional ones, each with the fix when it is missing. See [Check your setup](#check-your-setup). |
| `cargo run --example ch04_scratch` | Free-form experiments for a chapter. |

In the `cargo progress` table, ✓ means all tests pass, ◐ some tests pass and · no test passes yet. On a fresh clone, every C01 and three lessons with supplied helper tests (Chapter 18 L09, Chapter 22 L08 and Chapter 23 L09) show ◐, because some of their tests pass from the start. A chapter whose test program crashes shows `crashed (reason)` and is never counted. A chapter still running after 120 seconds is stopped and its unfinished tests are named. The limits can be changed with `COACH_TEST_SECS` (default 120), `COACH_DRILL_SECS` (20) and `COACH_PACKAGE_SECS` (900). Drill predictions accept `ok` or `compiles`, `panic` or `panics`, `error` or `compile error`, or error codes in any punctuation (`error[E0382].`). `redo` never overwrites an earlier archive and does nothing when the lesson still equals its starter. Your history lives in `.progress/` at the repository root, together with the starter copies that `redo` restores (about 2 MB). Git does not ignore it, so it is saved with your commits and survives when a codespace or a container is deleted. Commit it with the rest of your work.

### The nightly toolchain

The course runs on the pinned Rust 1.99.0. A few optional checks need a nightly toolchain: Miri (an interpreter that finds undefined behaviour in unsafe code) in Chapter 23, the running project, several mastery modules and capstones, and some labs of M9 (fuzzing, sanitizers, docs.rs-style documentation, minimal versions). All of them use one dated nightly, the same one the "Course checks" workflow uses: **`nightly-2026-11-20`** (`rustc 1.102.0-nightly (6905905bb 2026-11-19)`). It is pinned because Miri's messages, and now and then its verdicts, change from one nightly to the next; with the date fixed, your output matches the reports quoted in the course. Install it once:

```sh
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
```

Then write `+nightly-2026-11-20` in a command to use it for that command only, for example `cargo +nightly-2026-11-20 miri test --test chapter23`. loom (M3) does not need nightly; it runs on the pinned toolchain.

### Spaced review

Review 1 is due one day after a whole chapter first passes. Each later review is due 3, 7, 21 and 60 days after the previous one was recorded (dates are UTC). Each review is short and different; the coach prints the exact steps:

| Review | About | What you do |
| --- | --- | --- |
| 1 | 10 min | Re-answer the "Predictions I got wrong" entries in NOTES.md, and the drills you missed on the first try. |
| 2 | 10 min | Answer the Mastery check questions from memory. |
| 3 | 30 to 45 min | Redo the capstone from its contract (`cargo progress redo chNN lNN`). |
| 4 | 15 min | `cargo drill reset chNN`, every drill again, plus two mastery questions. |
| 5 | 20 min | Redo R01 (C01 where there is no R01) and write five sentences that teach the chapter's hardest idea. |

The coach explains at most two due reviews per day and lists the rest as "also due". Finish each with `cargo progress reviewed chNN`.

### Maintainer scripts

The scripts in [`scripts/`](scripts/) need Python 3.9 or newer and change nothing in the repository unless stated. [AUTHORING.md](AUTHORING.md) explains when to run them.

| Script | What it checks |
| --- | --- |
| `python3 scripts/check_starters.py --strict --run` | Starter structure, forbidden phrases, and that every prediction fails at `guess()` |
| `python3 scripts/check_solutions.py --only ch04` | Lays `solutions/` over a copy of the course and runs the unit's tests and drills (add `--clippy --repeat 3`) |
| `python3 scripts/check_concepts.py ch04` | Compiles and runs the code blocks of a `CONCEPTS.md` |
| `python3 scripts/check_links.py` | Every relative Markdown link and anchor resolves |
| `python3 scripts/make_notes.py --all --check` | Every chapter's `NOTES.md` matches its generator (writes a workbook only when asked) |

The "Course checks" workflow in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs these checks on Linux, macOS and Windows: it compiles every starter in both profiles and runs the coach, checks structure, links, notes and concept code blocks, runs every model solution and drill, and runs Miri and loom on the solved unsafe and concurrent code. This workflow and the "Dev container" workflow run only in the course repository, not in your copy (see [Your own copy](#setup)).

## Guides

- [Start here](guides/START_HERE.md): the course on one page, the three ways to set up, and your first hour
- [Coming to Rust from C and C++](guides/FROM_C_AND_CPP.md)
- [Coming to Rust from Go](guides/FROM_GO.md)
- [Coming to Rust from Python](guides/FROM_PYTHON.md)
- [Coming to Rust from Java and C#](guides/FROM_JAVA_AND_CSHARP.md)
- [Coming to Rust from JavaScript and TypeScript](guides/FROM_JAVASCRIPT_AND_TYPESCRIPT.md)
- [Taming compile times](guides/COMPILE_TIMES.md)
- [Hint directory](HINTS.md): every unit's hints in one list
- [Decision rules](DECISIONS.md): the judgement calls the course trains, with model answers

## The course in numbers

Counted from the starter files on 20 November 2026 by a script, not estimated:

| What | Count |
| --- | --- |
| Chapters | 24 |
| Chapter lessons (not counting capstones) | 183 |
| Chapter capstones / R01 review lessons / C01 code reviews | 24 / 21 / 23 |
| Mastery modules / their lessons (including module capstones) | 11 / 95 |
| loglens milestone test files / ecosystem lessons | 16 / 9 |
| Checkpoints / capstone projects (including the final assessment) | 4 / 7 |
| Compiler drills, in 32 drill sets | 339 |
| Predictions (answers you write in place of `guess()`) | 1,127 |
| `#[test]` functions in the starters | 3,979 |

## Licence, trademarks and errors

**Licence.** The code is under the [MIT License](LICENSE): every source and configuration file (`*.rs`, `*.toml`, `*.py`, `*.c`, `*.h`, `*.yml`, scripts, test fixtures), including the comments in those files, and every fenced code block in the Markdown files, except diagrams and figures in `text` blocks, which are prose. The prose and figures (the text of every `*.md` file and every diagram or figure, including diagrams and figures in `text` blocks) are under [Creative Commons Attribution-NonCommercial-ShareAlike 4.0](LICENSE-TEXT): you may share and adapt them for non-commercial use with credit and under the same licence. [NOTICE.md](NOTICE.md) explains the split. Copyright (c) 2026 Indicogni LLC.

**Trademarks.** This is an independent course. It is not affiliated with or endorsed by the Rust Foundation or any publisher. "Rust" is a trademark of the Rust Foundation, used under its trademark policy.

**Reporting errors.** Corrections are very welcome. If a test, a hint, a solution or a sentence is wrong, open an issue in the [course repository][repo]; [CONTRIBUTING.md](CONTRIBUTING.md) lists what to include. Pull requests cannot be merged until a contributor agreement is published, so please describe the correction in the issue.

[repo]: https://github.com/Indicogni/own-it-rust
