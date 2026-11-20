//! The `tasklane` command as a library, so that tests can call it without starting a
//! process.
//!
//! Exit codes:
//!
//! | Code | Meaning |
//! | --- | --- |
//! | 0 | success |
//! | 1 | a task failed |
//! | 2 | wrong command line |
//! | 3 | the Lanefile is wrong: syntax, unknown task, cycle, unknown step or variable |
//! | 4 | the Lanefile could not be read |

#![deny(missing_docs)]

pub mod args;

use std::io::{self, Write};

use tasklane_core::{
    Error, Lanefile, Registry, RunOptions, Runner, Vars, Workspace, describe_flags, parse, plan,
};

use crate::args::{Cli, Command, HELP, Show};

/// Runs the command line `args` (without the program name) and returns the exit code.
/// The Lanefile is read from the file system.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    run_with(args, &|path| std::fs::read_to_string(path), out, err)
}

/// Like [`run`], but `load` supplies the text of the Lanefile for a path. Tests use
/// this to run without files.
pub fn run_with(
    args: &[String],
    load: &dyn Fn(&str) -> io::Result<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let cli = match args::parse(args) {
        Ok(cli) => cli,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return 2;
        }
    };
    match cli.command {
        Command::Help => {
            let _ = write!(out, "{HELP}");
            return 0;
        }
        Command::Version => {
            let _ = writeln!(out, "tasklane {}", tasklane_core::VERSION);
            return 0;
        }
        Command::Steps => {
            for step in tasklane_core::builtin::all() {
                let _ = writeln!(out, "{}", step.usage());
            }
            return 0;
        }
        _ => {}
    }
    let source = match load(&cli.file) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(err, "error: cannot read `{}`: {error}", cli.file);
            return 4;
        }
    };
    match execute(&cli, &source, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}: {error}", cli.file);
            3
        }
    }
}

/// The commands that need a Lanefile. An `Err` is a mistake in the Lanefile.
fn execute(cli: &Cli, source: &str, out: &mut dyn Write) -> Result<u8, Error> {
    let file = parse(source)?;
    match &cli.command {
        Command::List => list(&file, out),
        Command::Plan(task) => {
            for name in plan(&file, task)? {
                let _ = writeln!(out, "{name}");
            }
        }
        Command::Check => {
            for name in file.task_names() {
                plan(&file, name)?;
            }
            let _ = writeln!(out, "ok: {} tasks", file.tasks.len());
        }
        Command::Run(task) => return run_task(cli, &file, task, out),
        Command::Help | Command::Version | Command::Steps => {
            unreachable!("handled before the Lanefile is read")
        }
    }
    Ok(0)
}

fn list(file: &Lanefile, out: &mut dyn Write) {
    for task in &file.tasks {
        if task.flags.contains(tasklane_core::TaskFlags::HIDDEN) {
            continue;
        }
        let mut line = task.name.clone();
        if !task.flags.is_empty() {
            line.push_str(&format!(" [{}]", describe_flags(task.flags)));
        }
        if let Some(doc) = &task.doc {
            line.push_str(" - ");
            line.push_str(doc);
        }
        let _ = writeln!(out, "{line}");
    }
}

fn run_task(cli: &Cli, file: &Lanefile, task: &str, out: &mut dyn Write) -> Result<u8, Error> {
    let mut vars = Vars::from_lanefile(file);
    for (name, value) in &cli.sets {
        vars.set(name, value);
    }
    let registry = Registry::with_builtins();
    let options = RunOptions::new()
        .keep_going(cli.keep_going)
        .dry_run(cli.dry_run);
    let mut ws = Workspace::new();
    let report = Runner::new(&registry)
        .with_options(options)
        .run(file, task, &vars, &mut ws)?;
    if cli.show.contains(Show::LOG) {
        for line in ws.log_lines() {
            let _ = writeln!(out, "{line}");
        }
    }
    if cli.show.contains(Show::FILES) {
        for path in ws.paths() {
            let size = ws.read(path).map_or(0, str::len);
            let _ = writeln!(out, "file {path} ({size} bytes)");
        }
    }
    if cli.show.contains(Show::SUMMARY) {
        let _ = writeln!(out, "{}", report.summary());
    }
    Ok(if report.success() { 0 } else { 1 })
}
