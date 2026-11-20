//! Prints the Fletcher-16 checksum of "abcde" as computed by the linked C
//! library. Lesson 02's tests run this before and after changing fletch.c.

fn main() {
    println!("{:04x}", checker::checksum16(b"abcde"));
}
