//! What happened during a run.

use std::collections::HashMap;

/// How one task ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Every step succeeded.
    Done,
    /// A step failed. The steps after it did not run.
    Failed,
    /// The task did not run.
    Skipped,
}

/// The result of one planned task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// The task name.
    pub name: String,
    /// How the task ended.
    pub status: TaskStatus,
    /// The error of the failed step, for `Failed`. May explain a `Skipped` task.
    pub message: Option<String>,
    /// The number of steps that completed.
    pub steps_run: usize,
}

/// The results of all planned tasks, in plan order.
#[derive(Debug, Clone, Default)]
pub struct RunReport {
    results: Vec<TaskResult>,
    failures: HashMap<String, String>,
    fatal: usize,
}

impl RunReport {
    /// Adds a result. `fatal` says whether a failure counts against
    /// [`RunReport::success`]; it is false for tasks with the `optional` flag.
    pub(crate) fn push(&mut self, result: TaskResult, fatal: bool) {
        if result.status == TaskStatus::Failed {
            let message = result.message.clone().unwrap_or_default();
            self.failures.insert(result.name.clone(), message);
            if fatal {
                self.fatal += 1;
            }
        }
        self.results.push(result);
    }

    /// One result per planned task, in plan order.
    pub fn results(&self) -> &[TaskResult] {
        &self.results
    }

    /// The status of the task called `name`, if it was planned.
    pub fn status_of(&self, name: &str) -> Option<TaskStatus> {
        self.results
            .iter()
            .find(|result| result.name == name)
            .map(|result| result.status)
    }

    /// The number of tasks with the given status.
    pub fn count(&self, status: TaskStatus) -> usize {
        self.results
            .iter()
            .filter(|result| result.status == status)
            .count()
    }

    /// Whether no task failed, apart from tasks with the `optional` flag.
    pub fn success(&self) -> bool {
        self.fatal == 0
    }

    /// One line for the user, for example
    /// `2 done, 1 failed, 1 skipped; failed: lint (fail: style)`.
    ///
    /// The failed tasks are listed in plan order.
    pub fn summary(&self) -> String {
        let mut line = format!(
            "{} done, {} failed, {} skipped",
            self.count(TaskStatus::Done),
            self.count(TaskStatus::Failed),
            self.count(TaskStatus::Skipped),
        );
        if !self.failures.is_empty() {
            let failed: Vec<String> = self
                .failures
                .iter()
                .map(|(name, message)| format!("{name} ({message})"))
                .collect();
            line.push_str("; failed: ");
            line.push_str(&failed.join(", "));
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(name: &str, status: TaskStatus) -> TaskResult {
        TaskResult {
            name: name.to_string(),
            status,
            message: (status == TaskStatus::Failed).then(|| "boom".to_string()),
            steps_run: 0,
        }
    }

    #[test]
    fn counts_and_success() {
        let mut report = RunReport::default();
        report.push(result("a", TaskStatus::Done), true);
        report.push(result("b", TaskStatus::Failed), false);
        report.push(result("c", TaskStatus::Skipped), true);
        assert_eq!(report.count(TaskStatus::Done), 1);
        assert_eq!(report.status_of("b"), Some(TaskStatus::Failed));
        assert_eq!(report.status_of("zzz"), None);
        assert!(report.success(), "an optional failure is not fatal");
        report.push(result("d", TaskStatus::Failed), true);
        assert!(!report.success());
    }

    #[test]
    fn summary_without_failures() {
        let mut report = RunReport::default();
        report.push(result("a", TaskStatus::Done), true);
        report.push(result("b", TaskStatus::Skipped), true);
        assert_eq!(report.summary(), "1 done, 0 failed, 1 skipped");
    }

    #[test]
    fn summary_with_one_failure() {
        let mut report = RunReport::default();
        report.push(result("a", TaskStatus::Failed), true);
        assert_eq!(
            report.summary(),
            "0 done, 1 failed, 0 skipped; failed: a (boom)"
        );
    }
}
