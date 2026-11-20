//! `kcore`: every part of your kernel that does not need hardware.
//!
//! This file is the *facade* that SPEC.md section 5 fixes: the host tests in
//! `../tests/host_*.rs` use exactly these names and signatures, and nothing
//! else. Everything behind the facade is yours to design. Add fields, add
//! private items, split the crate into modules and re-export the names from
//! here; only the public signatures below must stay.
//!
//! The crate must stay `no_std` and must not use `alloc`, because the kernel
//! links it before a heap exists.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
// The placeholder bodies do not use their parameters. Delete this line when
// you have written them.
#![allow(unused_variables)]

use core::alloc::Layout;
use core::marker::PhantomData;

// ---------------------------------------------------------------------------
// SPEC 5.1  Ring buffer
// ---------------------------------------------------------------------------

/// A first-in, first-out queue that holds at most `N` values and never
/// allocates.
pub struct RingBuffer<T, const N: usize> {
    _todo: PhantomData<T>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Creates an empty queue. Must be usable to initialise a `static`.
    pub const fn new() -> Self {
        todo!()
    }

    /// The largest number of values the queue can hold: `N`.
    pub const fn capacity(&self) -> usize {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn is_full(&self) -> bool {
        todo!()
    }

    /// Adds `value` at the back. A full queue returns `Err(value)`.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        todo!()
    }

    /// Removes and returns the oldest value.
    pub fn pop(&mut self) -> Option<T> {
        todo!()
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPEC 5.2  Allocators
// ---------------------------------------------------------------------------

/// A bump allocator over one region of memory.
pub struct Bump {}

impl Bump {
    /// An allocator without memory: every `alloc` returns null until `init`.
    pub const fn new() -> Self {
        todo!()
    }

    /// Gives the allocator the region `start .. start + len`.
    ///
    /// # Safety
    ///
    /// The region must be valid for reads and writes, must not be used by
    /// anything else while this allocator or any block from it is in use, and
    /// `start + len` must not overflow.
    pub unsafe fn init(&mut self, start: *mut u8, len: usize) {
        todo!()
    }

    /// Returns a block for `layout`, or null when the region is exhausted.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        todo!()
    }

    /// Marks one block as freed.
    ///
    /// # Safety
    ///
    /// `ptr` must come from `alloc` on this allocator with the same `layout`
    /// and must not have been freed already.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        todo!()
    }

    /// Bytes between the start of the region and the bump pointer.
    pub fn used(&self) -> usize {
        todo!()
    }
}

impl Default for Bump {
    fn default() -> Self {
        Self::new()
    }
}

/// A free-list allocator over one region of memory.
pub struct FreeList {}

impl FreeList {
    /// An allocator without memory: every `alloc` returns null until `init`.
    pub const fn new() -> Self {
        todo!()
    }

    /// Gives the allocator the region `start .. start + len`.
    ///
    /// # Safety
    ///
    /// Same contract as [`Bump::init`].
    pub unsafe fn init(&mut self, start: *mut u8, len: usize) {
        todo!()
    }

    /// Returns a block for `layout`, or null when no free block fits.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        todo!()
    }

    /// Returns a block to the allocator. Uses only the address of `ptr`: the
    /// pointers that the allocator keeps are derived from the region pointer
    /// (SPEC 5.2).
    ///
    /// # Safety
    ///
    /// Same contract as [`Bump::dealloc`].
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        todo!()
    }

    /// Total number of bytes that are free now.
    pub fn free_bytes(&self) -> usize {
        todo!()
    }
}

impl Default for FreeList {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPEC 5.3  Scheduler
// ---------------------------------------------------------------------------

/// Names one task. Ids are handed out in order (0, 1, 2, ...) and are never
/// reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Exited,
}

/// A round-robin scheduler for at most `N` tasks, counted over its whole
/// life. It decides which task runs; it does not switch contexts.
pub struct Scheduler<const N: usize> {}

impl<const N: usize> Scheduler<N> {
    /// Creates a scheduler in which task 0 exists and is `Running`.
    pub const fn new() -> Self {
        todo!()
    }

    pub fn spawn(&mut self) -> Option<TaskId> {
        todo!()
    }

    pub fn current(&self) -> TaskId {
        todo!()
    }

    pub fn state(&self, id: TaskId) -> Option<TaskState> {
        todo!()
    }

    pub fn yield_current(&mut self) -> Option<TaskId> {
        todo!()
    }

    pub fn block_current(&mut self) -> Option<TaskId> {
        todo!()
    }

    pub fn exit_current(&mut self) -> Option<TaskId> {
        todo!()
    }

    pub fn wake(&mut self, id: TaskId) -> bool {
        todo!()
    }
}

impl<const N: usize> Default for Scheduler<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPEC 5.4  IPC endpoint
// ---------------------------------------------------------------------------

/// A fixed-size message: a label and four payload words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub label: u64,
    pub words: [u64; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcError {
    Full,
    CallInProgress,
    ReceiverBusy,
    NotAwaitingReply,
}

/// The state machine of one synchronous send/receive/reply endpoint with
/// room for `N` calls in progress.
pub struct Endpoint<const N: usize> {}

impl<const N: usize> Endpoint<N> {
    pub const fn new() -> Self {
        todo!()
    }

    pub fn send(&mut self, from: TaskId, msg: Message) -> Result<Option<TaskId>, IpcError> {
        todo!()
    }

    pub fn receive(&mut self, me: TaskId) -> Result<Option<(TaskId, Message)>, IpcError> {
        todo!()
    }

    pub fn reply(&mut self, to: TaskId, msg: Message) -> Result<(), IpcError> {
        todo!()
    }

    pub fn take_reply(&mut self, me: TaskId) -> Option<Message> {
        todo!()
    }
}

impl<const N: usize> Default for Endpoint<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPEC 5.5  Register arithmetic
// ---------------------------------------------------------------------------

pub mod reg {
    /// Extracts bits `lo ..= hi` of `value`, shifted down to bit 0.
    pub const fn get_field(value: u64, hi: u32, lo: u32) -> u64 {
        todo!()
    }

    /// Returns `value` with bits `lo ..= hi` replaced by `field`.
    pub const fn set_field(value: u64, hi: u32, lo: u32, field: u64) -> u64 {
        todo!()
    }

    /// The PL011 baud rate divisor as `(UARTIBRD, UARTFBRD)`.
    pub const fn pl011_baud_divisors(uartclk_hz: u32, baud: u32) -> (u32, u32) {
        todo!()
    }

    /// `(n, mask)` for the one-bit-per-interrupt GIC registers.
    pub const fn gic_bit_slot(intid: u32) -> (usize, u32) {
        todo!()
    }

    /// `(n, shift)` for the one-byte-per-interrupt GIC registers.
    pub const fn gic_byte_slot(intid: u32) -> (usize, u32) {
        todo!()
    }

    /// The number of counter ticks between two timer interrupts.
    pub const fn timer_interval(counter_hz: u64, tick_hz: u64) -> u64 {
        todo!()
    }

    /// The cause of a synchronous exception: the `EC` field of `ESR_EL1`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ExceptionClass {
        /// EC 0x00
        Unknown,
        /// EC 0x15
        Svc,
        /// EC 0x20
        InstructionAbortLower,
        /// EC 0x21
        InstructionAbortSame,
        /// EC 0x24
        DataAbortLower,
        /// EC 0x25
        DataAbortSame,
        /// EC 0x3c
        Brk,
        /// Any other value of the 6-bit field.
        Other(u8),
    }

    /// Decodes bits 31..=26 of an `ESR_EL1` value.
    pub const fn exception_class(esr: u64) -> ExceptionClass {
        todo!()
    }

    /// The immediate of the `svc` instruction that caused `esr`, or `None`
    /// when the exception is not an SVC.
    pub const fn svc_number(esr: u64) -> Option<u16> {
        todo!()
    }
}
