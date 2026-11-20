//! LESSON 01 — Orienting in an unfamiliar codebase
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l01_
//! Read first: CONCEPTS.md, "Lesson 01". The project under study is `fixture/`
//! (tasklane). Build it, run it, and read its README before you answer anything.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Which file would you open first to
//! learn what a project's crates are for? How can a test name lead you to the code
//! that implements a rule?
//! IMPLEMENT the three tasks. Every answer is something you look up in the fixture:
//! with `cargo doc --open`, with `cargo test -- --list`, with a text search, or with
//! "go to definition" in your editor. Preserve the tests.
//! EXPLAIN afterward: Describe the path of one Lanefile through tasklane, from the
//! command line to the summary line, naming the crate and module of each stage. Say
//! which of your lookups was fastest with which tool.

/// A behaviour of tasklane that somebody may ask you to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    /// `${NAME}` in a step argument is replaced by the variable's value.
    Expansion,
    /// `"two words"` in a step line becomes one argument.
    Quoting,
    /// The decision which tasks run for a target, and in which order.
    Ordering,
    /// The process exit code for each kind of failure.
    ExitCodes,
    /// The decision that a task is skipped because one of its dependencies failed.
    SkipRule,
    /// The definition of the trait that a plugin implements.
    StepTrait,
}

/// A. Where each behaviour lives. Answer with the Rust path of the module that holds
/// the code, as rustdoc prints it: for example `tasklane_core::parse`. When the code
/// is in a crate's root module (`lib.rs`), the path is the crate name alone.
pub fn home_of(behaviour: Behaviour) -> &'static str {
    match behaviour {
        Behaviour::Expansion => "tasklane_core::vars",
        Behaviour::Quoting => "tasklane_core::text",
        Behaviour::Ordering => "tasklane_core::graph",
        Behaviour::ExitCodes => "tasklane_cli",
        Behaviour::SkipRule => "tasklane_core::run",
        Behaviour::StepTrait => "tasklane_plugin",
    }
}

/// A rule that the fixture's own test suite pins down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// A `step` line that is not inside a task is a parse error.
    StepNeedsTask,
    /// Registering a step with the name of a built-in step fails.
    PluginCannotReplaceBuiltin,
    /// A failing task with the `optional` flag does not stop the tasks after it.
    OptionalFailure,
    /// A Lanefile saved with Windows line endings parses like any other.
    WindowsLineEndings,
}

/// B. Tests are documentation that cannot go stale. For each rule, give the name of
/// the test function in the fixture that would fail if the rule were broken (the
/// function name only, without module path).
pub fn pinning_test(rule: Rule) -> &'static str {
    match rule {
        Rule::StepNeedsTask => "step_outside_task_is_an_error",
        Rule::PluginCannotReplaceBuiltin => "a_plugin_cannot_replace_a_builtin",
        Rule::OptionalFailure => "an_optional_failure_blocks_nothing",
        Rule::WindowsLineEndings => "windows_line_endings",
    }
}

/// One thing that happens during `tasklane -f examples/site.lane run site --set TITLE=Docs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The variables given with `--set` replace the values of the Lanefile.
    ApplySetOverrides,
    /// The dependency order of the tasks is computed.
    PlanOrder,
    /// The Lanefile text becomes a `Lanefile` value.
    ParseLanefile,
    /// The log lines are written to standard output.
    PrintLog,
    /// The command line becomes a `Cli` value.
    ParseArguments,
    /// Every step name of every planned task is looked up, and every argument is expanded.
    PrepareSteps,
    /// The summary line is written to standard output.
    PrintSummary,
    /// The Lanefile is read from disk.
    ReadLanefile,
    /// The steps run and change the workspace.
    RunSteps,
}

/// C. The data flow. Return the nine stages in the order in which they happen.
/// Follow the calls, starting at `main` in `fixture/crates/tasklane-cli/src/main.rs`.
pub fn trace_of_a_run() -> [Stage; 9] {
    [
        Stage::ParseArguments,
        Stage::ReadLanefile,
        Stage::ParseLanefile,
        Stage::ApplySetOverrides,
        Stage::PlanOrder,
        Stage::PrepareSteps,
        Stage::RunSteps,
        Stage::PrintLog,
        Stage::PrintSummary,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, digest, package_dir};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;

    #[test]
    fn a_home_of_each_behaviour() {
        let expected = [
            (Behaviour::Expansion, 0x1666_a53c_d013_a16a_u64),
            (Behaviour::Quoting, 0xcd19_582a_fbe7_54ab),
            (Behaviour::Ordering, 0x4cc2_3d5a_32af_1400),
            (Behaviour::ExitCodes, 0x4c26_a845_9477_dd7f),
            (Behaviour::SkipRule, 0xb62d_ca8e_0a48_2d29),
            (Behaviour::StepTrait, 0xd484_b845_643a_f9b0),
        ];
        for (behaviour, model) in expected {
            assert_answer(&format!("A, {behaviour:?}"), home_of(behaviour), model);
        }
    }

    #[test]
    fn b_test_that_pins_each_rule() {
        let expected = [
            (Rule::StepNeedsTask, 0x989e_bf8a_6f81_2ca5_u64),
            (Rule::PluginCannotReplaceBuiltin, 0xd2df_92b6_98ea_623f),
            (Rule::OptionalFailure, 0xee27_b08b_3d3c_dca7),
            (Rule::WindowsLineEndings, 0x5dd4_bc45_2407_ba42),
        ];
        for (rule, model) in expected {
            assert_answer(&format!("B, {rule:?}"), pinning_test(rule), model);
        }
    }

    #[test]
    fn c_trace_has_every_stage_in_order() {
        let trace = trace_of_a_run();
        for (i, stage) in trace.iter().enumerate() {
            assert!(
                !trace[..i].contains(stage),
                "C: {stage:?} appears twice; every stage happens exactly once"
            );
        }
        let model = [
            0x3650_6566_ab04_eb59_u64,
            0xbf0b_b7c5_93ff_1a67,
            0x1868_9a6a_4f93_50a5,
            0xbd8a_74c7_aa6a_287a,
            0x2319_8008_24b3_cca5,
            0xb414_c5ef_19b9_7f6d,
            0xe5da_867f_ac76_4fb6,
            0x1a7c_8092_f561_5f5a,
            0xcb6d_eb8b_a33c_9435,
        ];
        let right = trace
            .iter()
            .zip(model)
            .enumerate()
            .filter(|(i, (stage, model))| digest(&format!("{} {stage:?}", i + 1)) == *model)
            .count();
        assert_eq!(
            right, 9,
            "C: {right} of 9 stages are at the position where the code does them"
        );
    }

    /// Runs the tasklane command line in this process.
    fn tasklane(source: &str, args: &[&str]) -> (u8, String) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = tasklane_cli::run_with(&args, &|_| Ok(source.to_string()), &mut out, &mut err);
        (code, String::from_utf8(out).unwrap())
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer. Read the
    /// fixture's README, manifests and `examples/site.lane` first; do not run tasklane
    /// for P2 to P5 until you have written a prediction.
    #[test]
    fn p_predictions() {
        // P1. How many crates does the fixture workspace have?
        let p1: usize = 3;
        let crates = std::fs::read_dir(package_dir().join("fixture/crates"))
            .expect("fixture/crates exists")
            .filter(|entry| entry.as_ref().is_ok_and(|entry| entry.path().is_dir()))
            .count();
        assert_eq!(p1, crates, "P1: the crates of the tasklane workspace");

        // P2. `tasklane -f examples/site.lane plan site` prints one task per line.
        // Write the names in that order, separated by one space.
        let p2: &str = "clean header pages stats site";
        let site = include_str!("../fixture/examples/site.lane");
        let (_, plan) = tasklane(site, &["plan", "site"]);
        assert_eq!(
            p2,
            plan.split_whitespace().collect::<Vec<_>>().join(" "),
            "P2: the order in which `run site` runs the tasks"
        );

        // P3. How many lines does `tasklane -f examples/site.lane list` print?
        let p3: usize = 4;
        let (_, list) = tasklane(site, &["list"]);
        assert_eq!(p3, list.lines().count(), "P3: the tasks that `list` shows");

        // P4. The exit code of `tasklane run a` for this Lanefile:
        //     task a : b
        //     task b : a
        let p4: u8 = 3;
        let (code, _) = tasklane("task a : b\ntask b : a\n", &["run", "a"]);
        assert_eq!(p4, code, "P4: the exit code for a mistake in the Lanefile");

        // P5. `tasklane run top` without options, for the Lanefile below. Give the
        // status of a, b, c and top, each as "done", "failed" or "skipped".
        let source = "task a\n  flags optional\n  step fail first\n\
                      task b : a\n  step fail second\n\
                      task c\n  step echo third\n\
                      task top : b c\n";
        let p5: [&str; 4] = ["failed", "failed", "skipped", "skipped"];
        let file = tasklane_core::parse(source).unwrap();
        let registry = tasklane_core::Registry::with_builtins();
        let mut ws = tasklane_core::Workspace::new();
        let report = tasklane_core::Runner::new(&registry)
            .run(&file, "top", &tasklane_core::Vars::new(), &mut ws)
            .unwrap();
        let actual: Vec<String> = report
            .results()
            .iter()
            .map(|result| format!("{:?}", result.status).to_lowercase())
            .collect();
        assert_eq!(
            p5.to_vec(),
            actual,
            "P5: what an optional failure and an ordinary failure do to later tasks"
        );
    }
}
