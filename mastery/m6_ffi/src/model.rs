//! A Rust model of the C library, compiled **only under Miri**.
//!
//! Miri interprets Rust code and cannot run compiled C. Without this file,
//! every test that touches kvtab would have to be skipped under Miri, and
//! Miri would check nothing in the lessons where memory mistakes are most
//! likely. So when the crate is built for Miri, this module defines the
//! symbols `kvtab_new`, `kvtab_put` and so on in Rust. The declarations in
//! `sys.rs` (and yours in lesson 03) then resolve to these functions, and
//! Miri checks every pointer that crosses the boundary: use after free,
//! double free, leaks, out-of-bounds reads and wrong function signatures.
//!
//! What this does and does not prove: Miri checks **your side** of the
//! boundary against a model that follows the contract in kvtab.h. It says
//! nothing about `kvtab.c` itself. The model keeps one heap allocation per
//! key and per value, exactly like the C code, so a pointer from
//! `kvtab_peek` dies at the same moment in both.
//!
//! You do not need to edit this file.

use crate::sys::{
    KVTAB_ERR_FULL, KVTAB_ERR_NOT_FOUND, KVTAB_ERR_NULL, KVTAB_ERR_STOPPED, KVTAB_ERR_TOO_SMALL,
    KVTAB_EVENT_PUT, KVTAB_EVENT_REMOVE, KVTAB_OK, kvtab, kvtab_drop_fn, kvtab_observe_fn,
    kvtab_stats, kvtab_visit_fn,
};
use std::ffi::{CStr, CString, c_char, c_int, c_uchar, c_uint, c_void};
use std::ptr;

/// One entry. Raw pointers, as in C: the table owns both allocations.
struct Entry {
    /// From `CString::into_raw`.
    key: *mut c_char,
    /// From `Box::<[u8]>::into_raw`, with length `value_len`. Never null.
    value: *mut u8,
    value_len: usize,
}

struct Table {
    entries: Vec<Entry>,
    capacity: usize,
    puts: c_uint,
    observe: kvtab_observe_fn,
    drop: kvtab_drop_fn,
    userdata: *mut c_void,
}

impl Table {
    /// The index of `key`, compared byte by byte like `strcmp`.
    ///
    /// # Safety
    /// `key` must point to a NUL-terminated string.
    unsafe fn find(&self, key: *const c_char) -> Option<usize> {
        // SAFETY: the caller promises a NUL-terminated string.
        let wanted = unsafe { CStr::from_ptr(key) };
        self.entries.iter().position(|entry| {
            // SAFETY: `entry.key` came from `CString::into_raw` and is freed
            // only when the entry leaves the table.
            unsafe { CStr::from_ptr(entry.key) == wanted }
        })
    }
}

/// Frees the two allocations of an entry.
///
/// # Safety
/// The entry must have been removed from its table, and must not be used again.
unsafe fn free_entry(entry: Entry) {
    // SAFETY: `key` came from `CString::into_raw`, and `value` with
    // `value_len` from `Box::<[u8]>::into_raw`. The caller gives up the entry,
    // so each allocation is released exactly once.
    unsafe {
        drop(CString::from_raw(entry.key));
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
            entry.value,
            entry.value_len,
        )));
    }
}

/// Copies `len` bytes into a new allocation and leaks it as a raw pointer.
///
/// # Safety
/// `src` must be readable for `len` bytes, unless `len` is 0.
unsafe fn copy_bytes(src: *const c_uchar, len: usize) -> *mut u8 {
    let boxed: Box<[u8]> = if len == 0 {
        Box::default()
    } else {
        // SAFETY: the caller promises `len` readable bytes at `src`.
        unsafe { std::slice::from_raw_parts(src, len) }.into()
    };
    Box::into_raw(boxed).cast::<u8>()
}

/// Model of `const char *kvtab_version(void)`.
#[unsafe(no_mangle)]
pub extern "C" fn kvtab_version() -> *const c_char {
    c"kvtab 1.0.0".as_ptr()
}

/// Model of `const char *kvtab_strerror(int status)`.
#[unsafe(no_mangle)]
pub extern "C" fn kvtab_strerror(status: c_int) -> *const c_char {
    let text: &'static CStr = match status {
        0 => c"success",
        1 => c"a required pointer was NULL",
        2 => c"key not found",
        3 => c"table is full",
        4 => c"buffer too small",
        5 => c"out of memory",
        6 => c"stopped by the visitor",
        _ => c"unknown status",
    };
    text.as_ptr()
}

/// Model of `kvtab *kvtab_new(size_t capacity)`.
#[unsafe(no_mangle)]
pub extern "C" fn kvtab_new(capacity: usize) -> *mut kvtab {
    // The C code allocates `capacity` slots with calloc, which fails when the
    // byte count overflows. Three pointers-sized fields per slot, as in C.
    let slot = 3 * size_of::<usize>();
    if capacity == 0 || capacity > isize::MAX as usize / slot {
        return ptr::null_mut();
    }
    let table = Box::new(Table {
        entries: Vec::new(),
        capacity,
        puts: 0,
        observe: None,
        drop: None,
        userdata: ptr::null_mut(),
    });
    Box::into_raw(table).cast::<kvtab>()
}

/// Model of `void kvtab_free(kvtab *t)`.
///
/// # Safety
/// `t` must be NULL or a live table from `kvtab_new` that is not used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_free(t: *mut kvtab) {
    if t.is_null() {
        return;
    }
    // SAFETY: a non-null `t` came from `Box::into_raw` in `kvtab_new`, and the
    // caller gives up the table, so the box is rebuilt exactly once.
    let mut table = unsafe { Box::from_raw(t.cast::<Table>()) };
    for entry in table.entries.drain(..) {
        // SAFETY: `drain` removed the entry from the table.
        unsafe { free_entry(entry) };
    }
    if let Some(drop_userdata) = table.drop {
        // SAFETY: the caller of `kvtab_set_observer` promised that this
        // function accepts this userdata; it is called once, here.
        unsafe { drop_userdata(table.userdata) };
    }
}

/// Model of `kvtab_put`.
///
/// # Safety
/// `t` must be NULL or a live table with no other call in progress. `key`
/// must be NULL or a NUL-terminated string. `value` must be readable for
/// `value_len` bytes, unless `value_len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_put(
    t: *mut kvtab,
    key: *const c_char,
    value: *const c_uchar,
    value_len: usize,
) -> c_int {
    if t.is_null() || key.is_null() || (value.is_null() && value_len != 0) {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table and no other call is in progress, so this
    // is the only reference to it.
    let table = unsafe { &mut *t.cast::<Table>() };
    // SAFETY: `key` is a NUL-terminated string.
    let found = unsafe { table.find(key) };
    if found.is_none() && table.entries.len() == table.capacity {
        return KVTAB_ERR_FULL;
    }
    // SAFETY: `value` is readable for `value_len` bytes, or `value_len` is 0.
    let copy = unsafe { copy_bytes(value, value_len) };
    let index = match found {
        Some(index) => {
            let entry = &mut table.entries[index];
            // SAFETY: the old value came from `copy_bytes` with this length
            // and is replaced on the next lines.
            unsafe {
                drop(Box::from_raw(ptr::slice_from_raw_parts_mut(
                    entry.value,
                    entry.value_len,
                )));
            }
            entry.value = copy;
            entry.value_len = value_len;
            index
        }
        None => {
            // SAFETY: `key` is a NUL-terminated string.
            let key_copy = unsafe { CStr::from_ptr(key) }.to_owned().into_raw();
            table.entries.push(Entry {
                key: key_copy,
                value: copy,
                value_len,
            });
            table.entries.len() - 1
        }
    };
    table.puts = table.puts.wrapping_add(1);
    if let Some(observe) = table.observe {
        // SAFETY: whoever installed the observer promised that it accepts
        // this userdata. The key pointer is valid during the call.
        unsafe { observe(KVTAB_EVENT_PUT, table.entries[index].key, table.userdata) };
    }
    KVTAB_OK
}

/// Model of `kvtab_get`.
///
/// # Safety
/// `t` must be NULL or a live table. `key` must be NULL or a NUL-terminated
/// string. `len` must be NULL or valid for reads and writes. `buf` must be
/// writable for `*len` bytes, unless `*len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_get(
    t: *const kvtab,
    key: *const c_char,
    buf: *mut c_uchar,
    len: *mut usize,
) -> c_int {
    if t.is_null() || key.is_null() || len.is_null() {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table; reading needs only a shared reference.
    let table = unsafe { &*t.cast::<Table>() };
    // SAFETY: `key` is a NUL-terminated string.
    let Some(index) = (unsafe { table.find(key) }) else {
        return KVTAB_ERR_NOT_FOUND;
    };
    let entry = &table.entries[index];
    // SAFETY: `len` is valid for reads and writes.
    let room = unsafe { len.replace(entry.value_len) };
    if room < entry.value_len {
        return KVTAB_ERR_TOO_SMALL;
    }
    if entry.value_len != 0 {
        if buf.is_null() {
            return KVTAB_ERR_NULL;
        }
        // SAFETY: `buf` is writable for `room >= value_len` bytes, the value
        // is readable for `value_len` bytes, and the two allocations are
        // distinct.
        unsafe { ptr::copy_nonoverlapping(entry.value, buf, entry.value_len) };
    }
    KVTAB_OK
}

/// Model of `kvtab_peek`.
///
/// # Safety
/// `t` must be NULL or a live table. `key` must be NULL or a NUL-terminated
/// string. `len` must be NULL or valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_peek(
    t: *const kvtab,
    key: *const c_char,
    len: *mut usize,
) -> *const c_uchar {
    let mut found: Option<(*const u8, usize)> = None;
    if !t.is_null() && !key.is_null() && !len.is_null() {
        // SAFETY: `t` is a live table; reading needs only a shared reference.
        let table = unsafe { &*t.cast::<Table>() };
        // SAFETY: `key` is a NUL-terminated string.
        if let Some(index) = unsafe { table.find(key) } {
            let entry = &table.entries[index];
            found = Some((entry.value.cast_const(), entry.value_len));
        }
    }
    if !len.is_null() {
        // SAFETY: a non-null `len` is valid for writes.
        unsafe { len.write(found.map_or(0, |(_, value_len)| value_len)) };
    }
    found.map_or(ptr::null(), |(value, _)| value)
}

/// Model of `kvtab_remove`.
///
/// # Safety
/// `t` must be NULL or a live table with no other call in progress. `key`
/// must be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_remove(t: *mut kvtab, key: *const c_char) -> c_int {
    if t.is_null() || key.is_null() {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table and no other call is in progress.
    let table = unsafe { &mut *t.cast::<Table>() };
    // SAFETY: `key` is a NUL-terminated string.
    let Some(index) = (unsafe { table.find(key) }) else {
        return KVTAB_ERR_NOT_FOUND;
    };
    let removed = table.entries.remove(index);
    if let Some(observe) = table.observe {
        // SAFETY: as in `kvtab_put`. The key is freed only after the call.
        unsafe { observe(KVTAB_EVENT_REMOVE, removed.key, table.userdata) };
    }
    // SAFETY: `removed` is no longer in the table.
    unsafe { free_entry(removed) };
    KVTAB_OK
}

/// Model of `kvtab_len`.
///
/// # Safety
/// `t` must be NULL or a live table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_len(t: *const kvtab) -> usize {
    if t.is_null() {
        return 0;
    }
    // SAFETY: `t` is a live table; reading needs only a shared reference.
    unsafe { &*t.cast::<Table>() }.entries.len()
}

/// Model of `kvtab_get_stats`.
///
/// # Safety
/// `t` must be NULL or a live table. `out` must be NULL or valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_get_stats(t: *const kvtab, out: *mut kvtab_stats) -> c_int {
    if t.is_null() || out.is_null() {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table; reading needs only a shared reference.
    let table = unsafe { &*t.cast::<Table>() };
    let stats = kvtab_stats {
        len: table.entries.len(),
        capacity: table.capacity,
        value_bytes: table.entries.iter().map(|entry| entry.value_len).sum(),
        puts: table.puts,
        full: c_uchar::from(table.entries.len() == table.capacity),
    };
    // SAFETY: `out` is valid for writes.
    unsafe { out.write(stats) };
    KVTAB_OK
}

/// Model of `kvtab_for_each`.
///
/// # Safety
/// `t` must be NULL or a live table. `visit` must accept `userdata` and must
/// not change the table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_for_each(
    t: *const kvtab,
    visit: kvtab_visit_fn,
    userdata: *mut c_void,
) -> c_int {
    let Some(visit) = visit else {
        return KVTAB_ERR_NULL;
    };
    if t.is_null() {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table. The visitor must not change it, so a
    // shared reference stays valid across the calls.
    let table = unsafe { &*t.cast::<Table>() };
    for entry in &table.entries {
        // SAFETY: the caller promises that `visit` accepts `userdata`. Key
        // and value are valid for the duration of the call.
        let stop = unsafe { visit(entry.key, entry.value, entry.value_len, userdata) };
        if stop != 0 {
            return KVTAB_ERR_STOPPED;
        }
    }
    KVTAB_OK
}

/// Model of `kvtab_set_observer`.
///
/// # Safety
/// `t` must be NULL or a live table with no other call in progress.
/// `observe` and `drop` must accept `userdata`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kvtab_set_observer(
    t: *mut kvtab,
    observe: kvtab_observe_fn,
    userdata: *mut c_void,
    drop: kvtab_drop_fn,
) -> c_int {
    if t.is_null() {
        return KVTAB_ERR_NULL;
    }
    // SAFETY: `t` is a live table and no other call is in progress.
    let table = unsafe { &mut *t.cast::<Table>() };
    if let Some(drop_previous) = table.drop {
        // SAFETY: the previous observer's drop function accepts the previous
        // userdata; this is its one call.
        unsafe { drop_previous(table.userdata) };
    }
    table.observe = observe;
    table.userdata = userdata;
    table.drop = drop;
    KVTAB_OK
}
