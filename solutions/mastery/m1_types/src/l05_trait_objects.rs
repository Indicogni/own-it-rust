//! LESSON 05 — Trait objects: vtables, dyn compatibility, upcasting, lifetimes
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l05_
//! Read first: "Lesson 05" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A `&dyn Shape` is two machine words. What is the second word for, and
//!    which three things must it lead to besides the trait's methods?
//! 2. `Box<dyn Label>` has no lifetime in it. May the boxed value hold a
//!    reference to a local variable? What about `&'a dyn Label`?
//! 3. Why can a method with a type parameter not be called through `dyn`?
//!
//! IMPLEMENT the tasks below. A and B have signatures. In C and E the given
//! types are a first attempt that you must change; D has no skeleton. The
//! programs in `tests/cases/l05/` are part of the contract. Add one edge case.
//! EXPLAIN afterward: Describe what a `Box<dyn Shape>` consists of at run
//! time, and what the compiler does for the upcast in task B. Then give the
//! rule for the default lifetime of `Box<dyn Trait>` and of `&'a dyn Trait`,
//! and the three reasons why the first `Codec` was not dyn compatible.
//!
//! COMPILER LAB (optional): In task B, write `shape.type_id()` and compare it
//! with `TypeId::of::<Rect>()`. It compiles and is always unequal. Find out
//! which type's `type_id` is called (prediction P3 is the same trap).

use std::any::Any;
use std::io::{self, Write};

/// A shape that can also be inspected as `dyn Any`: `Any` is a supertrait.
/// (This also means that every shape is `'static`: it holds no borrows.)
pub trait Shape: Any {
    fn area(&self) -> f64;
    fn name(&self) -> String;
}

#[derive(Debug, PartialEq)]
pub struct Circle {
    pub radius: f64,
}

#[derive(Debug, PartialEq)]
pub struct Rect {
    pub width: f64,
    pub height: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
    fn name(&self) -> String {
        "circle".to_string()
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.width * self.height
    }
    fn name(&self) -> String {
        "rect".to_string()
    }
}

/// A. Return the circle behind the trait object, or `None` if the shape is
/// something else. Use the supertrait: no extra method on `Shape` is needed.
pub fn as_circle(shape: &dyn Shape) -> Option<&Circle> {
    // Upcasting coercion: `&dyn Shape` to `&dyn Any`, its supertrait.
    let any: &dyn Any = shape;
    any.downcast_ref()
}

/// B. Turn the box into a `Box<Rect>` if the shape is a `Rect`. Otherwise give
/// the box back unchanged. Do not allocate and do not copy the shape.
pub fn into_rect(shape: Box<dyn Shape>) -> Result<Box<Rect>, Box<dyn Shape>> {
    // Check first: `Box<dyn Any>::downcast` returns a `Box<dyn Any>` on
    // failure, and that cannot be turned back into a `Box<dyn Shape>`.
    // `&*shape` matters: `shape.type_id()` would ask the Box, not the shape.
    let any: &dyn Any = &*shape;
    if !any.is::<Rect>() {
        return Err(shape);
    }
    let any: Box<dyn Any> = shape;
    Ok(any
        .downcast()
        .expect("the shape was checked to be a Rect above"))
}

/// Something that can be shown in a legend. Unlike `Shape`, it has no `Any`
/// supertrait, so a label may hold borrows.
pub trait Label {
    fn label(&self) -> String;
}

impl Label for String {
    fn label(&self) -> String {
        self.clone()
    }
}

/// A label that borrows its text.
pub struct Caption<'a>(pub &'a str);

impl Label for Caption<'_> {
    fn label(&self) -> String {
        format!("[{}]", self.0)
    }
}

/// Joins the labels with ", ". Used by task D; do not change it.
pub fn join_labels<L: Label>(items: &[L]) -> String {
    let labels: Vec<String> = items.iter().map(Label::label).collect();
    labels.join(", ")
}

/// C. A legend owns a list of labels of different types.
///
/// First attempt: it works for `String`, and it rejects a `Caption` that
/// borrows a local variable (`tests/cases/l05/legend_accepts_borrowing_labels.rs`).
/// Find the lifetime that the compiler filled in for you, and replace it with
/// one that the user of `Legend` chooses. `add` appends a label; `render`
/// joins all labels with " | ".
pub struct Legend<'a> {
    // `Box<dyn Label>` alone means `Box<dyn Label + 'static>`.
    entries: Vec<Box<dyn Label + 'a>>,
}

impl<'a> Legend<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Legend {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: impl Label + 'a) {
        self.entries.push(Box::new(entry));
    }

    pub fn render(&self) -> String {
        let labels: Vec<String> = self.entries.iter().map(|entry| entry.label()).collect();
        labels.join(" | ")
    }
}

// D. `join_labels` is generic over `L: Label`. A `Vec<Box<dyn Label>>` cannot
// be passed to it yet, because `Box<dyn Label>` itself is not a `Label`.
// Write ONE impl that makes `Box<L>` a `Label` for every label type `L`,
// including `dyn Label` itself. Checked by
// `tests/cases/l05/boxed_labels_are_labels.rs`.
// `?Sized` is the point: without it, `L` could not be `dyn Label`.
impl<L: Label + ?Sized> Label for Box<L> {
    fn label(&self) -> String {
        (**self).label()
    }
}

/// E. A text encoder.
///
/// First attempt: every codec type works when it is used directly, and
/// `Box<dyn Codec>` is rejected (`tests/cases/l05/codecs_in_one_list.rs`).
/// Change the trait, and the two impls below to match, so that:
/// - `name` and `encode_into` can be called on a `dyn Codec`;
/// - `twin` and `extension` stay available on concrete codec types;
/// - the unit tests in this file still compile unchanged.
pub trait Codec {
    fn name(&self) -> String;

    /// Writes the encoded form of `text` to `out`.
    // A type parameter would need one vtable entry per writer type. A
    // `&mut dyn Write` needs one entry, and callers can still pass `&mut Vec<u8>`.
    fn encode_into(&self, text: &str, out: &mut dyn Write) -> io::Result<()>;

    /// A second codec of the same type.
    // `where Self: Sized` takes the method out of the vtable: a `dyn Codec`
    // has no known `Self` to return by value.
    fn twin(&self) -> Self
    where
        Self: Sized;

    /// The file extension for this codec's output.
    fn extension() -> &'static str
    where
        Self: Sized;
}

/// Writes the text in ASCII upper case.
pub struct Upper;

/// Rotates ASCII letters by `shift` places.
pub struct Caesar {
    pub shift: u8,
}

impl Codec for Upper {
    fn name(&self) -> String {
        "upper".to_string()
    }

    fn encode_into(&self, text: &str, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(text.to_ascii_uppercase().as_bytes())
    }

    fn twin(&self) -> Self {
        Upper
    }

    fn extension() -> &'static str {
        "up"
    }
}

impl Codec for Caesar {
    fn name(&self) -> String {
        format!("caesar-{}", self.shift)
    }

    fn encode_into(&self, text: &str, out: &mut dyn Write) -> io::Result<()> {
        let rotated: Vec<u8> = text.bytes().map(|byte| rotate(byte, self.shift)).collect();
        out.write_all(&rotated)
    }

    fn twin(&self) -> Self {
        Caesar { shift: self.shift }
    }

    fn extension() -> &'static str {
        "rot"
    }
}

/// Rotates one ASCII letter by `shift` places and keeps every other byte.
fn rotate(byte: u8, shift: u8) -> u8 {
    let base = match byte {
        b'a'..=b'z' => b'a',
        b'A'..=b'Z' => b'A',
        _ => return byte,
    };
    base + (byte - base + shift % 26) % 26
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;
    use std::any::TypeId;

    /// A third shape, known only to the tests.
    struct Blob;

    impl Shape for Blob {
        fn area(&self) -> f64 {
            1.5
        }
        fn name(&self) -> String {
            "blob".to_string()
        }
    }

    #[test]
    fn a_circle_is_found_behind_the_trait_object() {
        let circle = Circle { radius: 2.0 };
        let rect = Rect {
            width: 2.0,
            height: 3.0,
        };
        let found = as_circle(&circle).expect("this shape is a circle");
        assert!(
            std::ptr::eq(found, &circle),
            "as_circle must return a reference to the same circle, not to a copy"
        );
        assert!(as_circle(&rect).is_none(), "a Rect is not a Circle");
        assert!(as_circle(&Blob).is_none(), "a Blob is not a Circle");
    }

    #[test]
    fn b_rect_box_is_recovered_without_copying() {
        let shape: Box<dyn Shape> = Box::new(Rect {
            width: 2.0,
            height: 3.0,
        });
        let address = &*shape as *const dyn Shape as *const u8;
        let (result, allocations) = count(|| into_rect(shape));
        let rect = match result {
            Ok(rect) => rect,
            Err(_) => panic!(
                "into_rect must return Ok for a Rect. If you compared type ids, \
                 check WHOSE type id you asked for"
            ),
        };
        assert_eq!(rect.area(), 6.0);
        assert!(
            std::ptr::eq(&*rect as *const Rect as *const u8, address),
            "into_rect must return the same heap allocation, not a new box"
        );
        assert_eq!(
            allocations, 0,
            "into_rect made {allocations} heap allocation(s); converting a box needs none"
        );
    }

    #[test]
    fn b_other_shapes_come_back_unchanged() {
        let shape: Box<dyn Shape> = Box::new(Circle { radius: 1.0 });
        let address = &*shape as *const dyn Shape as *const u8;
        let back = match into_rect(shape) {
            Ok(_) => panic!("a Circle must not be returned as a Rect"),
            Err(back) => back,
        };
        assert_eq!(back.name(), "circle");
        assert!(
            std::ptr::eq(&*back as *const dyn Shape as *const u8, address),
            "the box that is handed back must be the one that was passed in"
        );
    }

    #[test]
    fn c_legend_joins_owned_labels() {
        let mut legend = Legend::new();
        assert_eq!(legend.render(), "");
        legend.add(String::from("north"));
        legend.add(Caption("static text"));
        assert_eq!(legend.render(), "north | [static text]");
    }

    #[test]
    fn e_codecs_work_on_concrete_types() {
        let mut out = Vec::new();
        Upper
            .encode_into("abc", &mut out)
            .expect("writing to a Vec");
        let caesar = Caesar { shift: 1 };
        caesar
            .twin()
            .encode_into(" Hal", &mut out)
            .expect("writing to a Vec");
        assert_eq!(out, b"ABC Ibm");
        assert_eq!(Upper.twin().name(), "upper");
        assert_eq!(caesar.name(), "caesar-1");
        assert_eq!((Upper::extension(), Caesar::extension()), ("up", "rot"));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Sizes in machine words: (&Circle, &dyn Shape, Box<dyn Shape>, &[u8]).
        let word = size_of::<usize>();
        let p1: (usize, usize, usize, usize) = (1, 2, 2, 2);
        assert_eq!(
            p1,
            (
                size_of::<&Circle>() / word,
                size_of::<&dyn Shape>() / word,
                size_of::<Box<dyn Shape>>() / word,
                size_of::<&[u8]>() / word
            ),
            "P1: pointer sizes in words (left: your prediction, right: actual)"
        );

        // P2. `size_of_val` on a trait object reads the size from the vtable.
        // What does it return for each of these two `&dyn Shape`, in bytes?
        let shapes: [&dyn Shape; 2] = [
            &Circle { radius: 1.0 },
            &Rect {
                width: 1.0,
                height: 1.0,
            },
        ];
        let p2: (usize, usize) = (8, 16);
        assert_eq!(
            p2,
            (size_of_val(shapes[0]), size_of_val(shapes[1])),
            "P2: size of the value behind each trait object (left: your prediction, right: actual)"
        );

        // P3. A `u8` in a `Box<dyn Any>`. Which of these two comparisons are
        // true? (`boxed.type_id() == u8's id`, `(*boxed).type_id() == u8's id`)
        let boxed: Box<dyn Any> = Box::new(5_u8);
        // The first call is the trap this prediction is about.
        #[allow(clippy::type_id_on_box)]
        let on_box = boxed.type_id() == TypeId::of::<u8>();
        let on_value = (*boxed).type_id() == TypeId::of::<u8>();
        let p3: (bool, bool) = (false, true);
        assert_eq!(
            p3,
            (on_box, on_value),
            "P3: which type_id call sees the u8 (left: your prediction, right: actual)"
        );
    }
}
