//! Which tasks run, and in which order.

use crate::error::Error;
use crate::model::Lanefile;

/// The tasks to run for `target`, in execution order.
///
/// A task comes after everything it depends on. Dependencies are visited in the order
/// they are written, so the plan is the same on every run. Each task appears once,
/// and `target` is last.
///
/// # Errors
///
/// [`Error::UnknownTask`] if `target` or a dependency does not exist, and
/// [`Error::Cycle`] with the full path if tasks depend on each other in a circle.
///
/// ```
/// use tasklane_core::{parse, plan};
///
/// let file = parse("task a\ntask b : a\ntask c : b a\n")?;
/// assert_eq!(plan(&file, "c")?, ["a", "b", "c"]);
/// # Ok::<(), tasklane_core::Error>(())
/// ```
pub fn plan(file: &Lanefile, target: &str) -> Result<Vec<String>, Error> {
    let mut order = Vec::new();
    let mut path = Vec::new();
    visit(file, target, &mut path, &mut order)?;
    Ok(order)
}

/// Depth-first walk. `path` holds the tasks between the target and `name`; meeting
/// one of them again means a cycle.
fn visit<'a>(
    file: &'a Lanefile,
    name: &'a str,
    path: &mut Vec<&'a str>,
    order: &mut Vec<String>,
) -> Result<(), Error> {
    // A task that is already planned was walked completely, together with everything
    // below it, and no cycle was found there. Walking it again cannot add anything.
    // Without this check a chain of diamonds is walked 2^depth times (#22).
    if order.iter().any(|done| done == name) {
        return Ok(());
    }
    if let Some(start) = path.iter().position(|seen| *seen == name) {
        let mut cycle: Vec<String> = path[start..].iter().map(|s| s.to_string()).collect();
        cycle.push(name.to_string());
        return Err(Error::Cycle(cycle));
    }
    let task = file
        .task(name)
        .ok_or_else(|| Error::UnknownTask(name.to_string()))?;
    path.push(name);
    for dep in &task.deps {
        visit(file, dep, path, order)?;
    }
    path.pop();
    order.push(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn single_task() {
        let file = parse("task only\n").unwrap();
        assert_eq!(plan(&file, "only").unwrap(), ["only"]);
    }

    #[test]
    fn dependencies_come_first_in_written_order() {
        let file = parse("task z\ntask a\ntask top : z a\n").unwrap();
        assert_eq!(plan(&file, "top").unwrap(), ["z", "a", "top"]);
    }

    #[test]
    fn shared_dependency_runs_once() {
        let file = parse("task base\ntask l : base\ntask r : base\ntask top : l r\n").unwrap();
        assert_eq!(plan(&file, "top").unwrap(), ["base", "l", "r", "top"]);
    }

    #[test]
    fn unknown_target() {
        let file = parse("task a\n").unwrap();
        assert_eq!(plan(&file, "b"), Err(Error::UnknownTask("b".to_string())));
    }

    #[test]
    fn cycle_reports_the_whole_path() {
        let file = parse("task a : b\ntask b : c\ntask c : a\ntask top : a\n").unwrap();
        assert_eq!(
            plan(&file, "top"),
            Err(Error::Cycle(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "a".to_string()
            ]))
        );
    }

    #[test]
    fn a_ladder_of_diamonds_is_planned_quickly() {
        // Regression test for #22: every level depends on both tasks of the level
        // below. The walk used to visit the bottom 2^60 times, which never ends, and
        // a test that never ends tells nobody what is wrong. So the planning runs on
        // a helper thread, and the test fails if no plan arrives in time.
        let mut source = String::from("task l0\ntask r0\n");
        for level in 1..=60 {
            let below = level - 1;
            source.push_str(&format!(
                "task l{level} : l{below} r{below}\ntask r{level} : l{below} r{below}\n"
            ));
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let file = parse(&source).unwrap();
            let _ = sender.send(plan(&file, "l60"));
        });
        let order = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("planning 121 tasks takes milliseconds, not seconds")
            .unwrap();
        assert_eq!(order.len(), 121);
        assert_eq!(order[..4], ["l0", "r0", "l1", "r1"]);
    }

    #[test]
    fn self_dependency_is_a_cycle() {
        let file = parse("task a : a\n").unwrap();
        assert_eq!(
            plan(&file, "a"),
            Err(Error::Cycle(vec!["a".to_string(), "a".to_string()]))
        );
    }
}
