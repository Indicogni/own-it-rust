# Start here

[Course overview](../README.md) · [All guides](README.md)

## What this course is

*Own It: Hands-On Mastery of Rust* is a course that you work through in a Git repository: <https://github.com/Indicogni/own-it-rust>. It takes you from your first Rust program to designing, building and reviewing real Rust software: libraries, services, concurrent code, unsafe code, bindings to C, and tools.

You learn by writing code. Every chapter has a text that teaches the ideas (`CONCEPTS.md`), lessons with tests that check your code on your own machine, compiler drills, and model solutions. You do not need a book or another course.

## Who it is for

You have written some code in another language: variables, functions, `if` and loops. That is enough for Chapter 1. The course also suits people who already use Rust but want to know *why* their code compiles, and teams who want a shared path for new colleagues.

## How long it takes

The 24 core chapters take about 210 to 235 hours for a careful first pass. This is an estimate; your time will differ. After the core come four checkpoints, a running project, an ecosystem track, eleven mastery modules and seven capstone projects. You choose how far to go. Sessions of 45 to 60 minutes work well.

## Three ways to start

All three ways give you the same tools. Choose one.

**Your own copy.** Your work should be saved in a repository that belongs to you. Sign in to GitHub (a free account is enough), open <https://github.com/Indicogni/own-it-rust>, and click **Use this template**. For ways 1 and 3, choose **Create a new repository**: this is your own copy. For way 2, see its steps.

### 1. A dev container on your computer (recommended)

A *dev container* (development container) is a small Linux system with every tool the course needs, running in a box on your computer.

1. Install Docker Desktop, or a free alternative: Podman, Rancher Desktop and, on macOS, Colima. OrbStack (macOS) is free for personal use. VS Code supports Docker officially; the others usually work, but are not officially supported.
2. Install VS Code and its **Dev Containers** extension.
3. In VS Code, run **Dev Containers: Clone Repository in Container Volume...** and paste the address of your copy.
4. Wait for the first start. It can take ten minutes or more, because it installs Rust and compiles the lessons. Later starts take seconds.

### 2. GitHub Codespaces in your browser (optional)

A *codespace* is the same dev container, running on a GitHub computer. You use VS Code in your web browser and install nothing.

1. Start a codespace in one of two ways (choose one):
   - On the course page, click **Use this template**, then **Open in a codespace**. Early on, publish your work: in the *Source Control* view, commit and click **Publish Branch**.
   - Or, if you already made your own copy with **Create a new repository**, open your copy and click **Code**, **Codespaces**, **Create codespace**.
2. Wait for the first start.

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/Indicogni/own-it-rust)

Codespaces uses your own free monthly allowance. On a free GitHub account that is about 30 hours of work a month for this course. A codespace that stays unused for 30 days is deleted, with everything you did not push. So commit and push at the end of every session.

### 3. Install the tools yourself

Install Git and Rust with rustup (<https://www.rust-lang.org/tools/install>). Then clone your copy (or the course itself, as here):

```sh
git clone https://github.com/Indicogni/own-it-rust.git
cd own-it-rust
cargo test --locked --offline --all-targets --no-run
```

The course pins Rust 1.99.0. rustup installs that version the first time you run `cargo` in the course directory.

The [Setup section of the course README](../README.md#setup) has every step for all three ways, and what Codespaces costs.

## Your first hour

1. Open a terminal in the course directory and check your tools:

   ```sh
   cargo progress doctor
   ```

2. Run `cargo progress`. It lists every chapter and lesson, keeps an untouched copy of each lesson, and prints the command for your next lesson.
3. Open Chapter 1's [CONCEPTS.md](../exercises/chapter01/CONCEPTS.md) and read Lesson 01.
4. Run your first lesson: `cargo test --test chapter01 l01_`. Four tests fail. That is the normal start, not a broken setup.
5. Work through the lesson with the loop that every lesson uses:
   - **Read** the lesson's section in `CONCEPTS.md`.
   - **Predict.** Each lesson has a test called `p_predictions`. Replace each `guess()` with the value you expect, *before* you run anything.
   - **Implement** the `todo!()` bodies, and run the lesson's tests until they pass.
   - **Explain** what you learned in the chapter's `NOTES.md`, then compare with the chapter's `HINTS.md`.
6. Save your work: commit and push.

In the first hour you will probably finish Lesson 01 and start Lesson 02. Chapter 1 takes about four and a half hours in total.

## When you are stuck

- **Hints.** Every chapter has a `HINTS.md` with hints for each lesson. Each hint says more than the one before. Open them one at a time, after a real attempt.
- **Solutions.** Model solutions for every lesson are in `solutions/`. `cargo progress solution ch01 l01` prints the path of one. Use them after the hints, and compare: what did the model do differently, and why?
- **Errors in the course.** If a test, a hint or a sentence is wrong, open an issue at <https://github.com/Indicogni/own-it-rust/issues>.
