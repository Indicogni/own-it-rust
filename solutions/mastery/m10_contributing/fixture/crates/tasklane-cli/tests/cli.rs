//! The command line from the outside: arguments in, text and exit code out.

use std::io;

const SITE: &str = include_str!("../../../examples/site.lane");

/// Runs `tasklane` with `source` as the content of every Lanefile path.
fn tasklane(source: &str, args: &[&str]) -> (u8, String, String) {
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = tasklane_cli::run_with(&args, &|_| Ok(source.to_string()), &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn version_and_help() {
    let (code, out, _) = tasklane("", &["--version"]);
    assert_eq!((code, out.as_str()), (0, "tasklane 0.4.0\n"));
    let (code, out, _) = tasklane("", &["--help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("Usage: tasklane"));
}

#[test]
fn list_shows_flags_and_docs_and_hides_hidden_tasks() {
    let (code, out, _) = tasklane(SITE, &["list"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "clean [silent]\n\
         pages - Writes the two pages.\n\
         stats [silent, optional] - Word counts. May fail without failing the build.\n\
         site - Builds build/site.txt.\n"
    );
}

#[test]
fn plan_prints_one_task_per_line() {
    let (code, out, _) = tasklane(SITE, &["plan", "pages"]);
    assert_eq!((code, out.as_str()), (0, "clean\nheader\npages\n"));
}

#[test]
fn run_prints_log_and_summary() {
    let (code, out, err) = tasklane(SITE, &["run", "pages"]);
    assert_eq!(err, "");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "[header] write build/pages/00-header.txt == My Site ==\n\
         [pages] write build/pages/01-home.txt Welcome to My Site.\n\
         [pages] write build/pages/02-about.txt Made with tasklane.\n\
         3 done, 0 failed, 0 skipped\n"
    );
}

#[test]
fn run_with_files_and_a_variable_override() {
    let (code, out, _) = tasklane(
        SITE,
        &["run", "site", "--show", "files", "--set", "TITLE=Docs"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "file build/pages/00-header.txt (10 bytes)\n\
         file build/pages/01-home.txt (16 bytes)\n\
         file build/pages/02-about.txt (19 bytes)\n\
         file build/site.txt (47 bytes)\n"
    );
}

#[test]
fn a_failed_task_gives_exit_code_1() {
    let source = "task bad\n  step fail no luck\ntask after : bad\n  step echo unreachable\n";
    let (code, out, _) = tasklane(source, &["-q", "run", "after"]);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "0 done, 1 failed, 1 skipped; failed: bad (fail: no luck)\n"
    );
}

#[test]
fn dry_run_shows_the_steps_and_creates_no_file() {
    let (code, out, _) = tasklane(
        SITE,
        &["run", "pages", "--dry-run", "--show", "log,files,summary"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "[header] would run: write build/pages/00-header.txt == My Site ==\n\
         [pages] would run: write build/pages/01-home.txt Welcome to My Site.\n\
         [pages] would run: write build/pages/02-about.txt Made with tasklane.\n\
         0 done, 0 failed, 3 skipped\n"
    );
}

#[test]
fn check_counts_tasks() {
    let (code, out, _) = tasklane(SITE, &["check"]);
    assert_eq!((code, out.as_str()), (0, "ok: 5 tasks\n"));
}

#[test]
fn lanefile_mistakes_give_exit_code_3() {
    let (code, out, err) = tasklane("task a\n  stpe x\n", &["list"]);
    assert_eq!((code, out.as_str()), (3, ""));
    assert_eq!(
        err,
        "error: Lanefile: line 2: unknown directive `stpe` in task `a`\n"
    );

    let (code, _, err) = tasklane("task a\n", &["-f", "ci.lane", "run", "b"]);
    assert_eq!(code, 3);
    assert_eq!(err, "error: ci.lane: unknown task `b`\n");
}

#[test]
fn usage_mistakes_give_exit_code_2() {
    let (code, _, err) = tasklane("", &["frobnicate"]);
    assert_eq!(code, 2);
    assert_eq!(err, "error: unknown command `frobnicate`\n");
}

#[test]
fn an_unreadable_lanefile_gives_exit_code_4() {
    let args = ["list".to_string()];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let load = |_: &str| Err(io::Error::new(io::ErrorKind::NotFound, "no such file"));
    let code = tasklane_cli::run_with(&args, &load, &mut out, &mut err);
    assert_eq!(code, 4);
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "error: cannot read `Lanefile`: no such file\n"
    );
}

#[test]
fn steps_lists_usage_lines() {
    let (code, out, _) = tasklane("", &["steps"]);
    assert_eq!(code, 0);
    assert_eq!(out.lines().count(), 10);
    assert!(out.contains("slice OUT IN START LEN\n"));
}
