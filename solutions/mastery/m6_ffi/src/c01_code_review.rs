//! CODE REVIEW 01 — A teammate's wrapper around kvtab
//! Do this after lessons 04 to 06.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml c01_
//! Then: cargo clippy --manifest-path mastery/m6_ffi/Cargo.toml --all-targets
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and the happy-path tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md.
//!    For every `unsafe` block, write the SAFETY comment that is missing and
//!    check whether it is true. List every status code that is ignored, every
//!    place where a panic could start, and say for each one in which frame it
//!    would start. Decide whether the two `unsafe impl` lines are justified by
//!    `vendor/kvtab/kvtab.h`.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix the code. Keep the names of the type and of its methods. You may
//!    change the fields, change parameter types as long as the tests still
//!    compile, and add impls.
//! 4. Run clippy and resolve every warning for this file.

use crate::sys;
use std::ffi::{CStr, CString, c_char, c_int, c_uchar, c_void};
use std::ptr::NonNull;

/// Why [`Settings::load`] failed.
#[derive(Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// A name contains a NUL byte.
    InvalidName,
    /// The library reported this status code.
    Library(c_int),
}

/// A read-only settings store on top of a kvtab table.
pub struct Settings {
    /// Invariant: a live table from `kvtab_new` that this value owns alone.
    raw: NonNull<sys::kvtab>,
}

// SAFETY: kvtab.h, "Threads": a table has no thread affinity; calls may come
// from different threads as long as they do not overlap. Moving the only
// handle to another thread cannot make calls overlap.
unsafe impl Send for Settings {}

// Not `Sync`: the header forbids two threads inside calls on one table at the
// same time, whatever the functions are, and `get` and `names` take `&self`.

impl Drop for Settings {
    fn drop(&mut self) {
        // SAFETY: by the invariant, `raw` is a live table that this value
        // owns, and `drop` runs once.
        unsafe { sys::kvtab_free(self.raw.as_ptr()) };
    }
}

impl Settings {
    /// Loads `(name, value)` pairs. A later pair with the same name wins.
    pub fn load(pairs: &[(String, String)]) -> Result<Settings, SettingsError> {
        // kvtab_new refuses a capacity of 0, and an empty store is a valid
        // store: give it room for one entry.
        // SAFETY: kvtab_new accepts every capacity and returns NULL on failure.
        let raw = unsafe { sys::kvtab_new(pairs.len().max(1)) };
        let raw = NonNull::new(raw).ok_or(SettingsError::Library(sys::KVTAB_ERR_NOMEM))?;
        // From here on `Drop` frees the table on every early return.
        let settings = Settings { raw };
        for (name, value) in pairs {
            let name = CString::new(name.as_str()).map_err(|_| SettingsError::InvalidName)?;
            // SAFETY: `raw` is live and used only here. `name` is
            // NUL-terminated. `value` is readable for `value.len()` bytes,
            // and is not read when that is 0.
            let status =
                unsafe { sys::kvtab_put(raw.as_ptr(), name.as_ptr(), value.as_ptr(), value.len()) };
            if status != sys::KVTAB_OK {
                return Err(SettingsError::Library(status));
            }
        }
        Ok(settings)
    }

    /// The value of `name`, or `None` when it is not set. Bytes that are not
    /// UTF-8 are replaced with U+FFFD.
    pub fn get(&self, name: &str) -> Option<String> {
        // A name with a NUL byte cannot be a key, so it is not set.
        let name = CString::new(name).ok()?;
        let mut len = 0usize;
        // SAFETY: `raw` is live, `name` is NUL-terminated, `&mut len` is a
        // valid out-pointer.
        let value = unsafe { sys::kvtab_peek(self.raw.as_ptr(), name.as_ptr(), &mut len) };
        if value.is_null() {
            return None;
        }
        // SAFETY: a non-null result points to `len` bytes that stay valid
        // until the next put, remove or free on this table. This type has no
        // method that changes the table, and the slice is not kept beyond
        // this function.
        let bytes = unsafe { std::slice::from_raw_parts(value, len) };
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// The number of settings.
    pub fn len(&self) -> usize {
        // SAFETY: `raw` is a live table.
        unsafe { sys::kvtab_len(self.raw.as_ptr()) }
    }

    /// Whether there are no settings.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// All names, in insertion order. Bytes that are not UTF-8 are replaced
    /// with U+FFFD.
    pub fn names(&self) -> Vec<String> {
        /// Pushes one name. Nothing in here can panic except an allocation
        /// failure, which aborts instead of unwinding.
        unsafe extern "C" fn collect(
            key: *const c_char,
            _value: *const c_uchar,
            _value_len: usize,
            userdata: *mut c_void,
        ) -> c_int {
            // SAFETY: `names` below passes a pointer to its local Vec and
            // does not touch it while kvtab_for_each runs.
            let names = unsafe { &mut *userdata.cast::<Vec<String>>() };
            // SAFETY: kvtab.h: `key` is NUL-terminated and valid during the call.
            let key = unsafe { CStr::from_ptr(key) };
            names.push(key.to_string_lossy().into_owned());
            0
        }
        let mut names: Vec<String> = Vec::new();
        // SAFETY: `raw` is live. `collect` reads `userdata` as the
        // `Vec<String>` that it is, and the Vec outlives the call.
        let status = unsafe {
            sys::kvtab_for_each(
                self.raw.as_ptr(),
                Some(collect),
                (&raw mut names).cast::<c_void>(),
            )
        };
        debug_assert_eq!(status, sys::KVTAB_OK);
        names
    }

    /// The raw handle, for tests and for `sys` calls that this type lacks.
    pub fn as_raw(&self) -> *mut sys::kvtab {
        self.raw.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::alloc::count;
    // Imported here too, so that the tests do not depend on which imports
    // your version of the code above still needs.
    use crate::sys;
    use std::ffi::c_void;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    // ---- These pass before you change anything. ----

    #[test]
    fn c01_loads_and_reads_short_values() {
        let settings = Settings::load(&pairs(&[("host", "localhost"), ("port", "8080")])).unwrap();
        assert_eq!(settings.len(), 2);
        assert!(!settings.is_empty());
        assert_eq!(settings.get("host").as_deref(), Some("localhost"));
        assert_eq!(settings.get("port").as_deref(), Some("8080"));
    }

    #[test]
    fn c01_lists_names_in_insertion_order() {
        let settings =
            Settings::load(&pairs(&[("b", "2"), ("a", "1"), ("b", "3"), ("c", "")])).unwrap();
        assert_eq!(settings.names(), ["b", "a", "c"]);
        assert_eq!(
            settings.get("b").as_deref(),
            Some("3"),
            "the later pair wins"
        );
    }

    // ---- These expose the bugs and the smells. ----

    #[test]
    fn c01_a_missing_name_is_none() {
        let settings = Settings::load(&pairs(&[("host", "localhost")])).unwrap();
        assert_eq!(
            settings.get("user"),
            None,
            "kvtab_get reported KVTAB_ERR_NOT_FOUND; a status code must not be ignored"
        );
    }

    #[test]
    fn c01_an_empty_value_is_not_a_missing_name() {
        let settings = Settings::load(&pairs(&[("motd", "")])).unwrap();
        assert_eq!(settings.get("motd").as_deref(), Some(""));
    }

    #[test]
    fn c01_long_values_come_back_whole() {
        let long = "x".repeat(100);
        let settings = Settings::load(&pairs(&[("banner", &long)])).unwrap();
        assert_eq!(
            settings.get("banner").as_deref(),
            Some(long.as_str()),
            "a value can have any length: no fixed buffer is large enough"
        );
    }

    #[test]
    fn c01_an_empty_store_works() {
        let settings = Settings::load(&pairs(&[])).unwrap();
        assert!(settings.is_empty());
        assert_eq!(settings.names(), Vec::<String>::new());
        assert_eq!(
            settings.get("anything"),
            None,
            "kvtab_new(0) returns NULL; an empty store still needs a real table"
        );
    }

    #[test]
    fn c01_a_name_with_a_nul_byte_is_an_error_not_a_panic() {
        let result = Settings::load(&pairs(&[("ok", "1"), ("bad\0name", "2")]));
        assert_eq!(result.err(), Some(SettingsError::InvalidName));
        let settings = Settings::load(&pairs(&[("ok", "1")])).unwrap();
        assert_eq!(
            settings.get("bad\0name"),
            None,
            "no key can contain a NUL byte, so such a name is simply not set"
        );
    }

    #[test]
    fn c01_dropping_the_store_frees_the_table() {
        unsafe extern "C" fn count_free(userdata: *mut c_void) {
            // SAFETY: the test passes a pointer to an AtomicUsize that it
            // keeps alive in an Arc.
            unsafe { &*userdata.cast::<AtomicUsize>() }.fetch_add(1, Ordering::SeqCst);
        }
        let frees = Arc::new(AtomicUsize::new(0));
        let settings = Settings::load(&pairs(&[("a", "1")])).unwrap();
        // The library calls an observer's drop function inside kvtab_free.
        // SAFETY: the table is live, and `frees` outlives it.
        let status = unsafe {
            sys::kvtab_set_observer(
                settings.as_raw(),
                None,
                Arc::as_ptr(&frees).cast_mut().cast::<c_void>(),
                Some(count_free),
            )
        };
        assert_eq!(status, sys::KVTAB_OK);
        drop(settings);
        assert_eq!(
            frees.load(Ordering::SeqCst),
            1,
            "the C table must be freed when the store is dropped; 0 means a leak"
        );
    }

    #[test]
    fn c01_a_store_must_not_be_shared_between_threads() {
        assert!(implements!(Settings: Send));
        assert!(
            !implements!(Settings: Sync),
            "kvtab.h: two threads must never be inside calls on the same table \
             at the same time, whatever the functions are. `get` takes &self."
        );
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "under Miri the model of kvtab allocates with Rust's allocator, so the count would include the library's own allocations"
    )]
    fn c01_technique_load_does_not_copy_its_input() {
        let input = pairs(&[("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")]);
        let (settings, allocations) = count(|| Settings::load(&input));
        assert!(settings.is_ok());
        assert!(
            allocations <= input.len() + 1,
            "load made {allocations} Rust allocations for {} pairs. Each name needs \
             one C string; nothing else needs to be copied on the Rust side",
            input.len()
        );
    }

    #[test]
    fn c01_technique_a_name_that_is_not_utf8_does_not_abort() {
        // A panic inside the function that C calls aborts the process, so
        // this test runs in a child process (see harness.rs).
        if crate::harness::in_child(
            "c01_code_review::tests::c01_technique_a_name_that_is_not_utf8_does_not_abort",
        ) {
            return;
        }
        let settings = Settings::load(&pairs(&[("plain", "1"), ("spare", "2")])).unwrap();
        // Another part of the program stores a key that is not UTF-8.
        // SAFETY: the table is live and nothing else is using it; the key is
        // NUL-terminated.
        let status = unsafe {
            sys::kvtab_remove(settings.as_raw(), c"spare".as_ptr());
            sys::kvtab_put(settings.as_raw(), c"caf\xe9".as_ptr(), b"x".as_ptr(), 1)
        };
        assert_eq!(status, sys::KVTAB_OK);
        assert_eq!(
            settings.names(),
            ["plain", "caf\u{FFFD}"],
            "keys are C strings and need not be UTF-8: convert them without panicking"
        );
    }
}
