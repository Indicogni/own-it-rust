//! Lesson 07, task A: what `attempt!` does.
//! This program must compile and run.

use m5_proc_macros::attempt;
use std::cell::Cell;

/// Fails `failures` times, then succeeds. Counts its calls.
struct Flaky {
    failures: u32,
    calls: Cell<u32>,
}

impl Flaky {
    fn new(failures: u32) -> Flaky {
        Flaky {
            failures,
            calls: Cell::new(0),
        }
    }

    fn step(&self) -> Result<u32, String> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if call > self.failures {
            Ok(call)
        } else {
            Err(format!("call {call} failed"))
        }
    }
}

/// `?` inside the macro's argument must leave *this* function.
fn question_mark(text: &str) -> Result<u32, std::num::ParseIntError> {
    let doubled: Result<u32, String> = attempt!(3, Ok(text.parse::<u32>()? * 2));
    Ok(doubled.expect("the expression above never gives Err") + 1)
}

/// `return` inside the macro's argument must leave this function too.
fn early_return(flag: bool) -> &'static str {
    let _: Result<(), ()> = attempt!(2, {
        if flag {
            return "returned from inside";
        }
        Ok(())
    });
    "reached the end"
}

fn main() {
    // Succeeds on the third try.
    let flaky = Flaky::new(2);
    assert_eq!(attempt!(5, flaky.step()), Ok(3));
    assert_eq!(flaky.calls.get(), 3, "stop at the first Ok");

    // Never succeeds: the last error is the result.
    let flaky = Flaky::new(u32::MAX);
    assert_eq!(attempt!(3, flaky.step()), Err("call 3 failed".to_owned()));
    assert_eq!(flaky.calls.get(), 3, "at most MAX tries");

    // MAX of 0 and of 1 both mean one try.
    for max in [0, 1] {
        let flaky = Flaky::new(u32::MAX);
        assert_eq!(attempt!(max, flaky.step()), Err("call 1 failed".to_owned()));
        assert_eq!(
            flaky.calls.get(),
            1,
            "at least one try, also for MAX = {max}"
        );
    }

    // MAX is evaluated once, and before the first try.
    let order = Cell::new(Vec::new());
    let note = |what: &'static str| {
        let mut seen = order.take();
        seen.push(what);
        order.set(seen);
    };
    let flaky = Flaky::new(2);
    let result = attempt!(
        {
            note("max");
            4
        },
        {
            note("try");
            flaky.step()
        },
    );
    assert_eq!(result, Ok(3));
    assert_eq!(order.take(), ["max", "try", "try", "try"]);

    // The types follow the expression: here `Result<&str, u8>`.
    let fixed: Result<&str, u8> = attempt!(2, Err(7));
    assert_eq!(fixed, Err(7));

    assert_eq!(question_mark("21"), Ok(43));
    assert!(question_mark("x").is_err());
    assert_eq!(early_return(true), "returned from inside");
    assert_eq!(early_return(false), "reached the end");
}
