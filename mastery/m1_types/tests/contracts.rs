//! Contract checks: small programs compiled against your `m1_types` library.
//!
//! Many rules in this module are about what must **not** compile, and an
//! ordinary `#[test]` cannot check that: a test that does not compile takes the
//! whole test binary down with it. So each check here is a separate program in
//! `tests/cases/`. This file compiles each one with `rustc`, the same way a
//! user of your library would, and compares what happened with the first line
//! of the case file:
//!
//! - `//@ expect: run`: the program must compile, run and exit normally.
//! - `//@ expect: E0599` (any error code): the compiler must reject the program
//!   with exactly this error code.
//! - `//@ expect: error`: the compiler must reject the program with an error
//!   that has no code (some lifetime errors have none).
//! - `//@ expect: E0308 | E0277`: the compiler must reject the program, and
//!   every error code it reports must be in the list. This is used where two
//!   sound designs lead to different messages.
//!
//! The `//@ why:` line of a case says what the check means. You do not need to
//! edit this file or the cases. Read the cases: they are the precise contract.
//!
//! Run one lesson's checks with, for example, `cargo test l07_`.

// Miri cannot start other programs, and these checks do nothing else.
#![cfg(not(miri))]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

/// How long one compiler run or one case program may take.
const LIMIT: Duration = Duration::from_secs(120);

fn rustc() -> Command {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut command = Command::new(rustc);
    // Run from the package directory, so that rust-toolchain.toml applies.
    command
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--edition", "2024", "--color", "never"])
        .args(["-A", "warnings", "-C", "debuginfo=0"]);
    command
}

/// Runs a command to its end, or stops it after [`LIMIT`].
fn run_limited(command: &mut Command, what: &str) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {what}: {e}. Is `rustc` on your PATH?"));
    // Read both pipes on helper threads, so that a full pipe never blocks the child.
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let read_out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stdout, &mut bytes);
        bytes
    });
    let read_err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stderr, &mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().expect("waiting for a child process") {
            Some(status) => break status,
            None if started.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "{what} was still running after {} s and was stopped. \
                     Look for a loop that never ends.",
                    LIMIT.as_secs()
                );
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    Output {
        status,
        stdout: read_out.join().unwrap_or_default(),
        stderr: read_err.join().unwrap_or_default(),
    }
}

/// A private scratch directory for this test process, inside Cargo's target
/// directory. Directories left behind by earlier runs are removed.
fn scratch() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR"));
        let stale = Duration::from_secs(15 * 60);
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let old = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|time| SystemTime::now().duration_since(time).ok())
                    .is_some_and(|age| age > stale);
                if old
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("contracts-")
                {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        let dir = base.join(format!("contracts-{}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", dir.display()));
        dir
    })
}

/// Builds the library once per test run and returns the path of the `.rlib`.
///
/// The library is built from `src/lib.rs` directly, so the cases always see
/// the code that is on disk now.
fn library() -> &'static Path {
    static LIB: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let built = LIB.get_or_init(|| {
        let out_dir = scratch().join("lib");
        std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
        let mut command = rustc();
        command
            .args(["--crate-type", "rlib", "--crate-name", "m1_types"])
            .arg("--out-dir")
            .arg(&out_dir)
            .arg("src/lib.rs");
        let output = run_limited(&mut command, "rustc (building the library)");
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        Ok(out_dir.join("libm1_types.rlib"))
    });
    match built {
        Ok(path) => path,
        Err(errors) => panic!("src/lib.rs does not compile, so no case can run:\n{errors}"),
    }
}

/// What the first line of a case file asks for.
#[derive(Debug, PartialEq)]
enum Expect {
    Run,
    /// A compile error whose codes are all in this list. The word `error`
    /// stands for an error without a code.
    Reject(Vec<String>),
}

fn header(source: &str, key: &str) -> (Expect, String) {
    let field = |name: &str| {
        source
            .lines()
            .find_map(|line| line.strip_prefix("//@ ")?.strip_prefix(name))
            .map(|rest| rest.trim().to_string())
    };
    let is_outcome = |word: &str| {
        word == "error"
            || (word.len() == 5
                && word.starts_with('E')
                && word[1..].bytes().all(|b| b.is_ascii_digit()))
    };
    let expect = match field("expect:").as_deref() {
        Some("run") => Expect::Run,
        Some(list) if list.split('|').all(|word| is_outcome(word.trim())) => Expect::Reject(
            list.split('|')
                .map(|word| word.trim().to_string())
                .collect(),
        ),
        other => panic!("{key}: cannot read its `//@ expect:` line ({other:?})"),
    };
    (expect, field("why:").unwrap_or_default())
}

/// Error codes in a compiler message, without repeats, in order of appearance.
fn error_codes(diagnostics: &str) -> Vec<String> {
    let mut codes: Vec<String> = Vec::new();
    for line in diagnostics.lines() {
        let code = line
            .strip_prefix("error[")
            .and_then(|rest| rest.split(']').next());
        if let Some(code) = code
            && !codes.iter().any(|known| known == code)
        {
            codes.push(code.to_string());
        }
    }
    codes
}

/// Compiles (and for `run` cases, runs) one case and checks its outcome.
/// `key` is a path below `tests/cases/`.
#[track_caller]
fn case(key: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cases")
        .join(key);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (expect, why) = header(&source, key);
    let library = library();
    let name: String = key
        .trim_end_matches(".rs")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let out_dir = scratch().join(&name);
    std::fs::create_dir_all(&out_dir).expect("creating a scratch directory");
    let exe = out_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));

    let mut compile = rustc();
    compile
        .args(["--crate-type", "bin", "--crate-name", &name])
        .arg("--extern")
        .arg(format!("m1_types={}", library.display()))
        .arg("-L")
        .arg(library.parent().expect("the library is in a directory"));
    if expect == Expect::Run {
        compile.arg("-o").arg(&exe);
    } else {
        // Type checking and borrow checking are enough to see the error.
        compile
            .arg("--emit=metadata")
            .arg("--out-dir")
            .arg(&out_dir);
    }
    compile.arg(&path);
    let compiled = run_limited(&mut compile, "rustc");
    let diagnostics = String::from_utf8_lossy(&compiled.stderr).into_owned();
    let codes = error_codes(&diagnostics);
    let shown = format!("tests/cases/{key}");
    let mut report = String::new();

    // A real compile error points into a source file. If rustc failed without
    // doing so, the tools are broken, and no verdict about the case is due.
    let points_into_source = diagnostics
        .lines()
        .any(|line| line.trim_start().starts_with("--> "));
    assert!(
        compiled.status.success() || points_into_source,
        "\nrustc could not build {shown}. This is a problem with the tools or the disk, \
         not with your code:\n{diagnostics}"
    );

    match (&expect, compiled.status.success()) {
        (Expect::Run, true) => {
            let ran = run_limited(&mut Command::new(&exe), &shown);
            if !ran.status.success() {
                let _ = write!(
                    report,
                    "{shown} compiled, but failed when it ran.\nMeaning: {why}\n\
                     --- output of the program ---\n{}{}",
                    String::from_utf8_lossy(&ran.stdout),
                    String::from_utf8_lossy(&ran.stderr)
                );
            }
        }
        (Expect::Run, false) => {
            let _ = write!(
                report,
                "{shown} must compile and run, but the compiler rejected it.\nMeaning: {why}\n"
            );
            if missing_item(&codes) {
                report.push_str(
                    "The program names an item that does not exist (yet) or is not `pub`. \
                     Check the names in the lesson's contract.\n",
                );
            }
            let _ = write!(report, "--- compiler output ---\n{diagnostics}");
        }
        (Expect::Reject(wanted), true) => {
            let _ = write!(
                report,
                "{shown} must NOT compile (expected {}), but the compiler accepted it.\n\
                 Meaning: {why}\nRead the case file to see the misuse that got through.",
                wanted.join(" or ")
            );
        }
        (Expect::Reject(wanted), false) => {
            // Every code that the compiler reported must be an expected one.
            // An error without any code is written `error`.
            let found = if codes.is_empty() {
                "error".to_string()
            } else {
                codes.join(", ")
            };
            let as_expected = if codes.is_empty() {
                wanted.iter().any(|code| code == "error")
            } else {
                codes.iter().all(|code| wanted.contains(code))
            };
            if !as_expected {
                let _ = write!(
                    report,
                    "{shown} was rejected, but for another reason than the contract asks for.\n\
                     Expected: {}. Found: {found}. (`error` means an error without a code.)\n\
                     Meaning: {why}\n",
                    wanted.join(" or ")
                );
                if missing_item(&codes) {
                    report.push_str(
                        "The program names an item that does not exist (yet) or is not `pub`. \
                         Check the names in the lesson's contract.\n",
                    );
                }
                let _ = write!(report, "--- compiler output ---\n{diagnostics}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    assert!(report.is_empty(), "\n{report}");
}

/// Codes the compiler uses for a name it cannot find or may not use.
fn missing_item(codes: &[String]) -> bool {
    const MISSING: [&str; 6] = ["E0412", "E0425", "E0432", "E0433", "E0405", "E0603"];
    codes.iter().any(|code| MISSING.contains(&code.as_str()))
}

/// Declares one `#[test]` per case. The test name starts with the lesson
/// number, so `cargo test l03_` runs a lesson's unit tests and its cases.
macro_rules! cases {
    ($($name:ident => $key:literal,)+) => {
        $(
            #[test]
            fn $name() {
                case($key);
            }
        )+
    };
}

cases! {
    c01_longest_of_local_strings => "c01/longest_of_local_strings.rs",
    c01_names_hides_its_iterator => "c01/names_hides_its_iterator.rs",
    c01_stage_may_borrow => "c01/stage_may_borrow.rs",
    l01_record_rejects_short_entry => "l01/record_rejects_short_entry.rs",
    l01_record_twice => "l01/record_twice.rs",
    l02_sum_by_ref_never_copies => "l02/sum_by_ref_never_copies.rs",
    l02_words_outlive_the_borrow => "l02/words_outlive_the_borrow.rs",
    l03_shouter_reuses_its_buffer => "l03/shouter_reuses_its_buffer.rs",
    l03_take_limits_any_lender => "l03/take_limits_any_lender.rs",
    l03_two_windows_at_once => "l03/two_windows_at_once.rs",
    l04_id_is_covariant => "l04/id_is_covariant.rs",
    l04_raw_iter_cannot_outlive_slice => "l04/raw_iter_cannot_outlive_slice.rs",
    l04_raw_iter_is_covariant => "l04/raw_iter_is_covariant.rs",
    l04_raw_iter_lifetime_cannot_grow => "l04/raw_iter_lifetime_cannot_grow.rs",
    l05_boxed_labels_are_labels => "l05/boxed_labels_are_labels.rs",
    l05_codecs_in_one_list => "l05/codecs_in_one_list.rs",
    l05_legend_accepts_borrowing_labels => "l05/legend_accepts_borrowing_labels.rs",
    l06_shout_cannot_fail => "l06/shout_cannot_fail.rs",
    l06_token_cannot_be_forged => "l06/token_cannot_be_forged.rs",
    l06_token_is_free_proof => "l06/token_is_free_proof.rs",
    l07_cc_needs_a_recipient => "l07/cc_needs_a_recipient.rs",
    l07_envelope_flow => "l07/envelope_flow.rs",
    l07_identity_is_square_only => "l07/identity_is_square_only.rs",
    l07_matrix_mul_shape_mismatch => "l07/matrix_mul_shape_mismatch.rs",
    l07_matrix_shapes => "l07/matrix_shapes.rs",
    l07_new_starts_at_needs_to => "l07/new_starts_at_needs_to.rs",
    l07_send_needs_a_body => "l07/send_needs_a_body.rs",
    l07_stage_is_consumed => "l07/stage_is_consumed.rs",
    l07_stages_are_sealed => "l07/stages_are_sealed.rs",
    l08_describe_std_types => "l08/describe_std_types.rs",
    l08_describe_through_pointers => "l08/describe_through_pointers.rs",
    l08_level_conversions => "l08/level_conversions.rs",
    l08_shown_needs_display => "l08/shown_needs_display.rs",
    l08_user_type_with_display => "l08/user_type_with_display.rs",
    l09_add_does_not_convert => "l09/add_does_not_convert.rs",
    l09_add_needs_same_unit => "l09/add_needs_same_unit.rs",
    l09_add_rejects_bare_number => "l09/add_rejects_bare_number.rs",
    l09_convert_needs_same_dimension => "l09/convert_needs_same_dimension.rs",
    l09_quantities_flow => "l09/quantities_flow.rs",
    l09_quantity_is_plain_data => "l09/quantity_is_plain_data.rs",
    l09_units_are_sealed => "l09/units_are_sealed.rs",
}
