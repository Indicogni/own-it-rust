//! LESSON 06, task C: a program with a bug. `cargo run --bin stockroom` panics.
//!
//! The stockroom starts with five items, takes one delivery, drops discontinued
//! items, and then ships three orders. Find out why it panics, and repair the cause.
//! The panic message names the line where the damage became visible. The cause is
//! somewhere else. LABS.md, lab 6.1, walks through the same hunt in a debugger.
//!
//! The expected output after the repair is in `tests/l06_process.rs`.

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Item {
    sku: &'static str,
    quantity: u32,
    discontinued: bool,
}

struct Stockroom {
    items: Vec<Item>,
    /// Position of each SKU in `items`, so lookups do not scan the vector.
    index: HashMap<&'static str, usize>,
}

impl Stockroom {
    fn new(items: Vec<Item>) -> Self {
        let mut stockroom = Self {
            items,
            index: HashMap::new(),
        };
        stockroom.rebuild_index();
        stockroom
    }

    /// The index stores positions, so it must be rebuilt whenever positions change.
    fn rebuild_index(&mut self) {
        self.index = self
            .items
            .iter()
            .enumerate()
            .map(|(position, item)| (item.sku, position))
            .collect();
    }

    fn receive(&mut self, sku: &str, quantity: u32) {
        let position = self.index[sku];
        self.items[position].quantity += quantity;
    }

    /// Removes the items that are no longer sold.
    fn drop_discontinued(&mut self) {
        self.items.retain(|item| !item.discontinued);
        self.rebuild_index();
    }

    fn ship(&mut self, sku: &str, quantity: u32) {
        let position = self.index[sku];
        let item = &mut self.items[position];
        item.quantity -= quantity;
    }

    fn report(&self) -> String {
        let mut lines: Vec<String> = self
            .items
            .iter()
            .map(|item| format!("{}: {}", item.sku, item.quantity))
            .collect();
        lines.sort();
        lines.join("\n")
    }
}

fn main() {
    let item = |sku, quantity, discontinued| Item {
        sku,
        quantity,
        discontinued,
    };
    let mut stockroom = Stockroom::new(vec![
        item("anvil", 2, false),
        item("bolt", 500, false),
        item("crank", 0, true),
        item("gear", 40, false),
        item("hinge", 12, false),
    ]);
    stockroom.receive("hinge", 30);
    stockroom.drop_discontinued();
    for (sku, quantity) in [("bolt", 120), ("gear", 25), ("hinge", 20)] {
        stockroom.ship(sku, quantity);
    }
    println!("{}", stockroom.report());
}
