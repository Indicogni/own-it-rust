//! Builds the throwaway git repository of lesson 03. You do not need to edit this
//! file, and reading the commit list here instead of bisecting teaches you nothing.
//!
//! The repository holds an early, single-crate tasklane with a scripted history.
//! It is created in a directory outside the course repository. Every git command runs
//! with `-C <that directory>` and with the user's and the system's git configuration
//! switched off, so nothing here can touch the course repository or your settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The exit code that tells `git bisect run` "this commit cannot be tested".
pub const SKIP_EXIT_CODE: i32 = 125;

/// One change to the files of the scripted project.
pub enum Edit {
    /// Creates or replaces the file `.0` with the text `.1`.
    Write(&'static str, &'static str),
    /// In the file `.0`, replaces the only occurrence of `.1` with `.2`.
    Replace(&'static str, &'static str, &'static str),
    /// Adds the text `.1` at the end of the file `.0`.
    Append(&'static str, &'static str),
    /// Moves a part of a file into a new file.
    Move {
        /// The file to cut from.
        from: &'static str,
        /// The cut starts where this text starts.
        start: &'static str,
        /// The cut ends before this text, or at the end of the file.
        end: Option<&'static str>,
        /// The file to create.
        to: &'static str,
        /// The first lines of the new file.
        header: &'static str,
    },
}

/// One scripted commit.
pub struct Commit {
    /// The first line of the commit message.
    pub subject: &'static str,
    /// The rest of the commit message; may be empty.
    pub body: &'static str,
    /// A tag to put on the commit.
    pub tag: Option<&'static str>,
    /// Who wrote it.
    pub author: &'static str,
    /// The changes.
    pub edits: &'static [Edit],
}

const ADA: &str = "Ada Okafor <ada@tasklane.example>";
const JUN: &str = "Jun Tanaka <jun@tasklane.example>";
const MIRA: &str = "Mira Novak <mira@tasklane.example>";

use Edit::{Append, Replace, Write};

/// The whole history, oldest commit first.
pub const COMMITS: &[Commit] = &[
    Commit {
        subject: "Initial commit",
        body: "",
        tag: None,
        author: ADA,
        edits: &[
            Write(
                "Cargo.toml",
                "[package]\nname = \"tasklane\"\nversion = \"0.1.0-dev\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
            ),
            Write(".gitignore", "target/\nCargo.lock\n"),
            Write(
                "README.md",
                "# tasklane\n\nRuns named tasks in dependency order.\n",
            ),
            Write(
                "src/lib.rs",
                r###"//! tasklane: named tasks in dependency order.

/// One task of a Lanefile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub deps: Vec<String>,
}

/// A parsed Lanefile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lanefile {
    pub tasks: Vec<Task>,
}

/// Parses the text of a Lanefile.
pub fn parse(source: &str) -> Result<Lanefile, String> {
    let mut file = Lanefile::default();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("task ") else {
            return Err(format!("unknown line `{line}`"));
        };
        let (name, deps) = match rest.split_once(':') {
            Some((name, deps)) => (name.trim(), deps),
            None => (rest.trim(), ""),
        };
        file.tasks.push(Task {
            name: name.to_string(),
            deps: deps.split_whitespace().map(str::to_string).collect(),
        });
    }
    Ok(file)
}
"###,
            ),
        ],
    },
    Commit {
        subject: "parse: skip blank lines and comments",
        body: "A line that starts with `#` is a comment.",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "src/lib.rs",
            "    for line in source.lines() {\n",
            "    for line in source.lines() {\n        if line.trim().is_empty() || line.trim_start().starts_with('#') {\n            continue;\n        }\n",
        )],
    },
    Commit {
        subject: "Add steps to tasks",
        body: "An indented `step NAME ARGS...` line adds a step to the task above it.\nArguments are separated by white space.",
        tag: None,
        author: ADA,
        edits: &[
            Replace(
                "src/lib.rs",
                "    pub deps: Vec<String>,\n}\n",
                r###"    pub deps: Vec<String>,
    pub steps: Vec<StepCall>,
}

/// One `step` line of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepCall {
    pub name: String,
    pub args: Vec<String>,
}
"###,
            ),
            Replace(
                "src/lib.rs",
                "        let Some(rest) = line.strip_prefix(\"task \") else {\n",
                r###"        if line.starts_with(' ') {
            let Some(task) = file.tasks.last_mut() else {
                return Err(format!("step before the first task: `{line}`"));
            };
            let mut words = line.split_whitespace();
            if words.next() != Some("step") {
                return Err(format!("unknown line `{line}`"));
            }
            let Some(name) = words.next() else {
                return Err(format!("step without a name: `{line}`"));
            };
            task.steps.push(StepCall {
                name: name.to_string(),
                args: words.map(str::to_string).collect(),
            });
            continue;
        }
        let Some(rest) = line.strip_prefix("task ") else {
"###,
            ),
            Replace(
                "src/lib.rs",
                "            deps: deps.split_whitespace().map(str::to_string).collect(),\n",
                "            deps: deps.split_whitespace().map(str::to_string).collect(),\n            steps: Vec::new(),\n",
            ),
        ],
    },
    Commit {
        subject: "tests: cover task parsing",
        body: "",
        tag: None,
        author: JUN,
        edits: &[Write(
            "tests/parse.rs",
            r###"use tasklane::{StepCall, parse};

#[test]
fn tasks_and_dependencies() {
    let file = parse("# demo\ntask a\n\ntask b : a\n").unwrap();
    assert_eq!(file.tasks.len(), 2);
    assert_eq!(file.tasks[1].deps, ["a"]);
}

#[test]
fn steps_belong_to_the_task_above() {
    let file = parse("task a\n  step write out.txt hello\n").unwrap();
    assert_eq!(
        file.tasks[0].steps,
        [StepCall {
            name: "write".to_string(),
            args: vec!["out.txt".to_string(), "hello".to_string()],
        }]
    );
}

#[test]
fn unknown_lines_are_errors() {
    assert!(parse("build it\n").is_err());
    assert!(parse("  step x\n").is_err());
}
"###,
        )],
    },
    Commit {
        subject: "Add plan() with dependency order",
        body: "plan() returns the tasks to run for a target. A task comes after\neverything it depends on, and each task appears once.",
        tag: None,
        author: ADA,
        edits: &[
            Append(
                "src/lib.rs",
                r###"
/// The tasks to run for `target`, dependencies first.
pub fn plan(file: &Lanefile, target: &str) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    visit(file, target, &mut order)?;
    Ok(order)
}

fn visit(file: &Lanefile, name: &str, order: &mut Vec<String>) -> Result<(), String> {
    if order.iter().any(|done| done == name) {
        return Ok(());
    }
    let Some(task) = file.tasks.iter().find(|task| task.name == name) else {
        return Err(format!("unknown task `{name}`"));
    };
    for dep in &task.deps {
        visit(file, dep, order)?;
    }
    order.push(name.to_string());
    Ok(())
}
"###,
            ),
            Write(
                "tests/plan.rs",
                r###"use tasklane::{parse, plan};

#[test]
fn dependencies_run_first() {
    let file = parse("task a\ntask b : a\ntask c : b a\n").unwrap();
    assert_eq!(plan(&file, "c").unwrap(), ["a", "b", "c"]);
}
"###,
            ),
        ],
    },
    Commit {
        subject: "plan: detect cycles",
        body: "Two tasks that depend on each other made plan() recurse until the\nstack overflowed. Keep the path from the target and stop when a\ntask is met again.",
        tag: None,
        author: ADA,
        edits: &[
            Replace(
                "src/lib.rs",
                "    visit(file, target, &mut order)?;\n",
                "    visit(file, target, &mut Vec::new(), &mut order)?;\n",
            ),
            Replace(
                "src/lib.rs",
                "fn visit(file: &Lanefile, name: &str, order: &mut Vec<String>) -> Result<(), String> {\n    if order.iter().any(|done| done == name) {\n        return Ok(());\n    }\n",
                r###"fn visit(
    file: &Lanefile,
    name: &str,
    path: &mut Vec<String>,
    order: &mut Vec<String>,
) -> Result<(), String> {
    if order.iter().any(|done| done == name) {
        return Ok(());
    }
    if path.iter().any(|seen| seen == name) {
        return Err(format!("dependency cycle through `{name}`"));
    }
"###,
            ),
            Replace(
                "src/lib.rs",
                "    for dep in &task.deps {\n        visit(file, dep, order)?;\n    }\n",
                "    path.push(name.to_string());\n    for dep in &task.deps {\n        visit(file, dep, path, order)?;\n    }\n    path.pop();\n",
            ),
            Append(
                "tests/plan.rs",
                r###"
#[test]
fn cycles_are_errors() {
    let file = parse("task a : b\ntask b : a\n").unwrap();
    assert!(plan(&file, "a").unwrap_err().contains("cycle"));
}
"###,
            ),
        ],
    },
    Commit {
        subject: "README: describe the Lanefile format",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Append(
            "README.md",
            "\n## Lanefile\n\n```text\n# A comment.\ntask clean\n  step remove build\n\ntask build : clean\n  step write build/out.txt hello\n```\n\nA `task` line names a task and, after a colon, the tasks it depends on.\nIndented `step` lines belong to the task above them.\n",
        )],
    },
    Commit {
        subject: "Release 0.1.0",
        body: "",
        tag: Some("v0.1.0"),
        author: ADA,
        edits: &[
            Replace(
                "Cargo.toml",
                "version = \"0.1.0-dev\"",
                "version = \"0.1.0\"",
            ),
            Write(
                "CHANGELOG.md",
                "# Changelog\n\n## 0.1.0\n\n- Tasks, dependencies and steps.\n- `plan()` with cycle detection.\n",
            ),
        ],
    },
    Commit {
        subject: "Add the `set` directive",
        body: "`set NAME = VALUE` stores a variable in the Lanefile.",
        tag: None,
        author: JUN,
        edits: &[
            Replace(
                "src/lib.rs",
                "pub struct Lanefile {\n    pub tasks: Vec<Task>,\n}\n",
                "pub struct Lanefile {\n    pub vars: Vec<(String, String)>,\n    pub tasks: Vec<Task>,\n}\n",
            ),
            Replace(
                "src/lib.rs",
                "        let Some(rest) = line.strip_prefix(\"task \") else {\n",
                r###"        if let Some(rest) = line.strip_prefix("set ") {
            let Some((name, value)) = rest.split_once('=') else {
                return Err(format!("expected `set NAME = VALUE`: `{line}`"));
            };
            file.vars
                .push((name.trim().to_string(), value.trim().to_string()));
            continue;
        }
        let Some(rest) = line.strip_prefix("task ") else {
"###,
            ),
        ],
    },
    Commit {
        subject: "Add expand() for ${NAME}",
        body: "expand() replaces `${NAME}` with the value of the variable. The last\n`set` of a name wins.",
        tag: None,
        author: JUN,
        edits: &[Append(
            "src/lib.rs",
            r###"
/// Replaces every `${NAME}` in `template` with the value of `NAME`.
pub fn expand(template: &str, vars: &[(String, String)]) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return Ok(out);
        };
        let name = &after[..end];
        let Some((_, value)) = vars.iter().rev().find(|(known, _)| known == name) else {
            return Err(format!("unknown variable `{name}`"));
        };
        out.push_str(value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
"###,
        )],
    },
    Commit {
        subject: "tests: variable expansion",
        body: "",
        tag: None,
        author: JUN,
        edits: &[Write(
            "tests/vars.rs",
            r###"use tasklane::{expand, parse};

#[test]
fn set_then_expand() {
    let file = parse("set OUT = build\nset OUT = dist\n").unwrap();
    assert_eq!(expand("${OUT}/a.txt", &file.vars).unwrap(), "dist/a.txt");
    assert!(expand("${NOPE}", &file.vars).is_err());
}
"###,
        )],
    },
    Commit {
        subject: "expand: `$$` escapes a dollar sign",
        body: "There was no way to write a literal `${`. Now `$$` gives one `$`, so\n`$${NAME}` gives the text `${NAME}`.",
        tag: None,
        author: JUN,
        edits: &[
            Replace(
                "src/lib.rs",
                "    while let Some(start) = rest.find(\"${\") {\n        out.push_str(&rest[..start]);\n",
                r###"    while let Some(start) = rest.find('$') {
        out.push_str(&rest[..start]);
        if rest[start..].starts_with("$$") {
            out.push('$');
            rest = &rest[start + 2..];
            continue;
        }
        if !rest[start..].starts_with("${") {
            out.push('$');
            rest = &rest[start + 1..];
            continue;
        }
"###,
            ),
            Append(
                "tests/vars.rs",
                r###"
#[test]
fn double_dollar_is_a_dollar() {
    assert_eq!(expand("$${OUT} costs $5", &[]).unwrap(), "${OUT} costs $5");
}
"###,
            ),
        ],
    },
    Commit {
        subject: "parse: reject duplicate task names",
        body: "The second definition used to shadow the first one silently.",
        tag: None,
        author: ADA,
        edits: &[
            Replace(
                "src/lib.rs",
                "        file.tasks.push(Task {\n",
                "        if file.tasks.iter().any(|task| task.name == name) {\n            return Err(format!(\"task `{name}` is defined twice\"));\n        }\n        file.tasks.push(Task {\n",
            ),
            Append(
                "tests/parse.rs",
                r###"
#[test]
fn duplicate_tasks_are_errors() {
    assert!(parse("task a\ntask a\n").unwrap_err().contains("twice"));
}
"###,
            ),
        ],
    },
    Commit {
        subject: "README: document variables",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Append(
            "README.md",
            "\n## Variables\n\n```text\nset OUT = build\n\ntask build\n  step write ${OUT}/out.txt hello\n```\n\n`${NAME}` in a step argument is replaced by the value. Write `$$` for a\nliteral dollar sign.\n",
        )],
    },
    Commit {
        subject: "parse: reject empty task names",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "src/lib.rs",
            "        if file.tasks.iter().any(|task| task.name == name) {\n",
            "        if name.is_empty() {\n            return Err(format!(\"task without a name: `{line}`\"));\n        }\n        if file.tasks.iter().any(|task| task.name == name) {\n",
        )],
    },
    Commit {
        subject: "Add a check script",
        body: "Run ./check.sh before you open a pull request.",
        tag: None,
        author: MIRA,
        edits: &[Write(
            "check.sh",
            "#!/bin/sh\n# Everything a pull request must pass.\nset -e\ncargo build\ncargo test\n",
        )],
    },
    Commit {
        subject: "plan: say which task has the unknown dependency",
        body: "`unknown task `x`` did not help in a long Lanefile.",
        tag: None,
        author: JUN,
        edits: &[
            Replace(
                "src/lib.rs",
                "        return Err(format!(\"unknown task `{name}`\"));\n",
                "        return Err(match path.last() {\n            Some(parent) => format!(\"task `{parent}` depends on unknown task `{name}`\"),\n            None => format!(\"unknown task `{name}`\"),\n        });\n",
            ),
            Append(
                "tests/plan.rs",
                r###"
#[test]
fn unknown_dependencies_name_the_task() {
    let file = parse("task a : ghost\n").unwrap();
    assert_eq!(
        plan(&file, "a").unwrap_err(),
        "task `a` depends on unknown task `ghost`"
    );
    assert_eq!(plan(&file, "b").unwrap_err(), "unknown task `b`");
}
"###,
            ),
        ],
    },
    Commit {
        subject: "Release 0.1.1",
        body: "",
        tag: Some("v0.1.1"),
        author: ADA,
        edits: &[
            Replace("Cargo.toml", "version = \"0.1.0\"", "version = \"0.1.1\""),
            Replace(
                "CHANGELOG.md",
                "# Changelog\n",
                "# Changelog\n\n## 0.1.1\n\n- `set` and `${NAME}`; `$$` for a literal dollar sign.\n- Duplicate and empty task names are errors.\n- Better message for an unknown dependency.\n",
            ),
        ],
    },
    Commit {
        subject: "parse: accept tabs for indentation",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[
            Replace(
                "src/lib.rs",
                "        if line.starts_with(' ') {\n",
                "        if line.starts_with([' ', '\\t']) {\n",
            ),
            Append(
                "tests/parse.rs",
                r###"
#[test]
fn tabs_indent_too() {
    let file = parse("task a\n\tstep echo hi\n").unwrap();
    assert_eq!(file.tasks[0].steps[0].name, "echo");
}
"###,
            ),
        ],
    },
    Commit {
        subject: "docs: add a crate-level example",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Replace(
            "src/lib.rs",
            "//! tasklane: named tasks in dependency order.\n",
            "//! tasklane: named tasks in dependency order.\n//!\n//! ```\n//! let file = tasklane::parse(\"task a\\ntask b : a\\n\").unwrap();\n//! assert_eq!(tasklane::plan(&file, \"b\").unwrap(), [\"a\", \"b\"]);\n//! ```\n",
        )],
    },
    Commit {
        subject: "tests: a shared dependency runs once",
        body: "",
        tag: None,
        author: JUN,
        edits: &[Append(
            "tests/plan.rs",
            r###"
#[test]
fn shared_dependency_runs_once() {
    let file = parse("task base\ntask l : base\ntask r : base\ntask top : l r\n").unwrap();
    assert_eq!(plan(&file, "top").unwrap(), ["base", "l", "r", "top"]);
}
"###,
        )],
    },
    Commit {
        subject: "Cargo.toml: add description and license",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "Cargo.toml",
            "edition = \"2024\"\n",
            "edition = \"2024\"\ndescription = \"Runs named tasks in dependency order.\"\nlicense = \"MIT\"\n",
        )],
    },
    Commit {
        subject: "refactor: move the planner into its own module (1/3)",
        body: "lib.rs is getting long. Start splitting it: plan() and visit() move to\ngraph.rs unchanged.",
        tag: None,
        author: ADA,
        edits: &[
            Edit::Move {
                from: "src/lib.rs",
                start: "/// The tasks to run for `target`, dependencies first.\n",
                end: Some("/// Replaces every `${NAME}` in `template`"),
                to: "src/graph.rs",
                header: "//! Dependency order.\n",
            },
            Replace(
                "src/lib.rs",
                "/// One task of a Lanefile.\n",
                "mod graph;\n\npub use graph::plan;\n\n/// One task of a Lanefile.\n",
            ),
        ],
    },
    Commit {
        subject: "refactor: move expand() into its own module (2/3)",
        body: "",
        tag: None,
        author: ADA,
        edits: &[
            Edit::Move {
                from: "src/lib.rs",
                start: "/// Replaces every `${NAME}` in `template`",
                end: None,
                to: "src/vars.rs",
                header: "//! Variables.\n",
            },
            Replace(
                "src/lib.rs",
                "mod graph;\n\npub use graph::plan;\n",
                "mod graph;\nmod vars;\n\npub use graph::plan;\npub use vars::expand;\n",
            ),
        ],
    },
    Commit {
        subject: "refactor: fix the imports after the module split (3/3)",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "src/graph.rs",
            "//! Dependency order.\n",
            "//! Dependency order.\n\nuse crate::Lanefile;\n",
        )],
    },
    Commit {
        subject: "parse: quoted arguments keep spaces",
        body: "`step write a.txt \"two words\"` now passes `two words` as one argument.\nThe quotes themselves are removed.",
        tag: None,
        author: JUN,
        edits: &[
            Replace(
                "src/lib.rs",
                r###"            let mut words = line.split_whitespace();
            if words.next() != Some("step") {
                return Err(format!("unknown line `{line}`"));
            }
            let Some(name) = words.next() else {
                return Err(format!("step without a name: `{line}`"));
            };
            task.steps.push(StepCall {
                name: name.to_string(),
                args: words.map(str::to_string).collect(),
            });
"###,
                r###"            let mut words = split_words(line)?.into_iter();
            if words.next().as_deref() != Some("step") {
                return Err(format!("unknown line `{line}`"));
            }
            let Some(name) = words.next() else {
                return Err(format!("step without a name: `{line}`"));
            };
            task.steps.push(StepCall {
                name,
                args: words.collect(),
            });
"###,
            ),
            Append(
                "src/lib.rs",
                r###"
/// Splits a line into words. Double quotes keep spaces inside one word.
fn split_words(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut in_quotes = false;
    for c in line.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            in_word = true;
        } else if c.is_whitespace() && !in_quotes {
            if in_word {
                words.push(std::mem::take(&mut current));
                in_word = false;
            }
        } else {
            current.push(c);
            in_word = true;
        }
    }
    if in_quotes {
        return Err(format!("unterminated quote: `{line}`"));
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}
"###,
            ),
        ],
    },
    Commit {
        subject: "tests: quoted arguments",
        body: "",
        tag: None,
        author: JUN,
        edits: &[Append(
            "tests/parse.rs",
            r###"
#[test]
fn quotes_keep_spaces() {
    let file = parse("task a\n  step write a.txt \"two words\" x\n").unwrap();
    assert_eq!(file.tasks[0].steps[0].args, ["a.txt", "two words", "x"]);
    assert!(parse("task a\n  step write \"open\n").is_err());
}
"###,
        )],
    },
    Commit {
        subject: "Add the `flags` directive",
        body: "An indented `flags silent` line marks a task as silent. More flags can\nfollow later.",
        tag: None,
        author: MIRA,
        edits: &[
            Replace(
                "src/lib.rs",
                "    pub steps: Vec<StepCall>,\n}\n",
                "    pub steps: Vec<StepCall>,\n    pub silent: bool,\n}\n",
            ),
            Replace(
                "src/lib.rs",
                "            steps: Vec::new(),\n",
                "            steps: Vec::new(),\n            silent: false,\n",
            ),
            Replace(
                "src/lib.rs",
                r###"            if words.next().as_deref() != Some("step") {
                return Err(format!("unknown line `{line}`"));
            }
"###,
                r###"            let directive = words.next();
            if directive.as_deref() == Some("flags") {
                for flag in words {
                    if flag != "silent" {
                        return Err(format!("unknown flag `{flag}`"));
                    }
                    task.silent = true;
                }
                continue;
            }
            if directive.as_deref() != Some("step") {
                return Err(format!("unknown line `{line}`"));
            }
"###,
            ),
            Append(
                "tests/parse.rs",
                r###"
#[test]
fn flags_mark_a_task() {
    let file = parse("task a\n  flags silent\ntask b\n").unwrap();
    assert!(file.tasks[0].silent);
    assert!(!file.tasks[1].silent);
    assert!(parse("task a\n  flags loud\n").is_err());
}
"###,
            ),
        ],
    },
    Commit {
        subject: "README: document flags and quoting",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Append(
            "README.md",
            "\n## Flags and quotes\n\n```text\ntask notes\n  flags silent\n  step write notes.txt \"two words\"\n```\n\n`flags silent` hides the task's steps from the log. Double quotes keep\nspaces inside one argument.\n",
        )],
    },
    Commit {
        subject: "expand: an unterminated `${` is an error",
        body: "`${NAME` without the closing brace was copied to the output. That hid\ntypos. It is an error now.",
        tag: None,
        author: JUN,
        edits: &[
            Replace(
                "src/vars.rs",
                "            out.push_str(&rest[start..]);\n            return Ok(out);\n",
                "            return Err(format!(\"missing `}}` in `{template}`\"));\n",
            ),
            Append(
                "tests/vars.rs",
                r###"
#[test]
fn unterminated_variable_is_an_error() {
    assert!(expand("${OUT", &[]).unwrap_err().contains("missing"));
}
"###,
            ),
        ],
    },
    Commit {
        subject: "parse: allow a comment at the end of a line",
        body: "A line may now end with a comment:\n\n    task site : pages   # the default target\n\nUntil now a comment needed a line of its own.",
        tag: None,
        author: MIRA,
        edits: &[Replace(
            "src/lib.rs",
            "        if line.trim().is_empty() || line.trim_start().starts_with('#') {\n            continue;\n        }\n",
            "        // Everything after a `#` is a comment.\n        let line = line.split('#').next().unwrap_or(\"\");\n        if line.trim().is_empty() {\n            continue;\n        }\n",
        )],
    },
    Commit {
        subject: "tests: comments at the end of a line",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Append(
            "tests/parse.rs",
            r###"
#[test]
fn trailing_comments() {
    let file = parse("task a # first\n  step echo hi # greet\nset X = 1 # one\n").unwrap();
    assert_eq!(file.tasks[0].name, "a");
    assert_eq!(file.tasks[0].steps[0].args, ["hi"]);
    assert_eq!(file.vars[0].1, "1");
}
"###,
        )],
    },
    Commit {
        subject: "CHANGELOG: list the changes since 0.1.1",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "CHANGELOG.md",
            "# Changelog\n",
            "# Changelog\n\n## Unreleased\n\n- Quoted arguments, `flags silent`, comments at the end of a line.\n- An unterminated `${` is an error.\n- Tabs may indent.\n",
        )],
    },
    Commit {
        subject: "plan: show the whole cycle in the error",
        body: "`dependency cycle through `a`` named one task of the cycle. Print the\npath instead: `dependency cycle: a -> b -> a`.",
        tag: None,
        author: JUN,
        edits: &[Replace(
            "src/graph.rs",
            "    if path.iter().any(|seen| seen == name) {\n        return Err(format!(\"dependency cycle through `{name}`\"));\n    }\n",
            "    if let Some(start) = path.iter().position(|seen| seen == name) {\n        let mut cycle = path[start..].to_vec();\n        cycle.push(name.to_string());\n        return Err(format!(\"dependency cycle: {}\", cycle.join(\" -> \")));\n    }\n",
        )],
    },
    Commit {
        subject: "Add Lanefile::task()",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "src/lib.rs",
            "/// Parses the text of a Lanefile.\n",
            "impl Lanefile {\n    /// The task called `name`.\n    pub fn task(&self, name: &str) -> Option<&Task> {\n        self.tasks.iter().find(|task| task.name == name)\n    }\n}\n\n/// Parses the text of a Lanefile.\n",
        )],
    },
    Commit {
        subject: "plan: use Lanefile::task()",
        body: "",
        tag: None,
        author: ADA,
        edits: &[Replace(
            "src/graph.rs",
            "    let Some(task) = file.tasks.iter().find(|task| task.name == name) else {\n",
            "    let Some(task) = file.task(name) else {\n",
        )],
    },
    Commit {
        subject: "tests: the cycle message",
        body: "",
        tag: None,
        author: JUN,
        edits: &[Append(
            "tests/plan.rs",
            r###"
#[test]
fn cycle_message_shows_the_path() {
    let file = parse("task a : b\ntask b : c\ntask c : a\n").unwrap();
    assert_eq!(
        plan(&file, "a").unwrap_err(),
        "dependency cycle: a -> b -> c -> a"
    );
}
"###,
        )],
    },
    Commit {
        subject: "parse: ignore white space at the end of a line",
        body: "Editors on Windows leave a carriage return there.",
        tag: None,
        author: MIRA,
        edits: &[Replace(
            "src/lib.rs",
            "        if line.trim().is_empty() {\n            continue;\n        }\n",
            "        let line = line.trim_end();\n        if line.is_empty() {\n            continue;\n        }\n",
        )],
    },
    Commit {
        subject: "README: add a complete example",
        body: "",
        tag: None,
        author: MIRA,
        edits: &[Append(
            "README.md",
            "\n## A complete example\n\n```text\nset OUT = build   # where results go\n\ntask clean\n  flags silent\n  step remove ${OUT}\n\ntask site : clean\n  step write ${OUT}/index.txt \"Hello, world\"\n```\n",
        )],
    },
    Commit {
        subject: "Release 0.2.0",
        body: "",
        tag: Some("v0.2.0"),
        author: ADA,
        edits: &[
            Replace("Cargo.toml", "version = \"0.1.1\"", "version = \"0.2.0\""),
            Replace("CHANGELOG.md", "## Unreleased\n", "## 0.2.0\n"),
        ],
    },
];

/// Applies the edits of one commit to the in-memory file tree.
fn apply(files: &mut BTreeMap<&'static str, String>, edits: &[Edit]) -> Result<(), String> {
    for edit in edits {
        match edit {
            Edit::Write(path, text) => {
                files.insert(path, text.to_string());
            }
            Edit::Append(path, text) => {
                files
                    .get_mut(path)
                    .ok_or_else(|| format!("append to missing file {path}"))?
                    .push_str(text);
            }
            Edit::Replace(path, from, to) => {
                let file = files
                    .get_mut(path)
                    .ok_or_else(|| format!("replace in missing file {path}"))?;
                if file.matches(from).count() != 1 {
                    return Err(format!("{path}: expected exactly one `{from}`"));
                }
                *file = file.replacen(from, to, 1);
            }
            Edit::Move {
                from,
                start,
                end,
                to,
                header,
            } => {
                let file = files
                    .get_mut(from)
                    .ok_or_else(|| format!("move from missing file {from}"))?;
                let begin = file
                    .find(start)
                    .ok_or_else(|| format!("{from}: `{start}` not found"))?;
                let stop = match end {
                    Some(end) => file
                        .find(end)
                        .ok_or_else(|| format!("{from}: `{end}` not found"))?,
                    None => file.len(),
                };
                let moved = format!("{header}\n{}\n", file[begin..stop].trim_end());
                let rest = file[stop..].to_string();
                file.truncate(begin);
                if rest.is_empty() {
                    let kept = file.trim_end().len();
                    file.truncate(kept);
                    file.push('\n');
                } else {
                    file.push_str(&rest);
                }
                files.insert(to, moved);
            }
        }
    }
    Ok(())
}

/// The files of the project after the first `count` commits.
pub fn files_after(count: usize) -> Result<BTreeMap<&'static str, String>, String> {
    let mut files = BTreeMap::new();
    for commit in &COMMITS[..count] {
        apply(&mut files, commit.edits)?;
    }
    Ok(files)
}

/// A git command for the repository in `dir` that ignores the user's and the system's
/// configuration and any git environment of the calling process.
fn git(dir: &Path) -> Command {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
        .args(["-c", "core.autocrlf=false", "-c", "init.defaultBranch=main"])
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
    ] {
        command.env_remove(name);
    }
    command
}

fn run(command: &mut Command) -> Result<String, String> {
    let output = command.output().map_err(|error| {
        format!("cannot start git ({error}). Lesson 03 needs git on your PATH.")
    })?;
    if !output.status.success() {
        return Err(format!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Whether `dir` or one of its parents is a git work tree.
fn inside_git_repository(dir: &Path) -> bool {
    dir.ancestors().any(|parent| parent.join(".git").exists())
}

/// Creates the scripted repository in `dir`.
///
/// # Errors
///
/// Refuses to work, with a message, if `dir` exists and is not empty, or if `dir`
/// lies inside a git repository: the history must never mix with real work.
pub fn build(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        let mut entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        if entries.next().is_some() {
            return Err(format!(
                "{} is not empty; delete it or choose another directory",
                dir.display()
            ));
        }
    } else {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let dir: PathBuf = dir.canonicalize().map_err(|e| e.to_string())?;
    if inside_git_repository(&dir) {
        let _ = std::fs::remove_dir(&dir);
        return Err(format!(
            "{} is inside a git repository; choose a directory outside of it",
            dir.display()
        ));
    }

    run(git(&dir).args(["init", "--quiet"]))?;
    run(git(&dir).args(["symbolic-ref", "HEAD", "refs/heads/main"]))?;
    run(git(&dir).args(["config", "user.name", "Tasklane Learner"]))?;
    run(git(&dir).args(["config", "user.email", "learner@tasklane.example"]))?;
    run(git(&dir).args(["config", "core.autocrlf", "false"]))?;
    run(git(&dir).args(["config", "commit.gpgsign", "false"]))?;

    let mut files: BTreeMap<&'static str, String> = BTreeMap::new();
    for (index, commit) in COMMITS.iter().enumerate() {
        apply(&mut files, commit.edits)?;
        for (path, text) in &files {
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&target, text).map_err(|e| e.to_string())?;
        }
        // One commit every three hours from a fixed start, so the history looks the
        // same every time it is generated.
        let hours = 9 + index * 3;
        let date = format!(
            "2026-03-{:02}T{:02}:00:00+00:00",
            2 + hours / 24,
            hours % 24
        );
        let (name, email) = commit
            .author
            .split_once(" <")
            .map(|(name, email)| (name, email.trim_end_matches('>')))
            .expect("author is `Name <email>`");
        let mut message = commit.subject.to_string();
        if !commit.body.is_empty() {
            message.push_str("\n\n");
            message.push_str(commit.body);
        }
        run(git(&dir).args(["add", "--all"]))?;
        run(git(&dir)
            .args(["commit", "--quiet", "--no-verify", "-m", &message])
            .env("GIT_AUTHOR_NAME", name)
            .env("GIT_AUTHOR_EMAIL", email)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_NAME", name)
            .env("GIT_COMMITTER_EMAIL", email)
            .env("GIT_COMMITTER_DATE", &date))?;
        if let Some(tag) = commit.tag {
            run(git(&dir).args(["tag", tag]))?;
        }
    }
    Ok(())
}

/// Runs a read-only git command in `dir` and returns its trimmed output. The tests of
/// lesson 03 use this to look at the generated repository.
pub fn git_output(dir: &Path, args: &[&str]) -> Result<String, String> {
    run(git(dir).args(args))
}
