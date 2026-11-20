//! `Arena<N>`: a bump allocator over `N` inline bytes (SPEC section 6).

use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};
use core::slice;

use crate::CapacityError;

/// The bytes of an arena.
///
/// The alignment makes every placement with an alignment of at most 16 a
/// pure function of the allocations made so far (SPEC 6.2): it does not
/// depend on where the arena happens to be in memory.
///
/// The `UnsafeCell` is what allows writing through a shared reference to the
/// arena. The bytes are `MaybeUninit` because they start uninitialised and
/// because padding inside stored values stays uninitialised.
#[repr(C, align(16))]
struct Storage<const N: usize>(UnsafeCell<[MaybeUninit<u8>; N]>);

/// A bump allocator with room for `N` bytes, stored inside itself.
///
/// ```text
/// Arena<32> after alloc(1u8), alloc(2u32), alloc(3u16):
///
/// offset   0    1         4              8        10                 32
///          +----+---------+--------------+---------+------------------+
///          | u8 | padding |     u32      |   u16   |       free       |
///          +----+---------+--------------+---------+------------------+
///                                                  ^ used = 10
/// ```
///
/// # Invariant
///
/// `used <= N`. Every reference the arena has handed out since the last
/// `reset` points to bytes below `used` (or is zero-sized), and no two of
/// them overlap. The bytes from `used` on are referenced by nobody.
///
/// # Why `&mut` from `&self` is sound
///
/// * The arena never creates a reference to its storage as a whole, only
///   raw pointers (`UnsafeCell::get`). So a new allocation does not disturb
///   the references to earlier ones.
/// * Each call hands out bytes that no earlier call handed out, so every
///   returned `&mut` is unique.
/// * The returned lifetime is the lifetime of the `&self` borrow. While any
///   reference lives, the arena is borrowed: it cannot be moved, dropped or
///   `reset` (which needs `&mut self`).
/// * `Cell` and `UnsafeCell` make the arena `!Sync`, so two threads cannot
///   race on `used`.
pub struct Arena<const N: usize> {
    storage: Storage<N>,
    used: Cell<usize>,
}

impl<const N: usize> Arena<N> {
    /// Creates an empty arena.
    pub const fn new() -> Self {
        Self {
            storage: Storage(UnsafeCell::new([MaybeUninit::uninit(); N])),
            used: Cell::new(0),
        }
    }

    /// The size of the storage in bytes: `N`.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The number of bytes that are taken, padding included.
    pub fn used(&self) -> usize {
        self.used.get()
    }

    /// The number of bytes that are still free: `capacity() - used()`.
    pub fn remaining(&self) -> usize {
        N - self.used.get()
    }

    /// Reserves room for `layout` and returns a pointer to it, or `None` if
    /// it does not fit. This is the only function that moves `used`.
    ///
    /// The pointer is aligned for `layout`. If the size is not zero, it is
    /// valid for reads and writes of `layout.size()` bytes that no other
    /// allocation since the last `reset` shares.
    fn reserve(&self, layout: Layout) -> Option<NonNull<u8>> {
        if layout.size() == 0 {
            // A zero-sized value needs no memory, only an address that is
            // aligned and not null. The alignment itself is such an address.
            return NonNull::new(ptr::without_provenance_mut(layout.align()));
        }
        // A raw pointer to the first byte. No reference to the storage is
        // created here, which keeps earlier allocations valid.
        let base = self.storage.0.get().cast::<u8>();
        let used = self.used.get();
        // The padding is computed from the real address, so alignments
        // above 16 work too. For alignments up to 16 the result is the same
        // as rounding the offset up, because `base` is 16-byte aligned.
        // `addr + used` cannot overflow: it is inside or just past the
        // storage. `align` is a power of two, so `align - 1` is a bit mask.
        let addr = base.addr() + used;
        let padding = addr.wrapping_neg() & (layout.align() - 1);
        let start = used.checked_add(padding)?;
        let end = start.checked_add(layout.size())?;
        if end > N {
            return None;
        }
        self.used.set(end);
        // SAFETY: `start < end <= N`, so the result points into the storage
        // that `base` points to.
        NonNull::new(unsafe { base.add(start) })
    }

    /// Moves `value` into the arena and returns a reference to it, or gives
    /// the value back if there is no room.
    ///
    /// The arena never runs the destructor of a stored value.
    // Clippy denies `&mut` made from `&self` by default, with good reason.
    // An arena is the exception; the type's documentation carries the proof.
    #[expect(
        clippy::mut_from_ref,
        reason = "sound for an arena; see the type's documentation"
    )]
    pub fn alloc<T>(&self, value: T) -> Result<&mut T, CapacityError<T>> {
        let Some(place) = self.reserve(Layout::new::<T>()) else {
            return Err(CapacityError::new(value));
        };
        let place = place.cast::<T>();
        // SAFETY: `reserve` returned a pointer that is aligned for `T` and
        // valid for writes of `size_of::<T>()` bytes (or `T` is zero-sized).
        unsafe { place.write(value) };
        // SAFETY: the place now holds a `T`. No other reference to these
        // bytes exists (invariant), and the reference cannot outlive the
        // borrow of the arena.
        Ok(unsafe { &mut *place.as_ptr() })
    }

    /// Copies a slice into the arena. Returns `None` if there is no room.
    #[expect(
        clippy::mut_from_ref,
        reason = "sound for an arena; see the type's documentation"
    )]
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> Option<&mut [T]> {
        let place = self.reserve(Layout::for_value(src))?.cast::<T>();
        // SAFETY: `place` is aligned for `T` and valid for writes of
        // `src.len()` elements. It cannot overlap `src`: no reference to
        // these bytes exists (invariant).
        unsafe { ptr::copy_nonoverlapping(src.as_ptr(), place.as_ptr(), src.len()) };
        // SAFETY: all `src.len()` elements are initialised now, and nothing
        // else refers to them.
        Some(unsafe { slice::from_raw_parts_mut(place.as_ptr(), src.len()) })
    }

    /// Copies a string slice into the arena. Returns `None` if there is no
    /// room.
    #[expect(
        clippy::mut_from_ref,
        reason = "sound for an arena; see the type's documentation"
    )]
    pub fn alloc_str(&self, src: &str) -> Option<&mut str> {
        let bytes = self.alloc_slice_copy(src.as_bytes())?;
        // SAFETY: the bytes are a copy of a `str`, so they are valid UTF-8.
        Some(unsafe { core::str::from_utf8_unchecked_mut(bytes) })
    }

    /// Allocates a slice of `len` elements; element `i` is `fill(i)`.
    /// Returns `None`, without calling `fill`, if there is no room.
    ///
    /// `fill` may allocate from this arena. If it panics, the elements made
    /// so far are not dropped and their bytes stay taken.
    #[expect(
        clippy::mut_from_ref,
        reason = "sound for an arena; see the type's documentation"
    )]
    pub fn alloc_slice_fill_with<T, F>(&self, len: usize, mut fill: F) -> Option<&mut [T]>
    where
        F: FnMut(usize) -> T,
    {
        // `Layout::array` fails if `len * size_of::<T>()` overflows.
        let layout = Layout::array::<T>(len).ok()?;
        // The room is reserved *before* `fill` runs. `fill` can therefore
        // call back into this arena: what it allocates lies behind the
        // slice, not inside it.
        let place = self.reserve(layout)?.cast::<T>();
        for index in 0..len {
            let value = fill(index);
            // SAFETY: `index < len`, so the element is inside the reserved
            // room, which is aligned for `T`.
            unsafe { place.add(index).write(value) };
        }
        // SAFETY: the loop initialised all `len` elements, and nothing else
        // refers to them. (If `fill` panicked, this line is not reached and
        // no reference to the partly filled room ever exists.)
        Some(unsafe { slice::from_raw_parts_mut(place.as_ptr(), len) })
    }

    /// Makes the whole storage free again. No destructor runs.
    ///
    /// Needs `&mut self`: the borrow checker proves that no reference into
    /// the arena is still alive.
    pub fn reset(&mut self) {
        self.used.set(0);
    }
}

impl<const N: usize> Default for Arena<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Debug for Arena<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("capacity", &N)
            .field("used", &self.used.get())
            .finish()
    }
}
