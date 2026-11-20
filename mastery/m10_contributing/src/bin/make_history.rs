//! Creates the practice repository of lesson 03.
//!
//!     cargo run --manifest-path mastery/m10_contributing/Cargo.toml --bin make-history
//!     cargo run --manifest-path mastery/m10_contributing/Cargo.toml --bin make-history -- DIR
//!
//! Without `DIR` the repository goes to `tasklane-history` in the system's temporary
//! directory. The directory must be empty or absent, and outside every git repository.

use std::path::PathBuf;
use std::process::ExitCode;

use m10_contributing::history;

fn main() -> ExitCode {
    let dir = match std::env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::temp_dir().join("tasklane-history"),
    };
    if let Err(message) = history::build(&dir) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
    }
    let dir = dir.canonicalize().unwrap_or(dir);
    println!(
        "Created a repository with {} commits in:",
        history::COMMITS.len()
    );
    println!("  {}", dir.display());
    println!("Change into that directory for lesson 03. Delete it when you are done.");
    ExitCode::SUCCESS
}
