//! A user who asked for a dry run (fixture issue 21) and wrote this code against the
//! API that the maintainers accepted in the issue. Do not edit this crate: make the
//! fixture provide what it uses.

use tasklane_core::{Error, Registry, RunOptions, RunReport, Runner, Vars, Workspace, parse};

/// Plans `target` and checks every step, but runs nothing.
pub fn preview(source: &str, target: &str) -> Result<(Workspace, RunReport), Error> {
    let file = parse(source)?;
    let registry = Registry::with_builtins();
    let mut ws = Workspace::new();
    ws.write("already-there.txt", "untouched");
    let options = RunOptions::new().dry_run(true);
    assert!(options.is_dry_run());
    let report = Runner::new(&registry).with_options(options).run(
        &file,
        target,
        &Vars::from_lanefile(&file),
        &mut ws,
    )?;
    Ok((ws, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tasklane_core::TaskStatus;

    const SOURCE: &str = "set OUT = build\n\
                          task a\n  step write ${OUT}/a.txt 1\n  step remove already-there.txt\n\
                          task b : a\n  step fail never\n\
                          task c : b\n  flags silent\n  step echo hello\n";

    #[test]
    fn dry_run_is_off_by_default() {
        assert!(!RunOptions::new().is_dry_run());
        assert!(!RunOptions::default().is_dry_run());
        assert!(!RunOptions::new().dry_run(true).dry_run(false).is_dry_run());
    }

    #[test]
    fn a_dry_run_runs_no_step() {
        let (ws, report) = preview(SOURCE, "c").unwrap();
        assert_eq!(
            ws.paths().collect::<Vec<_>>(),
            ["already-there.txt"],
            "a dry run must not create or remove files"
        );
        assert_eq!(ws.read("already-there.txt"), Some("untouched"));
        let names: Vec<&str> = report.results().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["a", "b", "c"],
            "the report lists the plan, in plan order"
        );
        for result in report.results() {
            assert_eq!(result.status, TaskStatus::Skipped, "task {}", result.name);
            assert_eq!(
                result.message.as_deref(),
                Some("dry run"),
                "task {}",
                result.name
            );
            assert_eq!(result.steps_run, 0, "task {}", result.name);
        }
        assert!(report.success(), "`fail` did not run, so nothing failed");
        assert_eq!(report.summary(), "0 done, 0 failed, 3 skipped");
    }

    #[test]
    fn a_dry_run_still_finds_mistakes() {
        assert_eq!(
            preview("task a\n  step polish x\n", "a").err(),
            Some(Error::UnknownStep {
                task: "a".to_string(),
                step: "polish".to_string()
            })
        );
        assert_eq!(
            preview("task a\n  step echo ${NOPE}\n", "a").err(),
            Some(Error::UnknownVariable("NOPE".to_string()))
        );
        assert_eq!(
            preview("task a : a\n", "a").err(),
            Some(Error::Cycle(vec!["a".to_string(), "a".to_string()]))
        );
    }

    #[test]
    fn dry_run_and_keep_going_are_independent_settings() {
        let options = RunOptions::new().keep_going(true).dry_run(true);
        assert!(options.is_keep_going() && options.is_dry_run());
        let options = RunOptions::new().dry_run(true).keep_going(false);
        assert!(!options.is_keep_going() && options.is_dry_run());
    }
}
