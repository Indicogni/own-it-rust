//! The example Lanefiles must keep working: they are part of the documentation.

use tasklane_core::{Registry, Runner, Vars, Workspace, parse, plan};

const SITE: &str = include_str!("../../../examples/site.lane");

#[test]
fn site_example_builds() {
    let file = parse(SITE).unwrap();
    for name in file.task_names() {
        plan(&file, name).unwrap();
    }
    let registry = Registry::with_builtins();
    let mut ws = Workspace::new();
    let report = Runner::new(&registry)
        .run(&file, "site", &Vars::from_lanefile(&file), &mut ws)
        .unwrap();
    assert!(report.success(), "{}", report.summary());
    assert_eq!(
        ws.read("build/site.txt"),
        Some("== My Site ==\nWelcome to My Site.\nMade with tasklane.")
    );
    assert!(
        ws.log_lines()
            .contains(&"build/all.txt: 3 lines, 11 words".to_string()),
        "{:?}",
        ws.log_lines()
    );
}
