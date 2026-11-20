//@ expect: run
//@ why: a stage closure must be allowed to borrow local data and need not be Send or Sync; the pipeline only stores it and calls it on this thread.
use m1_types::c01_code_review::Pipeline;
use std::cell::Cell;

fn main() {
    let prefix = String::from(">> ");
    let calls = Cell::new(0);
    let stage_name = String::from("count and prefix");

    let pipeline = Pipeline::new()
        .stage(&stage_name, |text| {
            calls.set(calls.get() + 1);
            format!("{prefix}{text}")
        })
        .stage("shout", |text| text.to_uppercase());

    assert_eq!(pipeline.run("hello"), ">> HELLO");
    assert_eq!(pipeline.run("again"), ">> AGAIN");
    assert_eq!(calls.get(), 2);
    let names: Vec<&str> = pipeline
        .names()
        .map(|name| -> &str { name.as_ref() })
        .collect();
    assert_eq!(names, ["count and prefix", "shout"]);
}
