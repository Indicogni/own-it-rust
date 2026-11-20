//! LESSON 07 — Enums and match
//! Run: cargo test --test chapter02 l07_
//! Read first: "Lesson 07" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A `match` has two arms that both fit the value. Which one runs?
//! 2. What does the compiler do when a `match` forgets one variant?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain what "a match must be exhaustive" means, and
//! what you gain when you list every variant and do not write `_`.
//!
//! COMPILER / TRANSFER LAB (optional): After you finish task A, add a fourth
//! variant `Blinking` to `Light` and run `cargo check --test chapter02`. Read
//! the error: which function does it name, and which pattern is missing?
//! Remove the variant again.

/// A traffic light. The `derive` line lets tests print and compare lights.
#[derive(Debug, PartialEq)]
enum Light {
    Red,
    Yellow,
    Green,
}

/// A flat shape. Two of the variants carry numbers.
#[derive(Debug, PartialEq)]
enum Shape {
    /// A square with this side length.
    Square(u32),
    /// A rectangle with this width and this height.
    Rectangle(u32, u32),
    /// A single point. It has no size.
    Dot,
}

/// A. Return the light that comes after `light`:
/// red is followed by green, green by yellow, yellow by red.
fn next(light: Light) -> Light {
    todo!("A: a match on `light` with one arm per variant")
}

/// B. Return how many seconds a driver waits at this light: 30 at red,
/// 5 at yellow, 0 at green.
fn wait_seconds(light: Light) -> u32 {
    todo!("B: one arm per variant")
}

/// C. Return the area of the shape. A dot has the area 0.
fn area(shape: Shape) -> u32 {
    todo!("C: take the numbers out of the variants with patterns")
}

/// D. Describe a number of guests: 0 is "none", 1 to 9 is "few",
/// 10 to 99 is "many", and anything larger is "crowd".
fn size_label(guests: u32) -> &'static str {
    todo!("D: match with a value, two ranges and `_`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_lights_follow_each_other() {
        assert_eq!(next(Light::Red), Light::Green);
        assert_eq!(next(Light::Green), Light::Yellow);
        assert_eq!(next(Light::Yellow), Light::Red);
        assert_eq!(
            next(next(next(Light::Red))),
            Light::Red,
            "three steps lead back to the start"
        );
    }

    #[test]
    fn b_wait_seconds() {
        assert_eq!(wait_seconds(Light::Red), 30);
        assert_eq!(wait_seconds(Light::Yellow), 5);
        assert_eq!(wait_seconds(Light::Green), 0);
    }

    #[test]
    fn c_area() {
        assert_eq!(area(Shape::Square(3)), 9);
        assert_eq!(
            area(Shape::Rectangle(2, 5)),
            10,
            "a rectangle's area is width times height"
        );
        assert_eq!(area(Shape::Dot), 0);
        assert_eq!(area(Shape::Square(0)), 0);
    }

    #[test]
    fn d_size_label() {
        assert_eq!(size_label(0), "none");
        assert_eq!(size_label(1), "few");
        assert_eq!(size_label(9), "few", "9 is the last number of `few`");
        assert_eq!(size_label(10), "many", "10 is the first number of `many`");
        assert_eq!(size_label(99), "many");
        assert_eq!(size_label(100), "crowd");
        assert_eq!(size_label(u32::MAX), "crowd");
    }

    /// Used by prediction P2.
    fn place(point: (i32, i32)) -> &'static str {
        match point {
            (0, 0) => "origin",
            (_, 0) => "x axis",
            (0, _) => "y axis",
            _ => "plane",
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Two arms fit the number 5. Which text does the match give?
        let n = 5;
        #[allow(unreachable_patterns)]
        let label = match n {
            1..=9 => "digit",
            5 => "five",
            _ => "other",
        };
        let p1: &str = guess();
        assert_eq!(
            p1, label,
            "P1: the arm chosen for 5 (left: your prediction, right: actual)"
        );

        // P2. A match on a pair. `_` fits any value. Read `place` above.
        // Answer with three texts in square brackets: `["...", "...", "..."]`.
        let p2: [&str; 3] = guess();
        assert_eq!(
            p2,
            [place((0, 3)), place((2, 0)), place((0, 0))],
            "P2: place((0, 3)), place((2, 0)), place((0, 0)) \
             (left: your prediction, right: actual)"
        );

        // P3. Patterns take the numbers out of a variant.
        let number = match Shape::Rectangle(2, 5) {
            Shape::Square(side) => side,
            Shape::Rectangle(width, height) => width + height,
            Shape::Dot => 0,
        };
        let p3: u32 = guess();
        assert_eq!(
            p3, number,
            "P3: the value of the match for Rectangle(2, 5) (left: your prediction, right: actual)"
        );
    }
}
