//! LESSON 06, tasks B and C — tests that run whole programs.
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --test l06_process
//!
//! The programs are `src/bin/panicker.rs` (supplied; read it for the predictions)
//! and `src/bin/stockroom.rs` (the program you repair).

// `guess` is no longer used once the predictions are answered.
#![allow(unused_imports)]

use m9_toolbox::guess;
use m9_toolbox::support::{self, Run};
use std::process::Command;
use std::time::Duration;

fn run(program: &str, args: &[&str], backtrace: Option<&str>) -> Run {
    let mut command = Command::new(program);
    command.args(args);
    // Start from a known state, whatever the learner's shell has set.
    command.env_remove("RUST_BACKTRACE");
    command.env_remove("RUST_LIB_BACKTRACE");
    if let Some(value) = backtrace {
        command.env("RUST_BACKTRACE", value);
    }
    support::run(command, program.to_owned(), Duration::from_secs(60))
}

fn panicker(args: &[&str], backtrace: Option<&str>) -> Run {
    run(env!("CARGO_BIN_EXE_panicker"), args, backtrace)
}

fn stockroom() -> Run {
    run(env!("CARGO_BIN_EXE_stockroom"), &[], None)
}

/// C. After the repair the program exits normally and prints the right stock.
///
/// Starting stock: anvil 2, bolt 500, crank 0 (discontinued), gear 40, hinge 12.
/// Then: 30 hinges arrive, discontinued items are dropped, and three orders ship:
/// 120 bolts, 25 gears, 20 hinges.
#[test]
fn l06_c_stockroom_ships_every_order_from_the_right_item() {
    let run = stockroom();
    assert_eq!(
        run.code,
        Some(0),
        "stockroom must exit normally, but stderr says:\n{}",
        run.stderr
    );
    assert_eq!(
        run.stdout, "anvil: 2\nbolt: 380\ngear: 15\nhinge: 22\n",
        "the program no longer panics, but the stock is wrong. Silencing the panic \
         where it appears (a bounds check, a saturating subtraction) hides the damage \
         instead of preventing it: some order was taken from the wrong item."
    );
}

/// C. The repair belongs where the stale data is produced, not where it is used.
#[test]
fn l06_c_stockroom_fix_is_at_the_cause() {
    let source = support::read(&support::package_root().join("src/bin/stockroom.rs"));
    for patch in [
        "saturating_sub",
        "wrapping_sub",
        "checked_sub",
        "get_mut(",
        ".get(",
    ] {
        assert!(
            !source.contains(patch),
            "`{patch}` treats the symptom. `ship` is correct for a correct index; \
             make the index correct."
        );
    }
}

/// LAB 6.1 (needs a debugger: rust-lldb or rust-gdb; see LABS.md). Run with:
/// `cargo test --manifest-path mastery/m9_toolbox/Cargo.toml lab_l06 -- --ignored`
#[test]
#[ignore = "LAB: needs rust-lldb or rust-gdb; see LABS.md"]
fn lab_l06_what_the_debugger_shows() {
    // In the ORIGINAL program, stopped at the panic, in the frame of `ship`:
    // what is `self.items[3].quantity`? (Lab 6.1 shows how to print it.)
    let quantity: &str = "17";
    assert!(
        support::lab_answer_is_right("l06.debugger.quantity", quantity),
        "that is not the value in the stopped program. It is not 42: something \
         changed this item before the panic."
    );
}

/// P. Predict BEFORE running: replace each `guess()` with your answer.
///
/// Read `src/bin/panicker.rs` first.
#[test]
fn l06_p_predictions() {
    // P1. `panicker` with RUST_BACKTRACE unset. Which exit code does a Rust program
    // have after a panic in `main`'s thread?
    let plain = panicker(&[], None);
    let p1: i32 = 101;
    assert_eq!(Some(p1), plain.code, "P1: the exit code of a panic");

    // P2. Same run: how many lines does stdout have? (Is the line printed before the
    // panic lost?)
    let p2: usize = 1;
    assert_eq!(
        p2,
        plain.stdout.lines().count(),
        "P2: a panic unwinds; output written before it is still flushed"
    );

    // P3. How many of these three runs print a "stack backtrace:" section?
    //   RUST_BACKTRACE unset,  RUST_BACKTRACE=0,  RUST_BACKTRACE=1
    let with_backtrace = [None, Some("0"), Some("1")]
        .into_iter()
        .filter(|setting| panicker(&[], *setting).stderr.contains("stack backtrace:"))
        .count();
    let p3: usize = 1;
    assert_eq!(
        p3, with_backtrace,
        "P3: backtraces are off unless asked for"
    );

    // P4. `panicker --dbg` evaluates `dbg!(2 + 3)`. On which stream does the line
    // `[src/bin/panicker.rs:LINE:COLUMN] 2 + 3 = 5` appear? Answer "stdout" or "stderr".
    let dbg = panicker(&["--dbg"], None);
    let actual = if dbg.stdout.contains("2 + 3 = 5") {
        "stdout"
    } else {
        "stderr"
    };
    let p4: &str = "stderr";
    assert_eq!(
        p4, actual,
        "P4: dbg! must not disturb a program's real output"
    );

    // P5. `panicker --hook` with RUST_BACKTRACE=1: the program installs its own
    // panic hook. Does stderr still contain "stack backtrace:"?
    let hooked = panicker(&["--hook"], Some("1"));
    let p5: bool = false;
    assert_eq!(
        p5,
        hooked.stderr.contains("stack backtrace:"),
        "P5: the message and the backtrace are printed by the default hook; \
         a hook that replaces it must print what it wants to keep"
    );
}
