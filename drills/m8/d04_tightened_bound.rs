//! DRILL M8-04 · A method asks for more than it uses
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does the body of `get` use the bound that its signature demands?

/// Stands for version 2.3.0 of a library crate that you maintain.
mod cache {
    use std::collections::HashMap;
    use std::hash::Hash;

    pub struct Cache<K, V> {
        entries: HashMap<K, V>,
    }

    impl<K: Hash + Eq, V> Cache<K, V> {
        pub fn new() -> Self {
            Cache { entries: HashMap::new() }
        }

        pub fn insert(&mut self, key: K, value: V) {
            self.entries.insert(key, value);
        }

        /// Since 2.3.0 with `V: Clone`: "we may want to return copies one day".
        pub fn get(&self, key: &K) -> Option<&V>
        where
            V: Clone,
        {
            self.entries.get(key)
        }
    }
}

/// A user's type. It owns a socket in real life, so it cannot be cloned.
struct Connection { // KEEP
    port: u16, // KEEP
} // KEEP

fn main() {
    let mut open = cache::Cache::new(); // KEEP
    open.insert("db", Connection { port: 5432 }); // KEEP
    assert_eq!(open.get(&"db").map(|connection| connection.port), Some(5432)); // KEEP
}
