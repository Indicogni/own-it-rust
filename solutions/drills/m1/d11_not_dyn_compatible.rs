//! DRILL M1-11 · A trait that cannot be a trait object
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0038
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which slot would `scaled` need in the vtable, and what would it return?

// FIX: a method that returns `Self` by value cannot be called on a
// `dyn Shape`: the caller does not know the size of the result. One such
// method makes the whole trait unusable as a trait object. `where Self: Sized`
// removes the method from the vtable; it stays available on concrete types.
trait Shape {
    fn area(&self) -> f64;
    fn scaled(&self, factor: f64) -> Self
    where
        Self: Sized;
}

struct Square(f64);

impl Shape for Square {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
    fn scaled(&self, factor: f64) -> Self {
        Square(self.0 * factor)
    }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Square(2.0))]; // KEEP
    assert_eq!(shapes[0].area(), 4.0); // KEEP
    assert_eq!(Square(1.0).scaled(3.0).area(), 9.0); // KEEP
}
