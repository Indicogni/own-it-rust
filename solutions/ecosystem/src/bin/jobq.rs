//! `jobq`: the E07 capstone binary. Keep it thin; the logic lives in `ecosystem::e07_capstone`.

use std::process::ExitCode;

/// E07. Wire the program together:
/// 1. Parse with `Cli::parse()` (fine in `main`: on bad usage clap prints and exits 2).
/// 2. Install the global tracing subscriber here and only here: logs to stderr, no
///    ANSI colors, max level WARN by default, INFO for `-v`, DEBUG for `-vv` or more.
/// 3. Build a current-thread tokio runtime with time enabled and `block_on(execute(..))`
///    with stdout as the output.
/// 4. On `Ok(code)` exit with that code. On `Err(err)`, print `error: {err:#}` to stderr
///    and exit with `error_exit_code(&err)`.
fn main() -> ExitCode {
    use clap::Parser as _;
    let cli = ecosystem::e07_capstone::Cli::parse();
    let level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        _ => tracing::Level::DEBUG,
    };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_max_level(level)
        .init();
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut stdout = std::io::stdout();
    let code = match runtime.block_on(ecosystem::e07_capstone::execute(cli, &mut stdout)) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ecosystem::e07_capstone::error_exit_code(&err)
        }
    };
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
