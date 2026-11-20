//! The one-line summary that the command line prints after a run.

use tasklane_core::{Registry, RunOptions, Runner, Vars, Workspace, parse};

fn summary(source: &str, target: &str) -> String {
    let file = parse(source).unwrap();
    let registry = Registry::with_builtins();
    let mut ws = Workspace::new();
    Runner::new(&registry)
        .with_options(RunOptions::new().keep_going(true))
        .run(&file, target, &Vars::new(), &mut ws)
        .unwrap()
        .summary()
}

#[test]
fn summary_counts_each_status() {
    let source = "task a\n  step echo a\ntask b : a\n  step echo b\ntask c : b\n";
    assert_eq!(summary(source, "c"), "3 done, 0 failed, 0 skipped");
}

#[test]
fn summary_names_failed_tasks() {
    let source = "task lint\n  step fail style\n\
                  task test\n  step fail 2 tests failed\n\
                  task docs\n  step echo ok\n\
                  task ci : lint test docs\n";
    assert_eq!(
        summary(source, "ci"),
        "1 done, 2 failed, 1 skipped; failed: lint (fail: style), test (fail: 2 tests failed)"
    );
}
