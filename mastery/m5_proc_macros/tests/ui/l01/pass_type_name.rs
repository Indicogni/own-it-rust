//! Lesson 01, tasks A and C: `#[derive(TypeName)]` on every kind of item.
//! This program must compile and run.

use m5_proc_macros::TypeName;

/// A doc comment reaches the macro as the attribute `#[doc = "..."]`.
#[derive(TypeName)]
#[allow(dead_code)]
pub struct Job {
    id: u32,
}

#[derive(Debug, TypeName)]
#[allow(dead_code)]
pub(crate) enum Level {
    Low,
    High,
}

#[derive(TypeName)]
#[allow(dead_code)]
union Bits {
    int: u32,
    float: f32,
}

#[derive(TypeName)]
struct Unit;

#[derive(TypeName)]
#[allow(dead_code)]
struct Pair(u8, u8);

mod inner {
    // The derive is named by its full path here, and no trait is imported:
    // the generated code must not depend on what the user's module imports.
    #[derive(m5_proc_macros::TypeName)]
    pub(in crate::inner) struct Deep;

    pub fn name() -> &'static str {
        <Deep as m5_proc_macros::TypeName>::TYPE_NAME
    }
}

fn main() {
    assert_eq!(Job::TYPE_NAME, "Job");
    assert_eq!(Level::TYPE_NAME, "Level");
    assert_eq!(Bits::TYPE_NAME, "Bits");
    assert_eq!(Unit::TYPE_NAME, "Unit");
    assert_eq!(Pair::TYPE_NAME, "Pair");
    assert_eq!(inner::name(), "Deep");
}
