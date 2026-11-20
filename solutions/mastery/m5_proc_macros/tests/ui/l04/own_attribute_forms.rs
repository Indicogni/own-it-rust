//! Lesson 04, task D (model answer): a case that the supplied files do not
//! cover. Three ways to write the `label` attribute wrongly, and a default
//! label that collides with an explicit one written earlier.
//! This program must NOT compile. The expected compiler output is in
//! `own_attribute_forms.stderr`.

use m5_proc_macros::Label;

#[derive(Label)]
enum Shape {
    #[label]
    Circle,
    #[label = "sq"]
    Square,
    #[label("tri", "angle")]
    Triangle,
    #[label("Line")]
    Segment,
    Line,
}

fn main() {}
