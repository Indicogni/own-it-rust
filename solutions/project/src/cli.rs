//! Milestone m18 (Chapter 18), refactored in m19: the command-line program.
//!
//! `src/main.rs` only connects the real process (arguments, stdin, stdout,
//! stderr, exit status) to [`main_with`]. Everything else lives here and
//! takes what it needs from the outside world as parameters: the three
//! streams, and the function that opens a FILE ([`Opener`]). That is why
//! tests can run the whole program on in-memory buffers, hand it a file
//! that fails half-way, and see which thread reads which file, as well as
//! running the built binary.

use crate::error::Error;
use crate::filter::Filter;
use crate::follow::Follower;
use crate::num::{Overflow, parse_u64};
use crate::parallel::map_ordered;
use crate::pipeline::{Tally, process_reader};
use crate::record::Record;
use crate::report::{Group, Summary, render};
use crate::sink::Sink;
use crate::text::truncate;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// The help text, printed by `loglens --help`. Its first line is the usage
/// line printed after argument errors.
pub const USAGE: &str = "\
usage: loglens <grep|count|report> [OPTIONS] [FILE...]

Commands:
  grep     print matching records, one canonical line each
  count    print the number of matching records
  report   print a summary of matching records

Options:
  -w, --where EXPR  keep records matching EXPR (repeatable; terms are ANDed)
  -b, --by KEY      report groups: target, level, or a field name [default: target]
  -n, --top N       report rows [default: 10]
  -j, --jobs N      worker threads for several files; 0 = one per core [default: 1]
      --width N     grep: cut each output line to N characters
  -f, --follow      grep: keep reading one FILE as it grows
      --strict      exit with status 2 if any line is malformed
  -h, --help        print this help

FILE '-', or no FILE, reads standard input.
Exit status: 0 if a record matched, 1 if none matched, 2 on error.
";

/// What the program was asked to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Command {
    /// Print [`USAGE`].
    #[default]
    Help,
    /// Print matching records.
    Grep,
    /// Print the number of matching records.
    Count,
    /// Print a [`crate::report::render`] report.
    Report,
}

/// One input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Standard input (`-`).
    Stdin,
    /// A file.
    Path(PathBuf),
}

/// Parsed command-line options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The command.
    pub command: Command,
    /// The filter expression: every `--where` value, joined by one space.
    pub filter: String,
    /// `--by`.
    pub group: Group,
    /// `--top`.
    pub top: usize,
    /// `--jobs`; 0 means one per core.
    pub jobs: usize,
    /// `--width`.
    pub width: Option<usize>,
    /// `--follow`.
    pub follow: bool,
    /// `--strict`.
    pub strict: bool,
    /// The inputs, in order; `[Input::Stdin]` when `grep`, `count`, or
    /// `report` was given no FILE. Empty only for [`Command::Help`].
    pub inputs: Vec<Input>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            command: Command::Help,
            filter: String::new(),
            group: Group::Target,
            top: 10,
            jobs: 1,
            width: None,
            follow: false,
            strict: false,
            inputs: Vec::new(),
        }
    }
}

/// m18 D. Parse the arguments (without the program name), by hand.
///
/// - An argument `-h` or `--help` before any `--` returns
///   `Options::default()` (whose command is `Help`), ignoring everything
///   else, even an unknown command.
/// - The first argument is the command: `grep`, `count`, or `report`. The
///   command `help` returns `Options::default()`, exactly like `--help`
///   (so its `inputs` stay empty).
/// - Options may appear anywhere after the command. A value is the next
///   argument (`--top 5`, `-n 5`) or, for long names only, follows `=`
///   (`--top=5`). Short options are never combined (`-fn` is unknown).
/// - `--` ends the options: every later argument is a FILE. `-` is
///   [`Input::Stdin`]; any other argument that starts with `-` is an option.
/// - Numbers use [`crate::num::parse_u64`] with `Overflow::Reject`, then
///   `usize::try_from`.
/// - Option names, values, and the command must be UTF-8. FILE arguments
///   need not be: keep them as `OsString`/`PathBuf`.
///
/// Every problem is an [`Error::Usage`] with one of these messages:
///
/// | Problem | Message |
/// | --- | --- |
/// | no arguments | `missing command` |
/// | first argument is not a command | `unknown command 'X'` |
/// | an unknown option | `unknown option 'X'` |
/// | the value is missing | `option 'X' needs a value` (X as typed: `-n` or `--top`) |
/// | a number is invalid | `invalid number 'V' for 'X'` |
/// | non-UTF-8 command, option, or value | `arguments must be valid UTF-8` |
/// | `--follow` with another command than grep (checked first) | `--follow works only with grep` |
/// | `--follow` without exactly one FILE (not `-`) | `--follow needs exactly one FILE` |
pub fn parse_args<I, S>(args: I) -> Result<Options, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let wants_help = args
        .iter()
        .take_while(|a| a.as_os_str() != "--")
        .any(|a| a.as_os_str() == "-h" || a.as_os_str() == "--help");
    if wants_help {
        return Ok(Options::default());
    }
    let usage = |message: String| Error::Usage(message);
    let utf8 = |arg: OsString| {
        arg.into_string()
            .map_err(|_| Error::Usage("arguments must be valid UTF-8".to_string()))
    };
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(usage("missing command".to_string()));
    };
    let command = match utf8(command)?.as_str() {
        "grep" => Command::Grep,
        "count" => Command::Count,
        "report" => Command::Report,
        "help" => return Ok(Options::default()),
        other => return Err(usage(format!("unknown command '{other}'"))),
    };
    let mut opts = Options {
        command,
        ..Options::default()
    };
    let mut filters: Vec<String> = Vec::new();
    let mut only_files = false;
    while let Some(arg) = args.next() {
        let bytes = arg.as_encoded_bytes();
        if only_files || bytes == b"-" || !bytes.starts_with(b"-") {
            opts.inputs.push(if bytes == b"-" {
                Input::Stdin
            } else {
                Input::Path(PathBuf::from(arg))
            });
            continue;
        }
        let arg = utf8(arg)?;
        if arg == "--" {
            only_files = true;
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        match name {
            "-f" | "--follow" if inline.is_none() => opts.follow = true,
            "--strict" if inline.is_none() => opts.strict = true,
            "-w" | "--where" | "-b" | "--by" | "-n" | "--top" | "-j" | "--jobs" | "--width" => {
                let value = match inline {
                    Some(value) => value,
                    None => utf8(
                        args.next()
                            .ok_or_else(|| usage(format!("option '{name}' needs a value")))?,
                    )?,
                };
                let number = || {
                    parse_u64(&value, Overflow::Reject)
                        .ok()
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or_else(|| usage(format!("invalid number '{value}' for '{name}'")))
                };
                match name {
                    "-w" | "--where" => filters.push(value),
                    "-b" | "--by" => opts.group = Group::from(value.as_str()),
                    "-n" | "--top" => opts.top = number()?,
                    "-j" | "--jobs" => opts.jobs = number()?,
                    _ => opts.width = Some(number()?),
                }
            }
            _ => return Err(usage(format!("unknown option '{arg}'"))),
        }
    }
    opts.filter = filters.join(" ");
    if opts.follow {
        if opts.command != Command::Grep {
            return Err(usage("--follow works only with grep".to_string()));
        }
        if !matches!(opts.inputs.as_slice(), [Input::Path(_)]) {
            return Err(usage("--follow needs exactly one FILE".to_string()));
        }
    }
    if opts.inputs.is_empty() {
        opts.inputs.push(Input::Stdin);
    }
    Ok(opts)
}

/// How the program opens a FILE: a function from the path to a buffered
/// reader. The real program passes [`open_file`]. Tests pass their own
/// opener to serve text from memory, to return a reader that fails, or to
/// record which thread opened which file.
///
/// The `Sync` bound lets worker threads share one opener (m19). The reader
/// it returns is used only on the thread that asked for it.
pub type Opener<'a> = &'a (dyn Fn(&Path) -> io::Result<Box<dyn BufRead>> + Sync);

/// The [`Opener`] of the real program: the file at `path`, buffered.
/// Provided.
pub fn open_file(path: &Path) -> io::Result<Box<dyn BufRead>> {
    Ok(Box::new(BufReader::new(File::open(path)?)))
}

/// The program on real files: [`run_with`] and [`open_file`]. Provided.
pub fn run(
    opts: &Options,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    run_with(opts, &open_file, stdin, stdout, stderr)
}

/// m18 E. Run parsed options; return the exit status.
///
/// Every FILE is opened by calling `open` with the path as given, and never
/// in another way.
///
/// 1. `Command::Help`: write [`USAGE`] to `stdout`, return 0.
/// 2. Compile the filter. If invalid, write `loglens: {error}` (the
///    [`Error::Filter`] message) and a newline to `stderr`, and return 2.
/// 3. `--follow`: follow the one file with [`crate::follow::Follower`],
///    printing matching lines; flush `stdout` after each poll and sleep
///    about 100 ms when a poll found nothing. This never returns unless an
///    error happens.
/// 4. Otherwise process each input in order with
///    [`crate::pipeline::process_reader`]:
///    - `grep` writes each matching record's canonical line to `stdout`,
///      cut to `--width` characters with [`crate::text::truncate`] if given;
///    - `count` and `report` aggregate across all inputs and, after the last
///      input, write `{matched}\n` or [`crate::report::render`] output.
///    - A file that cannot be opened (`open` returns an error): write
///      `loglens: {error}` for the [`Error::Open`] and a newline to `stderr`,
///      and go on with the next.
///    - A malformed line: write `loglens: {name}: {parse error}` and a newline
///      to `stderr`, where name is the path as given (`path.display()`) or
///      `<stdin>`.
///    - A read error in the middle of an input: write `loglens: {name}:
///      {error}` and a newline to `stderr`, and go on with the next input.
///      The status will be 2, as for a file that could not be opened. The
///      records that matched before the error still count: `grep` has
///      already printed them, and `count` and `report` include them.
///      (`process_reader` returns only the error then, not its tally, so
///      keep that count where the error cannot lose it: in your sink.)
/// 5. Flush `stdout`. The status is 2 if a file could not be opened or read
///    to its end, or if `--strict` is set and any line was malformed;
///    otherwise 0 if at least one record matched and 1 if none did.
///
/// Writing to `stdout` can fail. If the reader went away (`BrokenPipe`, as
/// in `loglens grep app.log | head -1`), stop quietly and return 0. For any
/// other write error, report `loglens: {Error::Io}` and return 2.
///
/// m19 refactor: when `jobs != 1` and there are several inputs, none of them
/// stdin, process the files with [`crate::parallel::map_ordered`]: each
/// worker calls `open` for its file and reads it on its own thread. Buffer
/// each file's output and messages, then write them in input order, so the
/// output is byte-for-byte the same as with `--jobs 1`. With `--jobs 1`,
/// with a single input, or when stdin is one of the inputs, everything
/// happens on the calling thread.
pub fn run_with(
    opts: &Options,
    open: Opener<'_>,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    if opts.command == Command::Help {
        return finish_stdout(stdout.write_all(USAGE.as_bytes()), stdout, stderr, 0);
    }
    let filter = match Filter::parse(&opts.filter) {
        Ok(filter) => filter,
        Err(e) => {
            let _ = writeln!(stderr, "loglens: {}", Error::from(e));
            return 2;
        }
    };
    if opts.follow {
        return follow(opts, open, &filter, stdout, stderr);
    }
    let mut total = Outcome::new(&opts.group);

    // m19: several files and more than one job. `paths` is `None` as soon as
    // one input is stdin, which only the calling thread may read.
    let paths: Option<Vec<&Path>> = opts
        .inputs
        .iter()
        .map(|input| match input {
            Input::Path(path) => Some(path.as_path()),
            Input::Stdin => None,
        })
        .collect();
    if let Some(paths) = paths.filter(|paths| opts.jobs != 1 && paths.len() > 1) {
        // Each worker opens and reads one file and writes into its own
        // buffers. Nothing reaches the real streams until the results are
        // back in input order.
        let results = map_ordered(&paths, opts.jobs, |path| {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let outcome = process_path(opts, open, &filter, path, &mut out, &mut err);
            (out, err, outcome)
        });
        for (out, err, outcome) in results {
            let _ = stderr.write_all(&err);
            match stdout.write_all(&out).and(outcome) {
                Ok(one) => total.merge(one),
                Err(e) => return stdout_failed(e, stderr),
            }
        }
        return finish(opts, &total, stdout, stderr);
    }

    for input in &opts.inputs {
        let one = match input {
            Input::Stdin => read_input(opts, &filter, "<stdin>", &mut *stdin, stdout, stderr),
            Input::Path(path) => process_path(opts, open, &filter, path, stdout, stderr),
        };
        match one {
            Ok(one) => total.merge(one),
            Err(e) => return stdout_failed(e, stderr),
        }
    }
    finish(opts, &total, stdout, stderr)
}

/// m18 F. The whole program: parse `args` and run them.
///
/// On an argument error, write `loglens: {error}`, a newline, and the first
/// line of [`USAGE`] (with its newline) to `stderr`, and return 2.
pub fn main_with<I, S>(
    args: I,
    stdin: &mut dyn BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    match parse_args(args) {
        Ok(opts) => run(&opts, stdin, stdout, stderr),
        Err(e) => {
            let usage = USAGE.lines().next().unwrap_or_default();
            let _ = writeln!(stderr, "loglens: {e}\n{usage}");
            2
        }
    }
}

/// Write the final count or report, flush, and choose the status.
fn finish(opts: &Options, total: &Outcome, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let written = match opts.command {
        Command::Count => writeln!(stdout, "{}", total.tally.matched),
        Command::Report => stdout.write_all(render(&total.summary, opts.top).as_bytes()),
        _ => Ok(()),
    };
    let status = if total.failed || (opts.strict && total.tally.malformed > 0) {
        2
    } else if total.tally.matched > 0 {
        0
    } else {
        1
    };
    finish_stdout(written, stdout, stderr, status)
}

/// What one input (or all of them together) produced.
struct Outcome {
    tally: Tally,
    summary: Summary,
    /// The input could not be opened, or not be read to its end.
    failed: bool,
}

impl Outcome {
    fn new(group: &Group) -> Self {
        Outcome {
            tally: Tally::default(),
            summary: Summary::new(group.clone()),
            failed: false,
        }
    }

    fn merge(&mut self, other: Outcome) {
        self.tally += other.tally;
        self.summary += other.summary;
        self.failed |= other.failed;
    }
}

/// The sink of every command. It remembers whether *its own* write failed,
/// which is how `run` tells a broken stdout from an unreadable input.
struct Output<'a> {
    command: Command,
    width: Option<usize>,
    out: &'a mut dyn Write,
    summary: &'a mut Summary,
    write_failed: bool,
    matched: u64,
}

impl Sink for Output<'_> {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        let result = match (self.command, self.width) {
            (Command::Grep, Some(width)) => {
                let line = record.to_string();
                writeln!(self.out, "{}", truncate(&line, width))
            }
            (Command::Grep, None) => writeln!(self.out, "{record}"),
            (Command::Report, _) => {
                self.summary.add(record);
                Ok(())
            }
            _ => Ok(()),
        };
        self.write_failed |= result.is_err();
        self.matched += 1;
        result
    }
}

/// Process one reader. `Err` means stdout failed; everything else is in the
/// outcome.
fn read_input(
    opts: &Options,
    filter: &Filter,
    name: &str,
    reader: impl BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<Outcome> {
    let mut outcome = Outcome::new(&opts.group);
    let mut sink = Output {
        command: opts.command,
        width: opts.width,
        out: stdout,
        summary: &mut outcome.summary,
        write_failed: false,
        matched: 0,
    };
    // `process_reader` returns only the error when a read fails, not its
    // tally, so the sink and this closure keep their own counts.
    let mut malformed = 0;
    let result = process_reader(reader, filter, &mut sink, &mut |e| {
        malformed += 1;
        let _ = writeln!(stderr, "loglens: {name}: {e}");
    });
    let (write_failed, matched) = (sink.write_failed, sink.matched);
    match result {
        Ok(tally) => outcome.tally = tally,
        Err(e) if write_failed => return Err(e),
        Err(e) => {
            let _ = writeln!(stderr, "loglens: {name}: {e}");
            outcome.tally.malformed = malformed;
            outcome.tally.matched = matched;
            outcome.failed = true;
        }
    }
    Ok(outcome)
}

/// Open one FILE through `open` and process it. A file that cannot be
/// opened is reported and marks the outcome as failed; it is not an `Err`.
fn process_path(
    opts: &Options,
    open: Opener<'_>,
    filter: &Filter,
    path: &Path,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<Outcome> {
    match open(path) {
        Ok(reader) => {
            let name = path.display().to_string();
            read_input(opts, filter, &name, reader, stdout, stderr)
        }
        Err(source) => {
            let error = Error::Open {
                path: path.to_path_buf(),
                source,
            };
            let _ = writeln!(stderr, "loglens: {error}");
            let mut outcome = Outcome::new(&opts.group);
            outcome.failed = true;
            Ok(outcome)
        }
    }
}

/// The status after a failed write to stdout.
fn stdout_failed(error: io::Error, stderr: &mut dyn Write) -> u8 {
    if error.kind() == io::ErrorKind::BrokenPipe {
        return 0;
    }
    let _ = writeln!(stderr, "loglens: {}", Error::Io(error));
    2
}

/// Flush stdout after the last write and turn failures into a status.
fn finish_stdout(
    written: io::Result<()>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    status: u8,
) -> u8 {
    match written.and_then(|()| stdout.flush()) {
        Ok(()) => status,
        Err(e) => stdout_failed(e, stderr),
    }
}

/// `grep --follow`: poll the one file forever, printing matches as they
/// arrive. Returns only when opening, reading, or writing fails.
fn follow(
    opts: &Options,
    open: Opener<'_>,
    filter: &Filter,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    // `parse_args` guarantees exactly one path; options built by hand may not.
    let [Input::Path(path)] = opts.inputs.as_slice() else {
        let _ = writeln!(stderr, "loglens: --follow needs exactly one FILE");
        return 2;
    };
    let reader = match open(path) {
        Ok(reader) => reader,
        Err(source) => {
            let error = Error::Open {
                path: path.clone(),
                source,
            };
            let _ = writeln!(stderr, "loglens: {error}");
            return 2;
        }
    };
    let name = path.display().to_string();
    let mut follower = Follower::new(reader);
    // `Output` needs a summary to borrow; grep never adds to it.
    let mut unused = Summary::new(opts.group.clone());
    loop {
        let mut sink = Output {
            command: Command::Grep,
            width: opts.width,
            out: &mut *stdout,
            summary: &mut unused,
            write_failed: false,
            matched: 0,
        };
        let result = follower.poll(filter, &mut sink, &mut |e| {
            let _ = writeln!(stderr, "loglens: {name}: {e}");
        });
        let write_failed = sink.write_failed;
        match result {
            Ok(tally) => {
                // Flush after every poll: a follower's reader wants each line
                // as soon as it exists, not when a buffer fills up.
                if let Err(e) = stdout.flush() {
                    return stdout_failed(e, stderr);
                }
                if tally == Tally::default() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            Err(e) if write_failed => return stdout_failed(e, stderr),
            Err(e) => {
                let _ = writeln!(stderr, "loglens: {name}: {e}");
                return 2;
            }
        }
    }
}
