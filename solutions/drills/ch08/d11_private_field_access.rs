//! DRILL 08-11 · Reaching past the constructor
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch08`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which code is allowed to touch a field that has no pub?

mod inventory {
    pub const CAPACITY: usize = 2;

    #[derive(Default)]
    pub struct Bin {
        items: Vec<String>,
    }

    impl Bin {
        pub fn add(&mut self, item: &str) -> bool {
            if self.items.len() >= CAPACITY {
                return false;
            }
            self.items.push(item.to_string());
            true
        }

        pub fn len(&self) -> usize {
            self.items.len()
        }
    }
}

fn main() {
    let mut bin = inventory::Bin::default();
    bin.add("bolt");
    // FIX: use the method the module offers. It also enforces the limit.
    bin.add("nut");
    assert!(!bin.add("washer")); // KEEP
    assert_eq!(bin.len(), inventory::CAPACITY); // KEEP
}
