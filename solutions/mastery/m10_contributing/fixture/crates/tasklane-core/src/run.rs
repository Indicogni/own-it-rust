//! Running a plan.

use tasklane_plugin::{Step, StepContext, TaskFlags, Workspace};

use crate::error::Error;
use crate::graph::plan;
use crate::model::{Lanefile, Task};
use crate::registry::Registry;
use crate::report::{RunReport, TaskResult, TaskStatus};
use crate::vars::{Vars, expand};

/// Settings for a run. Build one with [`RunOptions::new`] and the setter methods:
///
/// ```
/// use tasklane_core::RunOptions;
///
/// let options = RunOptions::new().keep_going(true);
/// assert!(options.is_keep_going());
/// ```
///
/// New settings may be added in minor releases. That is why the fields are private
/// and the struct is `#[non_exhaustive]`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RunOptions {
    keep_going: bool,
    dry_run: bool,
}

impl RunOptions {
    /// The default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// After a task fails, still run the tasks that do not depend on it. Off by
    /// default: the first failure skips everything that has not run yet.
    pub fn keep_going(mut self, on: bool) -> Self {
        self.keep_going = on;
        self
    }

    /// Whether [`RunOptions::keep_going`] is on.
    pub fn is_keep_going(&self) -> bool {
        self.keep_going
    }

    /// Plan and check the run, but run no step. Off by default.
    ///
    /// A dry run finds the same mistakes as a real run: an unknown task, a cycle, an
    /// unknown step, an unknown variable. Then it reports every planned task as
    /// [`TaskStatus::Skipped`] with the message `dry run`. The files of the workspace
    /// do not change. For each step of a task that is not `silent`, the log gets the
    /// line `[TASK] would run: STEP ARGS...`.
    ///
    /// ```
    /// use tasklane_core::{Registry, RunOptions, Runner, Vars, Workspace, parse};
    ///
    /// let file = parse("task t\n  step write a.txt hello\n")?;
    /// let registry = Registry::with_builtins();
    /// let mut ws = Workspace::new();
    /// let report = Runner::new(&registry)
    ///     .with_options(RunOptions::new().dry_run(true))
    ///     .run(&file, "t", &Vars::new(), &mut ws)?;
    /// assert!(ws.is_empty());
    /// assert_eq!(ws.log_lines(), ["[t] would run: write a.txt hello"]);
    /// assert_eq!(report.summary(), "0 done, 0 failed, 1 skipped");
    /// # Ok::<(), tasklane_core::Error>(())
    /// ```
    pub fn dry_run(mut self, on: bool) -> Self {
        self.dry_run = on;
        self
    }

    /// Whether [`RunOptions::dry_run`] is on.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

/// Runs tasks with the steps of a [`Registry`].
pub struct Runner<'r> {
    registry: &'r Registry,
    options: RunOptions,
}

/// A step call that is ready to run: the implementation and the expanded arguments.
type Prepared<'r> = (&'r dyn Step, Vec<String>);

impl<'r> Runner<'r> {
    /// A runner with the default [`RunOptions`].
    pub fn new(registry: &'r Registry) -> Self {
        Runner {
            registry,
            options: RunOptions::default(),
        }
    }

    /// Replaces the options.
    pub fn with_options(mut self, options: RunOptions) -> Self {
        self.options = options;
        self
    }

    /// Runs `target` and everything it depends on, in the order given by
    /// [`plan`].
    ///
    /// Before the first step runs, every step name is looked up and every argument is
    /// expanded with `vars`. A mistake anywhere in the plan is therefore reported
    /// before the workspace changes.
    ///
    /// A failing step does not make `run` return an error. The task is recorded as
    /// [`TaskStatus::Failed`], and the tasks that depend on it as
    /// [`TaskStatus::Skipped`], unless the failed task has the `optional` flag.
    ///
    /// # Errors
    ///
    /// The errors of [`plan`] and [`expand`], and
    /// [`Error::UnknownStep`].
    pub fn run(
        &self,
        file: &Lanefile,
        target: &str,
        vars: &Vars,
        ws: &mut Workspace,
    ) -> Result<RunReport, Error> {
        let order = plan(file, target)?;
        let mut prepared: Vec<(&Task, Vec<Prepared<'r>>)> = Vec::with_capacity(order.len());
        for name in &order {
            let task = file.task(name).expect("a planned task exists");
            prepared.push((task, self.prepare(task, vars)?));
        }

        let mut report = RunReport::default();
        if self.options.dry_run {
            for (task, calls) in &prepared {
                if !task.flags.contains(TaskFlags::SILENT) {
                    for (step, args) in calls {
                        ws.log(step_line(task, "would run: ", *step, args));
                    }
                }
                // `Skipped` with a message, not a new status: `TaskStatus` is not
                // `#[non_exhaustive]`, so a new variant would break every user who
                // matches on it.
                report.push(
                    TaskResult {
                        name: task.name.clone(),
                        status: TaskStatus::Skipped,
                        message: Some("dry run".to_string()),
                        steps_run: 0,
                    },
                    false,
                );
            }
            return Ok(report);
        }
        // Tasks whose dependents must not run: failed (and not optional) or skipped.
        let mut blocked: Vec<&str> = Vec::new();
        let mut stopped = false;
        for (task, calls) in &prepared {
            if stopped || task.deps.iter().any(|dep| blocked.contains(&dep.as_str())) {
                blocked.push(&task.name);
                report.push(
                    TaskResult {
                        name: task.name.clone(),
                        status: TaskStatus::Skipped,
                        message: None,
                        steps_run: 0,
                    },
                    false,
                );
                continue;
            }
            let (steps_run, failure) = run_task(task, calls, ws);
            let optional = task.flags.contains(TaskFlags::OPTIONAL);
            let status = if failure.is_some() {
                if !optional {
                    blocked.push(&task.name);
                    stopped = !self.options.keep_going;
                }
                TaskStatus::Failed
            } else {
                TaskStatus::Done
            };
            report.push(
                TaskResult {
                    name: task.name.clone(),
                    status,
                    message: failure,
                    steps_run,
                },
                !optional,
            );
        }
        Ok(report)
    }

    fn prepare(&self, task: &Task, vars: &Vars) -> Result<Vec<Prepared<'r>>, Error> {
        let mut calls = Vec::with_capacity(task.steps.len());
        for call in &task.steps {
            let step = self
                .registry
                .lookup(&call.name)
                .ok_or_else(|| Error::UnknownStep {
                    task: task.name.clone(),
                    step: call.name.clone(),
                })?;
            let args = call
                .args
                .iter()
                .map(|arg| expand(arg, vars))
                .collect::<Result<Vec<String>, Error>>()?;
            calls.push((step.as_ref(), args));
        }
        Ok(calls)
    }
}

/// Runs the steps of one task until one fails. Returns the number of completed steps
/// and the failure message, if any.
fn run_task(task: &Task, calls: &[Prepared<'_>], ws: &mut Workspace) -> (usize, Option<String>) {
    let cx = StepContext::new(&task.name, task.flags);
    let mut steps_run = 0;
    for (step, args) in calls {
        if !task.flags.contains(TaskFlags::SILENT) {
            ws.log(step_line(task, "", *step, args));
        }
        if let Err(error) = step.run(args, ws, &cx) {
            return (steps_run, Some(error.to_string()));
        }
        steps_run += 1;
    }
    (steps_run, None)
}

/// The log line for one step: `[TASK] STEP ARGS...`, with `note` before the step.
fn step_line(task: &Task, note: &str, step: &dyn Step, args: &[String]) -> String {
    let mut line = format!("[{}] {note}{}", task.name, step.name());
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    line
}
