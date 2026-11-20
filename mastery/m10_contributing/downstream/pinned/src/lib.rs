//! A downstream user of tasklane, written against version 0.4.0 exactly as released.
//!
//! Do not edit this crate. It stands for all the code you cannot see: the projects of
//! other people that depend on tasklane. It uses the public API in every way the API
//! allows. If a change to the fixture stops this crate from compiling, the change is
//! a breaking change, and 0.4.x must not contain one.

use std::collections::{BTreeSet, HashMap, HashSet};

use tasklane_core::{
    Error, Lanefile, Registry, RunOptions, RunReport, Runner, StepCall, Task, TaskResult,
    TaskStatus, Vars, expand, parse, plan,
};
use tasklane_plugin::{Step, StepContext, StepError, TaskFlags, Workspace};

/// A plugin step. It implements every method the trait had in 0.4.0, and no other.
pub struct Stamp;

impl Step for Stamp {
    fn name(&self) -> &str {
        "stamp"
    }

    fn usage(&self) -> &str {
        "stamp PATH"
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
        let old = ws.read(path).unwrap_or("").to_string();
        ws.write(path, format!("{old}[{}]", cx.task));
        Ok(())
    }
}

/// Runs `target` and describes every task in one word.
pub fn build(source: &str, target: &str) -> Result<(Workspace, Vec<String>), Error> {
    let file: Lanefile = parse(source)?;
    let mut registry = Registry::with_builtins();
    registry.register(Box::new(Stamp))?;

    // The exact signature of 0.4.0: `&String` in, `Option<&Box<dyn Step>>` out.
    let name = "stamp".to_string();
    let step: Option<&Box<dyn Step>> = registry.lookup(&name);
    assert!(step.is_some());

    let options: RunOptions = RunOptions::new().keep_going(true);
    let runner = Runner::new(&registry).with_options(options);
    let mut ws = Workspace::new();
    let vars = Vars::from_lanefile(&file);
    // `run` takes exactly these four arguments.
    let report: RunReport = runner.run(&file, target, &vars, &mut ws)?;

    let mut lines = Vec::new();
    for result in report.results() {
        lines.push(describe(result));
    }
    Ok((ws, lines))
}

/// `TaskStatus` is not `#[non_exhaustive]`, so a user may match it without a wildcard
/// arm. Many do.
pub fn describe(result: &TaskResult) -> String {
    let word = match result.status {
        TaskStatus::Done => "done",
        TaskStatus::Failed => "failed",
        TaskStatus::Skipped => "skipped",
    };
    format!("{} {word}", result.name)
}

/// `StepCall` and `Task` have only public fields and are not `#[non_exhaustive]`, so
/// a user may build them with a struct literal.
pub fn handmade_task() -> Task {
    Task {
        name: "handmade".to_string(),
        doc: None,
        deps: Vec::new(),
        flags: TaskFlags::SILENT | TaskFlags::OPTIONAL,
        steps: vec![StepCall {
            name: "echo".to_string(),
            args: vec!["made by hand".to_string()],
            line: 1,
        }],
        line: 1,
    }
}

/// `Error` is `#[non_exhaustive]`: a user must have a wildcard arm, so new variants
/// do not break them.
pub fn exit_code(error: &Error) -> u8 {
    match error {
        Error::Parse { line, .. } if *line > 0 => 3,
        Error::UnknownTask(_) | Error::Cycle(_) => 4,
        _ => 1,
    }
}

/// Everything `TaskFlags` could do in 0.4.0: copy, compare, order, hash, print,
/// combine, and convert to and from bits, with and without a check.
pub fn flag_facts() -> (usize, usize, u8, String) {
    let silent = TaskFlags::SILENT;
    let copy = silent;
    let both = silent | TaskFlags::HIDDEN;
    let mut hashed: HashSet<TaskFlags> = HashSet::new();
    hashed.insert(silent);
    hashed.insert(copy);
    hashed.insert(both);
    let mut ordered: BTreeSet<TaskFlags> = BTreeSet::new();
    ordered.insert(both);
    ordered.insert(TaskFlags::empty());
    ordered.insert(TaskFlags::all());
    let mut by_flag: HashMap<TaskFlags, &str> = HashMap::new();
    by_flag.insert(TaskFlags::OPTIONAL, "optional");
    assert!(silent == copy && silent < both);
    assert!(both.contains(TaskFlags::HIDDEN) && !both.is_empty());
    assert_eq!(TaskFlags::from_bits(both.bits()), Some(both));
    assert_eq!(TaskFlags::from_bits_truncate(0xff), TaskFlags::all());
    // SAFETY: 0b101 is `SILENT | HIDDEN`. Every bit belongs to a flag, which is all
    // that the method asked of its caller in 0.4.0.
    let unchecked = unsafe { TaskFlags::from_bits_unchecked(0b101) };
    assert_eq!(unchecked, both);
    assert_eq!(both.count(), 2);
    (
        hashed.len(),
        ordered.len(),
        both.bits(),
        tasklane_core::describe_flags(both),
    )
}

/// The free functions of the library.
pub fn order_and_text(source: &str, target: &str) -> Result<(Vec<String>, String), Error> {
    let file = parse(source)?;
    let order = plan(&file, target)?;
    let text = expand("${GREETING}, world", &Vars::from_lanefile(&file))?;
    Ok((order, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_with_a_plugin_step() {
        let source = "task a\n  step write out.txt x\n  step stamp out.txt\n\
                      task b : a\n  step fail no\n\
                      task c : b\n\
                      task all : a b c\n";
        let (ws, lines) = build(source, "all").unwrap();
        assert_eq!(ws.read("out.txt"), Some("x[a]"));
        assert_eq!(lines, ["a done", "b failed", "c skipped", "all skipped"]);
    }

    #[test]
    fn handmade_tasks_run() {
        let file = Lanefile {
            vars: Vec::new(),
            tasks: vec![handmade_task()],
        };
        let registry = Registry::with_builtins();
        let mut ws = Workspace::new();
        let report = Runner::new(&registry)
            .run(&file, "handmade", &Vars::new(), &mut ws)
            .unwrap();
        assert!(report.success());
        assert_eq!(ws.log_lines(), ["made by hand"]);
    }

    #[test]
    fn errors_map_to_exit_codes() {
        assert_eq!(exit_code(&parse("oops\n").unwrap_err()), 3);
        assert_eq!(exit_code(&Error::UnknownTask("x".to_string())), 4);
        assert_eq!(exit_code(&Error::UnknownVariable("x".to_string())), 1);
    }

    #[test]
    fn flags_behave_as_in_0_4_0() {
        assert_eq!(flag_facts(), (2, 3, 0b101, "silent, hidden".to_string()));
    }

    #[test]
    fn plan_and_expand() {
        let (order, text) =
            order_and_text("set GREETING = hello\ntask a\ntask b : a\n", "b").unwrap();
        assert_eq!(order, ["a", "b"]);
        assert_eq!(text, "hello, world");
    }
}
