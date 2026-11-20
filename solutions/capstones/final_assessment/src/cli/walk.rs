//! From PATH operands to the ordered list of inputs (SPEC 7.3).
//!
//! The whole list is built before the first file is read. Its order is the
//! order of the output, so everything after this module can work on "input
//! number 0, 1, 2, ..." and never thinks about paths again.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// The name that `-l` prints for standard input.
const STDIN_LABEL: &str = "(standard input)";

/// One thing to search, or one thing that could not be looked at.
#[derive(Debug, PartialEq, Eq)]
pub struct Input {
    /// The name as it is printed (SPEC 7.3): the operand as given, then
    /// `/` and a name for every step down into a directory.
    pub label: String,
    pub source: Source,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File {
        path: PathBuf,
        /// `true` for an operand, `false` for a file found in a directory.
        /// Only the first kind must be text (SPEC 7.4).
        named: bool,
    },
    /// The operand or directory could not be examined. The text is the
    /// reason, ready to print.
    Failed(String),
}

/// The inputs in output order, and whether lines get a `PATH:` prefix.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub inputs: Vec<Input>,
    pub show_names: bool,
}

/// Expands the operands. Directories are walked here; no file is opened.
pub fn plan(paths: &[OsString]) -> Plan {
    if paths.is_empty() {
        let stdin = Input {
            label: STDIN_LABEL.to_owned(),
            source: Source::Stdin,
        };
        return Plan {
            inputs: vec![stdin],
            show_names: false,
        };
    }
    let mut inputs = Vec::new();
    let mut show_names = paths.len() > 1;
    for operand in paths {
        let label = operand.to_string_lossy().into_owned();
        let path = PathBuf::from(operand);
        // `metadata` follows a symbolic link: a link given by name is
        // treated as what it points to.
        match fs::metadata(&path) {
            Err(error) => inputs.push(Input {
                label,
                source: Source::Failed(error.to_string()),
            }),
            Ok(meta) if meta.is_dir() => {
                show_names = true;
                walk(&path, &label, &mut inputs);
            }
            Ok(_) => inputs.push(Input {
                label,
                source: Source::File { path, named: true },
            }),
        }
    }
    Plan { inputs, show_names }
}

/// Appends the files below `dir`, depth first, the entries of each
/// directory in the byte order of their names.
fn walk(dir: &Path, label: &str, inputs: &mut Vec<Input>) {
    let entries = fs::read_dir(dir).and_then(|entries| {
        entries
            // `file_type` does not follow links, so a link is neither a
            // file nor a directory here and is passed over below.
            .map(|entry| entry.and_then(|entry| Ok((entry.file_name(), entry.file_type()?))))
            .collect::<Result<Vec<_>, _>>()
    });
    let mut entries = match entries {
        Ok(entries) => entries,
        Err(error) => {
            inputs.push(Input {
                label: label.to_owned(),
                source: Source::Failed(error.to_string()),
            });
            return;
        }
    };
    // `OsString` compares the bytes of the name on every platform. Sorting
    // the names, and not the full paths, is what puts `a/x` before `a-b`.
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    for (name, file_type) in entries {
        let path = dir.join(&name);
        // Always `/`, also on Windows: the output is the same everywhere.
        // An operand that ends in `/` already brings its own (SPEC 7.3).
        let separator = if label.ends_with('/') { "" } else { "/" };
        let label = format!("{label}{separator}{}", name.to_string_lossy());
        if file_type.is_dir() {
            walk(&path, &label, inputs);
        } else if file_type.is_file() {
            inputs.push(Input {
                label,
                source: Source::File { path, named: false },
            });
        }
    }
}
