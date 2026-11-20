//! LESSON 06 — Strings and buffers across the boundary
//! Read first: CONCEPTS.md, "Lesson 06". Start after: lesson 04.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l06_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: `kvtab_get` copies a value into a
//! buffer that the caller owns, and `kvtab_peek` returns a pointer into the table.
//! For each of the two, who owns the bytes afterwards, and what ends the validity
//! of the pointer?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: For every pointer that crosses the boundary in this lesson,
//! in either direction, state three things: who allocated the memory, who frees
//! it, and until when the other side may use it. Then explain why the slice that
//! `peek` returns cannot outlive the next `put`, and which line of code enforces it.
//! MIRI: this lesson runs under Miri against the Rust model of the library. Miri
//! then checks every buffer length and every borrow that you create.
//!
//! This lesson adds more methods to the `Table` of lesson 04. Only the public
//! signatures are given.

use crate::l04_safe_wrapper::{Error, Table};
use crate::sys;
use std::ffi::{CStr, CString};

/// A. The version text of the linked library, for example `kvtab 1.0.0`.
///
/// `kvtab_version` returns a pointer to a static C string. Borrow it; do not
/// copy it. This function must never panic: if the bytes were not UTF-8,
/// return the text `unknown`.
pub fn version() -> &'static str {
    // SAFETY: kvtab_version takes no arguments and returns a pointer to a
    // NUL-terminated string with static storage duration (kvtab.h: "Static:
    // never free it"), so `'static` is the true lifetime of the borrow.
    let text: &'static CStr = unsafe { CStr::from_ptr(sys::kvtab_version()) };
    text.to_str().unwrap_or("unknown")
}

impl Table {
    /// B. Copies the value of `key` into `buf` and returns its length.
    ///
    /// - The value must fit: otherwise the result is
    ///   `Error::BufferTooSmall { needed }` with the size of the value, and
    ///   `buf` is unchanged.
    /// - `buf` may be empty; then only an empty value fits.
    /// - A missing key is `Error::NotFound`.
    pub fn get_into(&self, key: &CStr, buf: &mut [u8]) -> Result<usize, Error> {
        let mut len = buf.len();
        // SAFETY:
        // - `as_raw` is a live table, and `&self` on a `!Sync` type means
        //   that no other thread is inside a call on it.
        // - `key` is NUL-terminated.
        // - `buf` is writable for `buf.len()` bytes, which is the size that
        //   `len` announces; the library never writes more. For an empty
        //   `buf` the pointer is dangling, and nothing is written through
        //   it: a value that fits into 0 bytes has 0 bytes.
        // - `&mut len` is valid for one read and one write.
        let status =
            unsafe { sys::kvtab_get(self.as_raw(), key.as_ptr(), buf.as_mut_ptr(), &mut len) };
        if status == sys::KVTAB_ERR_TOO_SMALL {
            // The library stored the size of the value in `len`.
            return Err(Error::BufferTooSmall { needed: len });
        }
        Error::check(status)?;
        Ok(len)
    }

    /// C. The value of `key` as a new `Vec<u8>`.
    ///
    /// Use the two-call protocol of `kvtab_get`: first ask for the size with
    /// no buffer at all, then allocate exactly once and let the library write
    /// into the vector's spare capacity. Do not fill the vector with zeros
    /// first, and do not allocate anything when the key is missing or the
    /// value is empty.
    pub fn get(&self, key: &CStr) -> Result<Vec<u8>, Error> {
        // First call: a buffer of size 0. kvtab.h allows a NULL buffer then.
        let mut len = 0usize;
        // SAFETY: live table, NUL-terminated key, and `&mut len` is valid.
        // The buffer pointer is not used, because `*len` is 0.
        let status =
            unsafe { sys::kvtab_get(self.as_raw(), key.as_ptr(), std::ptr::null_mut(), &mut len) };
        if status != sys::KVTAB_ERR_TOO_SMALL {
            // KVTAB_OK: the value is empty and "fitted". Otherwise: an error.
            return Error::check(status).map(|()| Vec::new());
        }
        let mut value: Vec<u8> = Vec::with_capacity(len);
        let mut written = value.capacity();
        // SAFETY: as above, and `value.as_mut_ptr()` is valid for writes of
        // `capacity()` bytes, which is the size that `written` announces.
        // Only a raw pointer to the uninitialised bytes exists; no reference
        // to them is created.
        let status = unsafe {
            sys::kvtab_get(
                self.as_raw(),
                key.as_ptr(),
                value.as_mut_ptr(),
                &mut written,
            )
        };
        // The table cannot have changed between the two calls: changing it
        // needs `&mut self`, and this method holds `&self`.
        Error::check(status)?;
        // A length that comes from foreign code is checked before it is used
        // in an unsafe operation.
        assert!(
            written <= value.capacity(),
            "kvtab_get reported more bytes than the buffer holds"
        );
        // SAFETY: the call returned KVTAB_OK, so the library initialised the
        // first `written` bytes, and `written <= capacity` was just checked.
        unsafe { value.set_len(written) };
        Ok(value)
    }

    /// D. A view of the stored value of `key`, without copying it. `None`
    /// when the key is missing. An empty value is `Some(&[])`.
    ///
    /// The library says that the pointer of `kvtab_peek` is valid until the
    /// next `kvtab_put`, `kvtab_remove` or `kvtab_free` on the table. The
    /// signature turns that sentence into something the compiler checks:
    ///
    /// ```compile_fail,E0502
    /// use m6_ffi::l04_safe_wrapper::Table;
    ///
    /// let mut table = Table::new(1).unwrap();
    /// table.put(c"k", b"old").unwrap();
    /// let value = table.peek(c"k").unwrap();
    /// table.put(c"k", b"new").unwrap(); // would free the bytes behind `value`
    /// assert_eq!(value, b"old");
    /// ```
    pub fn peek(&self, key: &CStr) -> Option<&[u8]> {
        let mut len = 0usize;
        // SAFETY: live table, NUL-terminated key, and `&mut len` is valid
        // for one write.
        let value = unsafe { sys::kvtab_peek(self.as_raw(), key.as_ptr(), &mut len) };
        if value.is_null() {
            return None;
        }
        // SAFETY: kvtab.h: a non-null result points to `len` bytes that stay
        // valid and unchanged until the next kvtab_put, kvtab_remove or
        // kvtab_free on this table. Each of those is reached only through
        // `&mut self` (`put`, `remove`) or by giving up the `Table` (`Drop`,
        // `into_raw`), and none of that is possible while the returned slice
        // borrows `&self`. The pointer is never NULL for an existing key,
        // also for an empty value, and the bytes are one allocation.
        Some(unsafe { std::slice::from_raw_parts(value, len) })
    }

    /// E. `put` for Rust strings. The key becomes a C string, so it must not
    /// contain a NUL byte: that is `Error::InvalidKey`, and the table is not
    /// changed. The value is stored as its UTF-8 bytes, without a terminator;
    /// it may contain NUL bytes, because values have an explicit length.
    pub fn put_str(&mut self, key: &str, value: &str) -> Result<(), Error> {
        // `CString::new` copies the key, appends the NUL, and refuses a key
        // with a NUL inside: C would silently stop reading there.
        let key = CString::new(key).map_err(|_| Error::InvalidKey)?;
        self.put(&key, value.as_bytes())
    }

    /// E. The value of `key` as a `String`. A key with a NUL byte is
    /// `Error::InvalidKey`; a missing key is `Error::NotFound`. Values are
    /// arbitrary bytes: replace invalid UTF-8 with U+FFFD, as
    /// `String::from_utf8_lossy` does. Copy the bytes only once.
    pub fn get_string(&self, key: &str) -> Result<String, Error> {
        let key = CString::new(key).map_err(|_| Error::InvalidKey)?;
        // Borrow the stored bytes and decode from there: one copy, made by
        // `into_owned`.
        let bytes = self.peek(&key).ok_or(Error::NotFound)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// F. Encodes `text` as a NUL-terminated UTF-16 string, the form that the
/// Windows API expects (`LPCWSTR`). The result ends with one 0 unit. A text
/// that contains U+0000 cannot be represented and gives `None`.
///
/// This task and the next are pure Rust and behave the same on every
/// platform.
pub fn to_wide(text: &str) -> Option<Vec<u16>> {
    if text.contains('\0') {
        return None;
    }
    Some(text.encode_utf16().chain(std::iter::once(0)).collect())
}

/// F. Decodes a NUL-terminated UTF-16 string that foreign code handed over.
/// An unpaired surrogate becomes U+FFFD. The terminator is not part of the
/// result.
///
/// # Safety
///
/// `text` must be non-null, aligned for `u16`, and point to a sequence of
/// initialised `u16` units that ends with a 0 unit. The memory up to and
/// including that 0 must be readable and must not be written during the call.
pub unsafe fn from_wide(text: *const u16) -> String {
    let mut len = 0;
    // SAFETY: the caller promises readable, initialised units up to and
    // including the first 0. The loop stops at the first 0, so it never
    // reads past it.
    while unsafe { *text.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: the `len` units before the terminator were just read, so they
    // are readable and initialised, and the caller promises that nobody
    // writes them during this call.
    let units = unsafe { std::slice::from_raw_parts(text, len) };
    String::from_utf16_lossy(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;
    use crate::sys;
    // Imported here too, so that the tests do not depend on which imports
    // your implementation above still needs.
    use std::ffi::{CStr, CString, c_int};

    fn table_with(entries: &[(&CStr, &[u8])]) -> Table {
        let mut table = Table::new(entries.len().max(1)).unwrap();
        for (key, value) in entries {
            table.put(key, value).unwrap();
        }
        table
    }

    #[test]
    fn a_version_is_the_library_text() {
        assert_eq!(version(), "kvtab 1.0.0");
    }

    #[test]
    fn a_technique_version_borrows_the_static_string() {
        // SAFETY: kvtab_version takes no arguments.
        let library = unsafe { sys::kvtab_version() };
        // Miri gives a `c"..."` literal a new address at each use, so the
        // address is only compared in a normal build.
        if !cfg!(miri) {
            assert!(
                std::ptr::eq(version().as_ptr(), library.cast::<u8>()),
                "version must borrow the library's static string, not a copy of it"
            );
        }
        let (_, allocations) = count(version);
        assert_eq!(
            allocations, 0,
            "borrowing a static string needs no allocation"
        );
    }

    #[test]
    fn b_get_into_copies_a_value_that_fits() {
        let table = table_with(&[(c"k", b"abc")]);
        let mut exact = [0u8; 3];
        assert_eq!(table.get_into(c"k", &mut exact), Ok(3));
        assert_eq!(&exact, b"abc");

        let mut roomy = [0xAAu8; 8];
        assert_eq!(table.get_into(c"k", &mut roomy), Ok(3));
        assert_eq!(&roomy[..3], b"abc");
        assert_eq!(
            roomy[3..],
            [0xAA; 5],
            "bytes after the value must stay as they were"
        );
    }

    #[test]
    fn b_get_into_reports_the_needed_size() {
        let table = table_with(&[(c"k", b"0123456789")]);
        let mut small = [0x55u8; 4];
        assert_eq!(
            table.get_into(c"k", &mut small),
            Err(Error::BufferTooSmall { needed: 10 }),
            "the library writes the size of the value into *len"
        );
        assert_eq!(
            small, [0x55; 4],
            "nothing is copied when the value does not fit"
        );
        assert_eq!(
            table.get_into(c"k", &mut []),
            Err(Error::BufferTooSmall { needed: 10 })
        );
    }

    #[test]
    fn b_get_into_edge_cases() {
        let table = table_with(&[(c"empty", b"")]);
        assert_eq!(
            table.get_into(c"empty", &mut []),
            Ok(0),
            "an empty value fits into an empty buffer"
        );
        assert_eq!(
            table.get_into(c"missing", &mut [0u8; 4]),
            Err(Error::NotFound)
        );
    }

    #[test]
    fn c_get_returns_the_bytes() {
        let binary: &[u8] = &[0, 255, 0, 7, b'x', 0];
        let table = table_with(&[(c"text", b"hello"), (c"bin", binary), (c"empty", b"")]);
        assert_eq!(table.get(c"text"), Ok(b"hello".to_vec()));
        assert_eq!(
            table.get(c"bin").as_deref(),
            Ok(binary),
            "values are bytes with a length: a 0 byte does not end them"
        );
        assert_eq!(table.get(c"empty"), Ok(Vec::new()));
        assert_eq!(table.get(c"missing"), Err(Error::NotFound));
    }

    #[test]
    fn c_technique_get_allocates_once_and_only_when_needed() {
        let big = vec![7u8; 1000];
        let table = table_with(&[(c"big", &big), (c"empty", b"")]);

        let (value, allocations) = count(|| table.get(c"big"));
        assert_eq!(value.as_deref(), Ok(&big[..]));
        assert!(
            allocations <= 1,
            "get made {allocations} allocations: ask the library for the size \
             first, then allocate once"
        );

        let (missing, allocations) = count(|| table.get(c"missing"));
        assert_eq!(missing, Err(Error::NotFound));
        assert_eq!(allocations, 0, "a missing key must not cost an allocation");

        let (empty, allocations) = count(|| table.get(c"empty"));
        assert_eq!(empty, Ok(Vec::new()));
        assert_eq!(allocations, 0, "an empty value needs no allocation");
    }

    #[test]
    fn d_peek_shows_the_stored_bytes() {
        let table = table_with(&[(c"a", b"alpha"), (c"e", b"")]);
        assert_eq!(table.peek(c"a"), Some(&b"alpha"[..]));
        assert_eq!(
            table.peek(c"e"),
            Some(&[][..]),
            "an empty value exists; it is not the same as a missing key"
        );
        assert_eq!(table.peek(c"missing"), None);
    }

    #[test]
    fn d_technique_peek_does_not_copy() {
        let table = table_with(&[(c"a", b"alpha")]);
        let mut len = 0usize;
        // SAFETY: the table is live and no other call is in progress.
        let stored = unsafe { sys::kvtab_peek(table.as_raw(), c"a".as_ptr(), &mut len) };
        let (view, allocations) = count(|| table.peek(c"a"));
        let view = view.unwrap();
        assert!(
            std::ptr::eq(view.as_ptr(), stored),
            "peek must return the table's own bytes, not a copy"
        );
        assert_eq!(view.len(), len);
        assert_eq!(allocations, 0);
    }

    #[test]
    fn d_two_views_can_exist_together() {
        let table = table_with(&[(c"a", b"1"), (c"b", b"22")]);
        let (a, b) = (table.peek(c"a").unwrap(), table.peek(c"b").unwrap());
        assert_eq!((a, b, table.len()), (&b"1"[..], &b"22"[..], 2));
    }

    #[test]
    fn e_strings_round_trip() {
        let mut table = Table::new(3).unwrap();
        table.put_str("greeting", "h\u{e9}llo").unwrap();
        table.put_str("", "empty key").unwrap();
        table.put_str("nul inside", "a\0b").unwrap();
        assert_eq!(table.get_string("greeting").as_deref(), Ok("h\u{e9}llo"));
        assert_eq!(table.get_string("").as_deref(), Ok("empty key"));
        assert_eq!(
            table.get_string("nul inside").as_deref(),
            Ok("a\0b"),
            "a value has an explicit length, so it may contain NUL bytes"
        );
        assert_eq!(
            table.peek(c"greeting"),
            Some("h\u{e9}llo".as_bytes()),
            "the value is stored as UTF-8 bytes without a terminator"
        );
        assert_eq!(table.get_string("missing"), Err(Error::NotFound));
    }

    #[test]
    fn e_a_key_with_a_nul_byte_is_refused() {
        let mut table = Table::new(2).unwrap();
        assert_eq!(
            table.put_str("ab\0cd", "value"),
            Err(Error::InvalidKey),
            "C would read this key as `ab`: refuse it instead of storing a different key"
        );
        assert_eq!(table.len(), 0, "a refused key must not change the table");
        assert_eq!(table.get_string("ab\0cd"), Err(Error::InvalidKey));
    }

    #[test]
    fn e_get_string_replaces_invalid_utf8() {
        let table = table_with(&[(c"bytes", &[b'f', 0xFF, b'o'])]);
        assert_eq!(table.get_string("bytes").as_deref(), Ok("f\u{FFFD}o"));
    }

    #[test]
    fn e_technique_get_string_copies_the_value_once() {
        let long = "x".repeat(1000);
        let mut table = Table::new(1).unwrap();
        table.put_str("long", &long).unwrap();
        let (text, allocations) = count(|| table.get_string("long"));
        assert_eq!(text.as_deref(), Ok(long.as_str()));
        assert!(
            allocations <= 2,
            "get_string made {allocations} allocations. One for the key as a C \
             string and one for the result are enough: decode the bytes where \
             they are, or reuse the buffer that you read them into"
        );
    }

    #[test]
    fn f_to_wide_encodes_and_terminates() {
        assert_eq!(to_wide(""), Some(vec![0]));
        assert_eq!(to_wide("hi"), Some(vec![0x68, 0x69, 0]));
        assert_eq!(
            to_wide("h\u{e9}"),
            Some(vec![0x68, 0xE9, 0]),
            "U+00E9 is two bytes in UTF-8 and one unit in UTF-16"
        );
        assert_eq!(
            to_wide("\u{1D11E}"),
            Some(vec![0xD834, 0xDD1E, 0]),
            "a character above U+FFFF is a surrogate pair"
        );
        assert_eq!(
            to_wide("a\0b"),
            None,
            "a NUL inside the text would end the string early on the other side"
        );
    }

    #[test]
    fn f_from_wide_decodes_up_to_the_terminator() {
        let hello = [0x68u16, 0xE9, 0x6C, 0x6C, 0x6F, 0, 0x21, 0];
        // SAFETY: the array contains a 0 unit.
        assert_eq!(unsafe { from_wide(hello.as_ptr()) }, "h\u{e9}llo");
        // SAFETY: the array is a single 0 unit.
        assert_eq!(unsafe { from_wide([0u16].as_ptr()) }, "");
        let unpaired = [0x61u16, 0xD800, 0x62, 0];
        // SAFETY: the array contains a 0 unit.
        let decoded = unsafe { from_wide(unpaired.as_ptr()) };
        assert_eq!(
            decoded, "a\u{FFFD}b",
            "an unpaired surrogate is not a character: replace it"
        );
    }

    #[test]
    fn f_wide_strings_round_trip() {
        for text in ["", "plain", "h\u{e9}llo w\u{f6}rld", "\u{1D11E} clef"] {
            let wide = to_wide(text).unwrap();
            assert_eq!(
                wide.last(),
                Some(&0),
                "to_wide must end its result with a 0 unit; without it, \
                 from_wide would read past the end of the vector"
            );
            // SAFETY: the vector ends with a 0 unit, as just checked.
            let decoded = unsafe { from_wide(wide.as_ptr()) };
            assert_eq!(decoded, text);
        }
    }

    #[test]
    fn p_predictions() {
        // P1. The text "h\u{e9}llo" has five characters. How many bytes does
        // it need as a C string, with the NUL? How many u16 units does it
        // need as a wide string, with the 0 unit?
        let text = "h\u{e9}llo";
        let as_c = CString::new(text).unwrap().as_bytes_with_nul().len();
        let as_wide = text.encode_utf16().count() + 1;
        let p1: (usize, usize) = (7, 6);
        assert_eq!(p1, (as_c, as_wide), "P1: the same text in two encodings");

        // P2. `CString::new` refuses bytes with a NUL inside. At which index
        // does it find the NUL in "ab\0cd\0"?
        let position = CString::new("ab\0cd\0").unwrap_err().nul_position();
        let p2: usize = 2;
        assert_eq!(p2, position, "P2: where a C reader would stop");

        // P3. How many bytes does this C string have, without its NUL?
        let length = CStr::from_bytes_until_nul(b"ab\0cd\0")
            .unwrap()
            .to_bytes()
            .len();
        let p3: usize = 2;
        assert_eq!(p3, length, "P3: a C string ends at the first NUL");

        // P4. A value of 5 bytes is stored. kvtab_get is called with a NULL
        // buffer and `len = 0`. Write (the status, the value of len afterwards).
        let mut len = 0usize;
        // SAFETY: the table is created, used and freed here; the key is
        // NUL-terminated; a NULL buffer is allowed when *len is 0.
        let status = unsafe {
            let table = sys::kvtab_new(1);
            sys::kvtab_put(table, c"k".as_ptr(), b"12345".as_ptr(), 5);
            let status = sys::kvtab_get(table, c"k".as_ptr(), std::ptr::null_mut(), &mut len);
            sys::kvtab_free(table);
            status
        };
        let p4: (c_int, usize) = (4, 5);
        assert_eq!(
            p4,
            (status, len),
            "P4: the first call of the two-call protocol"
        );
    }
}
