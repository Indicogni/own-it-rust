//! Typed access to memory-mapped device registers.
//!
//! A device is described by a `#[repr(C)]` struct whose fields sit at the
//! register offsets from the data sheet. An [`Mmio`] is the address of one
//! such block. Registers are read and written with the two macros below,
//! which never create a Rust reference to device memory: a reference would
//! allow the compiler to add, remove or reorder accesses, and a device
//! register can change or cause an action on every single access.

/// The address of one register block of type `R`.
pub struct Mmio<R> {
    base: *mut R,
}

// SAFETY: an `Mmio` is an address, not memory that Rust owns. All access
// goes through volatile reads and writes on raw pointers, so sharing it
// cannot create aliasing references or data races in the Rust sense. Whether
// the *device* tolerates interleaved register access is a question each
// driver answers for itself.
unsafe impl<R> Send for Mmio<R> {}
// SAFETY: as above.
unsafe impl<R> Sync for Mmio<R> {}

impl<R> Mmio<R> {
    /// # Safety
    ///
    /// `addr` must be the address of a device whose registers match the
    /// layout of `R`, the block must stay mapped for as long as the value
    /// exists, and no Rust object may live in that address range.
    pub const unsafe fn new(addr: usize) -> Self {
        Self {
            base: core::ptr::with_exposed_provenance_mut(addr),
        }
    }

    pub const fn as_ptr(&self) -> *mut R {
        self.base
    }
}

/// `read_reg!(mmio, field)` or `read_reg!(mmio, array_field[index])`:
/// one volatile read of a register.
macro_rules! read_reg {
    ($mmio:expr, $field:ident $([$index:expr])?) => {{
        let block = $crate::mmio::Mmio::as_ptr(&$mmio);
        // SAFETY: `Mmio::new` promised that `block` addresses a live device
        // register block with this layout. `&raw const` computes the
        // register address without creating a reference, array indices are
        // bounds-checked, and the read is volatile.
        unsafe { (&raw const (*block).$field $([$index])?).read_volatile() }
    }};
}

/// `write_reg!(mmio, field, value)` or
/// `write_reg!(mmio, array_field[index], value)`: one volatile write.
macro_rules! write_reg {
    ($mmio:expr, $field:ident $([$index:expr])?, $value:expr) => {{
        let block = $crate::mmio::Mmio::as_ptr(&$mmio);
        let value = $value;
        // SAFETY: as for `read_reg!`; the write is volatile and goes to a
        // register of a device this driver owns.
        unsafe { (&raw mut (*block).$field $([$index])?).write_volatile(value) }
    }};
}

pub(crate) use {read_reg, write_reg};
