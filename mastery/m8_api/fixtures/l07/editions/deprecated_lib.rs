//! Lesson 07, prediction P4: a library that has deprecated a function.
//! You do not need to edit this file.

#[deprecated(since = "2.1.0", note = "use `area` instead")]
pub fn get_area(width: u32, height: u32) -> u32 {
    area(width, height)
}

pub fn area(width: u32, height: u32) -> u32 {
    width * height
}
