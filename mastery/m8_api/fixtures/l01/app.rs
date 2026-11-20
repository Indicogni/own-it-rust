//! A program written against shelf 1.0.0. You do not need to edit this file.
//!
//! It uses the library in the ways that ordinary programs do. Every line is
//! correct for version 1.0.0. Lesson 01 asks what happens to this program
//! when the library changes underneath it.

use shelf::{Event, Item, Level, Shelf};

/// Something this program adds to `Shelf` for its own use.
trait Report {
    fn count(&self) -> u32;
}

impl Report for Shelf {
    fn count(&self) -> u32 {
        shelf::total(self.items())
    }
}

/// The text this program shows for a stock level.
fn advice(level: Level) -> &'static str {
    match level {
        Level::Out => "order now",
        Level::Low => "order soon",
        Level::Plenty => "fine",
    }
}

/// A line for the log.
fn log_line(event: &Event) -> String {
    match event {
        Event::Added { name } => format!("+ {name}"),
        Event::Taken { name, left } => format!("- {name} ({left} left)"),
    }
}

fn main() {
    let mut shelf = Shelf::new();
    let added = shelf.add("nut", 12);
    shelf.add("bolt", 3);
    assert_eq!(log_line(&added), "+ nut");

    let taken = shelf.take("nut", 2).expect("there are 12 nuts");
    assert_eq!(log_line(&taken), "- nut (10 left)");
    assert_eq!(shelf.take("bolt", 9), None);

    // A value of a library type, built with a struct expression.
    let spare = Item {
        name: String::from("washer"),
        qty: 40,
    };
    assert_eq!(shelf::total(&[spare.clone(), spare]), 80);

    // `Level` is used twice: the second use needs a copy.
    let level = shelf.level("bolt");
    assert_eq!(advice(level), "order soon");
    assert_eq!(level, Level::Low);

    // This program relies on the documented order of `names`.
    let names = shelf.names();
    assert_eq!(names.join(" "), "bolt nut");

    // The trait method that this program added.
    let pieces: u32 = shelf.count();
    assert_eq!(pieces, 13);
}
