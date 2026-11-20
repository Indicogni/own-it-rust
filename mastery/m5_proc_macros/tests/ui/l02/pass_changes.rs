//! Lesson 02, tasks A and B: `#[derive(Changes)]`.
//! This program must compile and run.

use m5_proc_macros::Changes;

/// `Vec<String>` is not `Copy`, and `Stamp` is not `Clone`: the generated code
/// must compare the fields where they are.
#[derive(PartialEq)]
struct Stamp(u64);

#[derive(Changes)]
struct Job {
    id: u32,
    name: String,
    tags: Vec<String>,
    stamp: Stamp,
    r#type: u8,
}

#[derive(Changes)]
struct Empty {}

mod elsewhere {
    // No import of the trait, and a local type with the name `Vec`: the
    // generated code must still compile here.
    #[allow(dead_code)]
    pub struct Vec;

    #[derive(m5_proc_macros::Changes)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }
}

fn job() -> Job {
    Job {
        id: 1,
        name: "build".into(),
        tags: vec!["ci".into()],
        stamp: Stamp(10),
        r#type: 0,
    }
}

fn main() {
    assert!(
        job().changes(&job()).is_empty(),
        "equal values have no changes"
    );

    let newer = Job {
        name: "deploy".into(),
        stamp: Stamp(11),
        r#type: 1,
        ..job()
    };
    assert_eq!(
        job().changes(&newer),
        ["name", "stamp", "type"],
        "the changed fields, in declaration order"
    );
    assert!(Empty {}.changes(&Empty {}).is_empty());

    let a = elsewhere::Point { x: 1, y: 2 };
    let b = elsewhere::Point { x: 1, y: 3 };
    assert_eq!(a.changes(&b), ["y"]);
}
