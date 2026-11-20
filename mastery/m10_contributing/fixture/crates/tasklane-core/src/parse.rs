//! The Lanefile parser.
//!
//! The format is line based:
//!
//! ```text
//! # A comment.
//! set OUT = build
//!
//! ## Shown by `tasklane list`.
//! task site : pages
//!   flags silent
//!   step concat ${OUT}/site.txt "pages/*.txt"
//! ```
//!
//! Lines that start in the first column are `set` and `task` directives. Indented
//! lines belong to the task above them and are `step` or `flags` directives.

use tasklane_plugin::TaskFlags;

use crate::error::Error;
use crate::model::{Lanefile, StepCall, Task};
use crate::text::{is_name, split_args, split_word};

/// Parses the text of a Lanefile.
///
/// # Errors
///
/// [`Error::Parse`] with the line number for the first problem found. A dependency on
/// a task that does not exist is reported at the line of the task that names it.
pub fn parse(source: &str) -> Result<Lanefile, Error> {
    let mut file = Lanefile::default();
    let mut doc: Option<String> = None;
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = raw.trim_end();
        let content = text.trim_start();
        if content.is_empty() {
            doc = None;
            continue;
        }
        if let Some(comment) = content.strip_prefix("##") {
            doc = Some(comment.trim().to_string());
            continue;
        }
        if content.starts_with('#') {
            continue;
        }
        let (word, rest) = split_word(content);
        if text.starts_with([' ', '\t']) {
            let Some(task) = file.tasks.last_mut() else {
                return Err(error(line, "indented line before the first task"));
            };
            parse_task_line(task, word, rest, line)?;
        } else {
            match word {
                "set" => file.vars.push(parse_set(rest, line)?),
                "task" => {
                    let task = parse_task(rest, line, doc.take())?;
                    if file.task(&task.name).is_some() {
                        return Err(error(
                            line,
                            format!("task `{}` is defined twice", task.name),
                        ));
                    }
                    file.tasks.push(task);
                }
                "step" | "flags" => {
                    return Err(error(
                        line,
                        format!("`{word}` must be indented under a task"),
                    ));
                }
                other => return Err(error(line, format!("unknown directive `{other}`"))),
            }
        }
        doc = None;
    }
    for task in &file.tasks {
        for dep in &task.deps {
            if file.task(dep).is_none() {
                return Err(error(
                    task.line,
                    format!("task `{}` depends on unknown task `{dep}`", task.name),
                ));
            }
        }
    }
    Ok(file)
}

fn error(line: usize, message: impl Into<String>) -> Error {
    Error::Parse {
        line,
        message: message.into(),
    }
}

/// `set NAME = VALUE`. The value is the rest of the line, trimmed, and may be empty.
fn parse_set(rest: &str, line: usize) -> Result<(String, String), Error> {
    let Some((name, value)) = rest.split_once('=') else {
        return Err(error(line, "expected `set NAME = VALUE`"));
    };
    let name = name.trim();
    if !is_name(name) {
        return Err(error(line, format!("invalid variable name `{name}`")));
    }
    Ok((name.to_string(), value.trim().to_string()))
}

/// `task NAME` or `task NAME : DEP DEP ...`.
fn parse_task(rest: &str, line: usize, doc: Option<String>) -> Result<Task, Error> {
    let (name, deps) = match rest.split_once(':') {
        Some((name, deps)) => (name.trim(), deps),
        None => (rest.trim(), ""),
    };
    if !is_name(name) {
        return Err(error(line, format!("invalid task name `{name}`")));
    }
    let mut names = Vec::new();
    for dep in deps.split_whitespace() {
        if !is_name(dep) {
            return Err(error(line, format!("invalid task name `{dep}`")));
        }
        names.push(dep.to_string());
    }
    Ok(Task {
        name: name.to_string(),
        doc,
        deps: names,
        flags: TaskFlags::empty(),
        steps: Vec::new(),
        line,
    })
}

/// An indented `step` or `flags` line.
fn parse_task_line(task: &mut Task, word: &str, rest: &str, line: usize) -> Result<(), Error> {
    match word {
        "step" => {
            let mut args = split_args(rest).map_err(|message| error(line, message))?;
            if args.is_empty() {
                return Err(error(line, "`step` needs a step name"));
            }
            let name = args.remove(0);
            if !is_name(&name) {
                return Err(error(line, format!("invalid step name `{name}`")));
            }
            task.steps.push(StepCall { name, args, line });
        }
        "flags" => {
            for name in rest.split_whitespace() {
                let Some(flag) = TaskFlags::from_keyword(name) else {
                    return Err(error(line, format!("unknown flag `{name}`")));
                };
                task.flags |= flag;
            }
        }
        other => {
            return Err(error(
                line,
                format!("unknown directive `{other}` in task `{}`", task.name),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_error(source: &str) -> (usize, String) {
        match parse(source) {
            Err(Error::Parse { line, message }) => (line, message),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn empty_file() {
        assert_eq!(parse("").unwrap(), Lanefile::default());
        assert_eq!(
            parse("\n# only a comment\n\n").unwrap(),
            Lanefile::default()
        );
    }

    #[test]
    fn sets_keep_file_order() {
        let file = parse("set A = 1\nset B=two words \nset EMPTY =\n").unwrap();
        assert_eq!(
            file.vars,
            [
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "two words".to_string()),
                ("EMPTY".to_string(), String::new()),
            ]
        );
    }

    #[test]
    fn task_with_dependencies_steps_and_flags() {
        let file = parse(
            "task a\ntask b\ntask c : a b\n  flags silent\n\tflags hidden silent\n  step write out.txt \"x y\"\n",
        )
        .unwrap();
        let c = file.task("c").unwrap();
        assert_eq!(c.deps, ["a", "b"]);
        assert_eq!(c.flags, TaskFlags::SILENT | TaskFlags::HIDDEN);
        assert_eq!(
            c.steps,
            [StepCall {
                name: "write".to_string(),
                args: vec!["out.txt".to_string(), "x y".to_string()],
                line: 6,
            }]
        );
        assert_eq!(c.line, 3);
    }

    #[test]
    fn doc_comment_belongs_to_the_next_task_only() {
        let file = parse("## Builds it.\ntask a\n## Lost.\n\ntask b\n# plain\ntask c\n").unwrap();
        assert_eq!(file.task("a").unwrap().doc.as_deref(), Some("Builds it."));
        assert_eq!(file.task("b").unwrap().doc, None);
        assert_eq!(file.task("c").unwrap().doc, None);
    }

    #[test]
    fn windows_line_endings() {
        let file = parse("set A = 1\r\ntask t\r\n  step echo hi\r\n").unwrap();
        assert_eq!(file.vars[0].1, "1");
        assert_eq!(file.task("t").unwrap().steps[0].args, ["hi"]);
    }

    #[test]
    fn step_outside_task_is_an_error() {
        assert_eq!(
            parse_error("step echo hi\n"),
            (1, "`step` must be indented under a task".to_string())
        );
        assert_eq!(
            parse_error("\n  step echo hi\n"),
            (2, "indented line before the first task".to_string())
        );
    }

    #[test]
    fn errors_name_the_line() {
        assert_eq!(parse_error("task a\ntask a\n").0, 2);
        assert_eq!(
            parse_error("task a\n  stpe echo\n"),
            (2, "unknown directive `stpe` in task `a`".to_string())
        );
        assert_eq!(
            parse_error("task a\n  flags quiet\n"),
            (2, "unknown flag `quiet`".to_string())
        );
        assert_eq!(
            parse_error("set = 3\n"),
            (1, "invalid variable name ``".to_string())
        );
        assert_eq!(
            parse_error("task a\n  step write \"open\n"),
            (2, "unterminated quote".to_string())
        );
        assert_eq!(
            parse_error("build all\n"),
            (1, "unknown directive `build`".to_string())
        );
    }

    #[test]
    fn unknown_dependency_is_reported_at_the_task() {
        assert_eq!(
            parse_error("task a\n\ntask b : a missing\n"),
            (3, "task `b` depends on unknown task `missing`".to_string())
        );
    }
}
