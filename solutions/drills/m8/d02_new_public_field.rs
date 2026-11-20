//! DRILL M8-02 · A struct gets a public field
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a struct expression must name every field. Which syntax lets a user say "and the rest as in this other value"?

/// Stands for version 2.0.0 of a library crate. In 1.x, `Size` had two fields.
mod geometry {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Size {
        pub width: u32, // KEEP
        pub height: u32, // KEEP
        /// New in 2.0.0.
        pub depth: u32, // KEEP
    }

    impl Default for Size {
        /// One unit in every direction.
        fn default() -> Size {
            Size { width: 1, height: 1, depth: 1 }
        }
    }

    impl Size {
        pub fn volume(&self) -> u32 {
            self.width * self.height * self.depth
        }
    }
}

fn main() {
    // A user's code, written for 1.x. Repair it so that it also survives the
    // next public field.
    let size = geometry::Size { width: 3, height: 4, ..Default::default() };
    assert_eq!(size.volume(), 12); // KEEP
}
