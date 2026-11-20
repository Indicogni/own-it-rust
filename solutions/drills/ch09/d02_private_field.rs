//! DRILL 09-02 · Private fields outside the module
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0451
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who may build a value whose fields are private?

mod shop {
    pub struct Price {
        cents: u32,
    }

    impl Price {
        // FIX: the field stays private. Code outside the module builds a
        // Price through this constructor, the place where checks would go.
        pub fn new(cents: u32) -> Price {
            Price { cents }
        }

        pub fn cents(&self) -> u32 {
            self.cents
        }
    }
}

fn main() {
    let price = shop::Price::new(250);
    assert_eq!(price.cents(), 250); // KEEP
}
