//! `tally report [--json] <file>` adds up a ledger file by category.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["report", "--json", file] => run(file, Output::Json),
        // `tally report --json` has no file; do not read a file named "--json".
        ["report", file] if *file != "--json" => run(file, Output::Table),
        _ => {
            eprintln!("usage: tally report [--json] <file>");
            ExitCode::from(2)
        }
    }
}

#[derive(Clone, Copy)]
enum Output {
    Table,
    Json,
}

fn run(file: &str, output: Output) -> ExitCode {
    if matches!(output, Output::Json) && !cfg!(feature = "json") {
        eprintln!("this build has no JSON support (feature `json` is off)");
        return ExitCode::from(2);
    }
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {file}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let entries = match tally_core::parse(&text) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match output {
        Output::Table => print!("{}", tally_core::report(&entries)),
        Output::Json => println!("{}", json(&entries)),
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "json")]
fn json(entries: &[tally_core::Entry]) -> String {
    tally_core::to_json(entries)
}

#[cfg(not(feature = "json"))]
fn json(_entries: &[tally_core::Entry]) -> String {
    unreachable!("`run` returns before it gets here when the feature is off")
}
