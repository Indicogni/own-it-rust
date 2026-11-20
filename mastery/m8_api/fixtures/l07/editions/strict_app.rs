//! Lesson 07, prediction P4: the same program in a project that treats
//! warnings as errors. You do not need to edit this file.

#![deny(warnings)]

fn main() {
    assert_eq!(deprecated_lib::get_area(3, 4), 12);
}
