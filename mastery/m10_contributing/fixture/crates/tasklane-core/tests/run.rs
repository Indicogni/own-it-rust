//! The runner from the outside: Lanefile text in, workspace and report out.

use tasklane_core::{
    Error, Registry, RunOptions, RunReport, Runner, Step, StepContext, StepError, TaskStatus, Vars,
    Workspace, parse,
};

fn run_with(source: &str, target: &str, options: RunOptions) -> (Workspace, RunReport) {
    let file = parse(source).expect("the test Lanefile parses");
    let registry = Registry::with_builtins();
    let mut ws = Workspace::new();
    let report = Runner::new(&registry)
        .with_options(options)
        .run(&file, target, &Vars::from_lanefile(&file), &mut ws)
        .expect("the run starts");
    (ws, report)
}

fn run(source: &str, target: &str) -> (Workspace, RunReport) {
    run_with(source, target, RunOptions::new())
}

fn statuses(report: &RunReport) -> Vec<(&str, TaskStatus)> {
    report
        .results()
        .iter()
        .map(|result| (result.name.as_str(), result.status))
        .collect()
}

#[test]
fn steps_run_in_order_and_are_logged() {
    let (ws, report) = run(
        "set WHO = you\ntask greet\n  step write hi.txt hello ${WHO}\n  step upper hi.txt\n",
        "greet",
    );
    assert!(report.success());
    assert_eq!(ws.read("hi.txt"), Some("HELLO YOU"));
    assert_eq!(
        ws.log_lines(),
        ["[greet] write hi.txt hello you", "[greet] upper hi.txt"]
    );
    assert_eq!(report.results()[0].steps_run, 2);
}

#[test]
fn silent_tasks_are_not_logged_but_their_output_is() {
    let (ws, _) = run("task t\n  flags silent\n  step echo still here\n", "t");
    assert_eq!(ws.log_lines(), ["still here"]);
}

#[test]
fn a_failure_stops_the_task_and_skips_the_rest() {
    let source = "task a\n  step write a.txt 1\n  step fail broken\n  step write never.txt 1\n\
                  task b\n  step write b.txt 1\n\
                  task top : a b\n";
    let (ws, report) = run(source, "top");
    assert_eq!(
        statuses(&report),
        [
            ("a", TaskStatus::Failed),
            ("b", TaskStatus::Skipped),
            ("top", TaskStatus::Skipped)
        ]
    );
    assert!(!report.success());
    assert_eq!(report.results()[0].steps_run, 1);
    assert_eq!(report.results()[0].message.as_deref(), Some("fail: broken"));
    assert!(ws.contains("a.txt"));
    assert!(!ws.contains("never.txt"));
    assert!(!ws.contains("b.txt"));
}

#[test]
fn keep_going_runs_independent_tasks() {
    let source = "task a\n  step fail broken\n\
                  task b\n  step write b.txt 1\n\
                  task c : a\n  step write c.txt 1\n\
                  task top : a b c\n";
    let (ws, report) = run_with(source, "top", RunOptions::new().keep_going(true));
    assert_eq!(
        statuses(&report),
        [
            ("a", TaskStatus::Failed),
            ("b", TaskStatus::Done),
            ("c", TaskStatus::Skipped),
            ("top", TaskStatus::Skipped)
        ]
    );
    assert!(ws.contains("b.txt"));
    assert!(!ws.contains("c.txt"));
}

#[test]
fn an_optional_failure_blocks_nothing() {
    let source = "task lint\n  flags optional\n  step fail style\n\
                  task build : lint\n  step write out.txt ok\n";
    let (ws, report) = run(source, "build");
    assert_eq!(
        statuses(&report),
        [("lint", TaskStatus::Failed), ("build", TaskStatus::Done)]
    );
    assert!(report.success());
    assert!(ws.contains("out.txt"));
}

#[test]
fn mistakes_are_found_before_anything_runs() {
    let registry = Registry::with_builtins();
    let mut ws = Workspace::new();
    let runner = Runner::new(&registry);

    let file = parse("task a\n  step write a.txt 1\ntask b : a\n  step polish a.txt\n").unwrap();
    assert_eq!(
        runner.run(&file, "b", &Vars::new(), &mut ws).err(),
        Some(Error::UnknownStep {
            task: "b".to_string(),
            step: "polish".to_string()
        })
    );
    let file = parse("task a\n  step write a.txt 1\ntask b : a\n  step echo ${NOPE}\n").unwrap();
    assert_eq!(
        runner.run(&file, "b", &Vars::new(), &mut ws).err(),
        Some(Error::UnknownVariable("NOPE".to_string()))
    );
    assert!(ws.is_empty(), "no step may run when the plan has a mistake");
    assert!(ws.log_lines().is_empty());
}

/// A plugin step, as a user of the library would write it.
struct Shout;

impl Step for Shout {
    fn name(&self) -> &str {
        "shout"
    }
    fn usage(&self) -> &str {
        "shout PATH"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        cx: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [path] = args else {
            return Err(StepError::usage(self));
        };
        let text = ws
            .read(path)
            .ok_or_else(|| StepError::new(self.name(), format!("no such file `{path}`")))?;
        let loud = format!("{}! ({})", text.to_uppercase(), cx.task);
        ws.write(path, loud);
        Ok(())
    }
}

#[test]
fn plugin_steps_run_like_builtin_steps() {
    let mut registry = Registry::with_builtins();
    registry.register(Box::new(Shout)).unwrap();
    let file = parse("task t\n  step write a hey\n  step shout a\n").unwrap();
    let mut ws = Workspace::new();
    let report = Runner::new(&registry)
        .run(&file, "t", &Vars::new(), &mut ws)
        .unwrap();
    assert!(report.success());
    assert_eq!(ws.read("a"), Some("HEY! (t)"));
}
