//! A generational arena: the entity store.
//!
//! Entities refer to each other with an `Id` (a slot number plus the
//! generation the slot had when the id was issued) instead of with
//! references. Removing an entity bumps the generation of its slot, so every
//! old id for it quietly stops resolving. Nothing dangles and nothing needs
//! `Rc<RefCell<..>>`.

/// A handle to a value in a [`Store`]. Plain data: `Copy`, comparable, and
/// free to keep after the value is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Id {
    index: u32,
    generation: u32,
}

#[derive(Clone)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Values in a `Vec` of slots, with a free list for reuse.
#[derive(Clone)]
pub(crate) struct Store<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Store<T> {
    pub(crate) fn with_capacity(n: usize) -> Store<T> {
        Store {
            slots: Vec::with_capacity(n),
            // Every slot could be free at once, so reserve that much now and
            // `remove` never has to grow the list in the middle of a frame.
            free: Vec::with_capacity(n),
        }
    }

    pub(crate) fn insert(&mut self, value: T) -> Id {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            Id {
                index,
                generation: slot.generation,
            }
        } else {
            let index = u32::try_from(self.slots.len()).expect("fewer than 2^32 entities");
            self.slots.push(Slot {
                generation: 0,
                value: Some(value),
            });
            Id {
                index,
                generation: 0,
            }
        }
    }

    /// Takes the value out. Later lookups with the same id return `None`,
    /// even after the slot has been given to a new value.
    pub(crate) fn remove(&mut self, id: Id) -> Option<T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation || slot.value.is_none() {
            return None;
        }
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        slot.value.take()
    }

    pub(crate) fn get(&self, id: Id) -> Option<&T> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    pub(crate) fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Store;

    #[test]
    fn a_stale_id_does_not_reach_the_new_tenant_of_its_slot() {
        let mut store = Store::with_capacity(2);
        let a = store.insert("a");
        let b = store.insert("b");
        assert_eq!(store.remove(a), Some("a"));
        assert_eq!(store.get(a), None);
        let c = store.insert("c");
        assert_eq!(store.get(c), Some(&"c"));
        assert_eq!(store.get(a), None, "old id, same slot, newer generation");
        assert_eq!(store.remove(a), None);
        assert_eq!(store.get(b), Some(&"b"));
        *store.get_mut(b).unwrap() = "B";
        assert_eq!(store.get(b), Some(&"B"));
    }
}
