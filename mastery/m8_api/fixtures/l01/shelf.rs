//! shelf 1.0.0: a small stock list. You do not need to edit this file.
//!
//! This is the published version 1.0.0 of a library crate. Lesson 01 applies
//! one change at a time to this text and compiles `app.rs` against the result.

/// One kind of thing on the shelf.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub qty: u32,
}

/// How much of an item is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Out,
    Low,
    Plenty,
}

/// What a call changed.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Added { name: String },
    Taken { name: String, left: u32 },
}

/// A stock list.
#[derive(Debug, Default)]
pub struct Shelf {
    items: Vec<Item>,
}

impl Shelf {
    /// An empty shelf.
    pub fn new() -> Shelf {
        Shelf::default()
    }

    /// Puts `qty` more pieces of `name` on the shelf.
    pub fn add(&mut self, name: &str, qty: u32) -> Event {
        match self.items.iter_mut().find(|item| item.name == name) {
            Some(item) => item.qty += qty,
            None => self.items.push(Item {
                name: name.to_string(),
                qty,
            }),
        }
        Event::Added {
            name: name.to_string(),
        }
    }

    /// Takes `qty` pieces of `name`. Returns `None`, and changes nothing, when
    /// the shelf does not hold that many.
    pub fn take(&mut self, name: &str, qty: u32) -> Option<Event> {
        let item = self.items.iter_mut().find(|item| item.name == name)?;
        item.qty = item.qty.checked_sub(qty)?;
        Some(Event::Taken {
            name: name.to_string(),
            left: item.qty,
        })
    }

    /// Whether the shelf has no items at all.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, in the order in which they were first added.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The names of all items, sorted alphabetically.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.items.iter().map(|item| item.name.clone()).collect();
        names.sort();
        names
    }

    /// How much of `name` is left: `Out` for 0 or an unknown name, `Low` for 1
    /// to 4 pieces, `Plenty` for 5 or more.
    pub fn level(&self, name: &str) -> Level {
        match self.items.iter().find(|item| item.name == name) {
            None => Level::Out,
            Some(item) if item.qty == 0 => Level::Out,
            Some(item) if item.qty < 5 => Level::Low,
            Some(_) => Level::Plenty,
        }
    }
}

/// The number of pieces in `items`.
pub fn total(items: &[Item]) -> u32 {
    items.iter().map(|item| item.qty).sum()
}
