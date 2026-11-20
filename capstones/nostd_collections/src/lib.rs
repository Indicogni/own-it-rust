//! `nostd_collections`: fixed-capacity collections and a bump arena for
//! programs without a heap.
//!
//! This file is the *facade* of the capstone. SPEC.md fixes the public
//! names and signatures that the acceptance tests in `tests/` use; this file
//! declares exactly those, with `todo!()` bodies, so that the tests compile
//! before you have written anything. It contains no design:
//!
//! * the structs have no real fields;
//! * the iterator type [`Todo`] is a stand-in that you replace by your own
//!   iterator types (or by `impl Trait` return types) and then delete;
//! * there are no modules. Split the crate as you like and re-export the
//!   four names from here: `ArrayVec`, `RingBuffer`, `Arena`,
//!   `CapacityError`.
//!
//! Three things must stay as they are: `#![no_std]`, the signatures that
//! SPEC.md gives, and the `spec_examples` module at the end of this file.

#![no_std]
// The placeholder bodies do not use their parameters. Delete this line when
// you have written them.
#![allow(unused_variables)]

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut, RangeBounds};

// ---------------------------------------------------------------------------
// SPEC 3  CapacityError
// ---------------------------------------------------------------------------

/// "No room for this value." Carries the value that did not fit.
pub struct CapacityError<T> {
    _todo: PhantomData<T>,
}

impl<T> CapacityError<T> {
    pub fn into_inner(self) -> T {
        todo!("SPEC 3")
    }
}

impl<T> fmt::Debug for CapacityError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC 3")
    }
}

impl<T> fmt::Display for CapacityError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC 3")
    }
}

impl<T> core::error::Error for CapacityError<T> {}

// ---------------------------------------------------------------------------
// SPEC 4  ArrayVec
// ---------------------------------------------------------------------------

/// A vector that stores at most `N` elements inside itself.
pub struct ArrayVec<T, const N: usize> {
    _todo: PhantomData<T>,
}

impl<T, const N: usize> ArrayVec<T, N> {
    pub const fn new() -> Self {
        // `todo!` with a message is not allowed in a `const fn`.
        panic!("not yet implemented: SPEC 4.1")
    }

    pub fn capacity(&self) -> usize {
        todo!("SPEC 4.1")
    }

    pub fn len(&self) -> usize {
        todo!("SPEC 4.1")
    }

    pub fn is_empty(&self) -> bool {
        todo!("SPEC 4.1")
    }

    pub fn is_full(&self) -> bool {
        todo!("SPEC 4.1")
    }

    pub fn push(&mut self, value: T) {
        todo!("SPEC 4.2")
    }

    pub fn try_push(&mut self, value: T) -> Result<(), CapacityError<T>> {
        todo!("SPEC 4.2")
    }

    pub fn pop(&mut self) -> Option<T> {
        todo!("SPEC 4.2")
    }

    pub fn as_slice(&self) -> &[T] {
        todo!("SPEC 4.2")
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        todo!("SPEC 4.2")
    }

    pub fn truncate(&mut self, len: usize) {
        todo!("SPEC 4.3")
    }

    pub fn clear(&mut self) {
        todo!("SPEC 4.3")
    }

    pub fn insert(&mut self, index: usize, value: T) {
        todo!("SPEC 4.4")
    }

    pub fn try_insert(&mut self, index: usize, value: T) -> Result<(), CapacityError<T>> {
        todo!("SPEC 4.4")
    }

    pub fn remove(&mut self, index: usize) -> T {
        todo!("SPEC 4.4")
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        todo!("SPEC 4.4")
    }

    pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        todo!("SPEC 4.5")
    }

    /// The real return type must borrow the vector; see SPEC 4.6 and 7.
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Todo<T> {
        todo!("SPEC 4.6")
    }

    pub fn into_inner(self) -> Result<[T; N], Self> {
        todo!("SPEC 4.8")
    }
}

impl<T, const N: usize> Default for ArrayVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Deref for ArrayVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        todo!("SPEC 4.2")
    }
}

impl<T, const N: usize> DerefMut for ArrayVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        todo!("SPEC 4.2")
    }
}

impl<T, const N: usize> IntoIterator for ArrayVec<T, N> {
    type Item = T;
    type IntoIter = Todo<T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 4.7")
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a ArrayVec<T, N> {
    type Item = &'a T;
    type IntoIter = Todo<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 4.7")
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut ArrayVec<T, N> {
    type Item = &'a mut T;
    type IntoIter = Todo<&'a mut T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 4.7")
    }
}

impl<T, const N: usize> Extend<T> for ArrayVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        todo!("SPEC 4.8")
    }
}

impl<T, const N: usize> FromIterator<T> for ArrayVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        todo!("SPEC 4.8")
    }
}

impl<T, const N: usize> From<[T; N]> for ArrayVec<T, N> {
    fn from(array: [T; N]) -> Self {
        todo!("SPEC 4.8")
    }
}

impl<T: Clone, const N: usize> Clone for ArrayVec<T, N> {
    fn clone(&self) -> Self {
        todo!("SPEC 4.8")
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for ArrayVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC 4.8")
    }
}

impl<T: PartialEq, const N: usize> PartialEq for ArrayVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        todo!("SPEC 4.8")
    }
}

impl<T: Eq, const N: usize> Eq for ArrayVec<T, N> {}

// ---------------------------------------------------------------------------
// SPEC 5  RingBuffer
// ---------------------------------------------------------------------------

/// A double-ended queue that stores at most `N` elements inside itself.
pub struct RingBuffer<T, const N: usize> {
    _todo: PhantomData<T>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        // `todo!` with a message is not allowed in a `const fn`.
        panic!("not yet implemented: SPEC 5.1")
    }

    pub fn capacity(&self) -> usize {
        todo!("SPEC 5.1")
    }

    pub fn len(&self) -> usize {
        todo!("SPEC 5.1")
    }

    pub fn is_empty(&self) -> bool {
        todo!("SPEC 5.1")
    }

    pub fn is_full(&self) -> bool {
        todo!("SPEC 5.1")
    }

    pub fn push_back(&mut self, value: T) -> Result<(), CapacityError<T>> {
        todo!("SPEC 5.2")
    }

    pub fn push_front(&mut self, value: T) -> Result<(), CapacityError<T>> {
        todo!("SPEC 5.2")
    }

    pub fn push_overwrite(&mut self, value: T) -> Option<T> {
        todo!("SPEC 5.2")
    }

    pub fn pop_front(&mut self) -> Option<T> {
        todo!("SPEC 5.2")
    }

    pub fn pop_back(&mut self) -> Option<T> {
        todo!("SPEC 5.2")
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        todo!("SPEC 5.3")
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        todo!("SPEC 5.3")
    }

    pub fn front(&self) -> Option<&T> {
        todo!("SPEC 5.3")
    }

    pub fn back(&self) -> Option<&T> {
        todo!("SPEC 5.3")
    }

    pub fn as_slices(&self) -> (&[T], &[T]) {
        todo!("SPEC 5.3")
    }

    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        todo!("SPEC 5.3")
    }

    pub fn clear(&mut self) {
        todo!("SPEC 5.4")
    }

    pub fn iter(&self) -> Todo<&T> {
        todo!("SPEC 5.5")
    }

    pub fn iter_mut(&mut self) -> Todo<&mut T> {
        todo!("SPEC 5.5")
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> IntoIterator for RingBuffer<T, N> {
    type Item = T;
    type IntoIter = Todo<T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 5.5")
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N> {
    type Item = &'a T;
    type IntoIter = Todo<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 5.5")
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut RingBuffer<T, N> {
    type Item = &'a mut T;
    type IntoIter = Todo<&'a mut T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!("SPEC 5.5")
    }
}

impl<T: Clone, const N: usize> Clone for RingBuffer<T, N> {
    fn clone(&self) -> Self {
        todo!("SPEC 5.6")
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for RingBuffer<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC 5.6")
    }
}

impl<T: PartialEq, const N: usize> PartialEq for RingBuffer<T, N> {
    fn eq(&self, other: &Self) -> bool {
        todo!("SPEC 5.6")
    }
}

impl<T: Eq, const N: usize> Eq for RingBuffer<T, N> {}

// ---------------------------------------------------------------------------
// SPEC 6  Arena
// ---------------------------------------------------------------------------

/// A bump allocator with room for `N` bytes, stored inside itself.
pub struct Arena<const N: usize> {
    _todo: (),
}

impl<const N: usize> Arena<N> {
    pub const fn new() -> Self {
        // `todo!` with a message is not allowed in a `const fn`.
        panic!("not yet implemented: SPEC 6.1")
    }

    pub fn capacity(&self) -> usize {
        todo!("SPEC 6.1")
    }

    pub fn used(&self) -> usize {
        todo!("SPEC 6.1")
    }

    pub fn remaining(&self) -> usize {
        todo!("SPEC 6.1")
    }

    // Clippy's `mut_from_ref` warns when a function makes a `&mut` from a
    // `&self`. Handing out such references is the purpose of an arena;
    // SPEC 6.3 asks you to write down why yours is sound.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> Result<&mut T, CapacityError<T>> {
        todo!("SPEC 6.3")
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> Option<&mut [T]> {
        todo!("SPEC 6.4")
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_str(&self, src: &str) -> Option<&mut str> {
        todo!("SPEC 6.4")
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice_fill_with<T, F>(&self, len: usize, fill: F) -> Option<&mut [T]>
    where
        F: FnMut(usize) -> T,
    {
        todo!("SPEC 6.4")
    }

    pub fn reset(&mut self) {
        todo!("SPEC 6.5")
    }
}

impl<const N: usize> Default for Arena<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Debug for Arena<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC 6.1")
    }
}

// ---------------------------------------------------------------------------
// A stand-in for the iterator types that you will write
// ---------------------------------------------------------------------------

/// A placeholder iterator. It implements the iterator traits that the tests
/// ask for, so that the facade compiles. Replace every use of it by an
/// iterator type of your own, then delete it.
pub struct Todo<Item> {
    _todo: PhantomData<Item>,
}

impl<Item> Iterator for Todo<Item> {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        todo!()
    }
}

impl<Item> DoubleEndedIterator for Todo<Item> {
    fn next_back(&mut self) -> Option<Item> {
        todo!()
    }
}

impl<Item> ExactSizeIterator for Todo<Item> {}

impl<Item> Clone for Todo<Item> {
    fn clone(&self) -> Self {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// The examples of the specification, as tests
// ---------------------------------------------------------------------------

// Keep this module declaration; see `src/spec_examples.rs` and SPEC 9.2.
#[cfg(doctest)]
mod spec_examples;
