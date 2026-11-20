//! LESSON 11 — Building text and writing literals
//! Run: cargo test --test chapter03 l11_
//! Read first: CONCEPTS.md, section "Lesson 11".
//!
//! PREDICT: first answer `p_predictions` at the bottom (replace each `guess()`
//! with your answer). Then consider: What are the types of "hi", 'h',
//! "hi".to_string(), and r"\n"? How many bytes are in r"\n" versus "\n"? What
//! does String::pop return for a final emoji? What changes between Display and
//! Debug output for text?
//!
//! EXPLAIN: Why accept &str when reading input, and return String for newly
//! built text? Which value owns the UTF-8 buffer? Does `let mut s = "hi"`
//! turn that borrowed text into a growable owned string?
//!
//! COMPILER LAB: print a string containing a newline with `{}` and `{:?}`.
//! Try a multiline literal, then escape the source line break with a trailing
//! backslash. Compare included whitespace. Try embedding quotes in a raw string.

/// A. Build exactly: "Hi NAME, you have N unread messages."
/// Use format!, including when N is 1. Do not trim or alter the supplied name.
fn inbox_message(name: &str, unread: usize) -> String {
    todo!("A: build an owned String from borrowed input and a number")
}

/// B. Append a space, '[', the tag, and ']' to the existing String.
/// Use String's append methods; preserve all existing content.
// The signature is part of the contract: appending needs the String itself,
// so clippy's advice to take `&mut str` does not apply here.
#[allow(clippy::ptr_arg)]
fn append_tag(message: &mut String, tag: &str) {
    todo!("B: append both string slices and individual characters")
}

/// C. Trim each item and join the results with " | ". Preserve empty items.
/// Empty input yields an empty String. Collect the trimmed pieces (each one
/// is a borrowed `&str`) into a vector with a small loop, then call `join`.
/// Do not build a String for each item.
fn trimmed_list(items: &[&str]) -> String {
    todo!("C: borrow trimmed pieces, then allocate the joined text")
}

/// D. Remove and return the final Unicode scalar, or None for an empty String.
// The signature is part of the contract: removing needs the String itself.
#[allow(clippy::ptr_arg)]
fn remove_last(text: &mut String) -> Option<char> {
    todo!("D: remove a scalar without corrupting UTF-8")
}

/// E. Return the same text twice: Open "draft.txt" in C:\notes
/// First use an ordinary escaped literal; second use a raw literal with hashes.
/// The supplied 'static annotation describes literal text that lasts for the
/// program's lifetime. You do not need to design lifetimes in this exercise.
fn literal_forms() -> (&'static str, &'static str) {
    todo!("E: compare escaped and raw forms of identical text")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};

    #[test]
    fn a_format_builds_an_owned_result() {
        assert_eq!(
            inbox_message("Ada", 3),
            "Hi Ada, you have 3 unread messages."
        );
        assert_eq!(inbox_message("李", 1), "Hi 李, you have 1 unread messages.");
        assert_eq!(
            inbox_message(" Bo ", 0),
            "Hi  Bo , you have 0 unread messages."
        );
    }

    #[test]
    fn b_append_preserves_existing_utf8() {
        let mut message = "ready 🦀".to_string();
        append_tag(&mut message, "lab");
        assert_eq!(message, "ready 🦀 [lab]");
        append_tag(&mut message, "");
        assert_eq!(message, "ready 🦀 [lab] []");
    }

    #[test]
    fn c_trim_and_join_keep_empty_fields() {
        assert_eq!(
            trimmed_list(&["  alpha", "β  ", "\n gamma \t"]),
            "alpha | β | gamma"
        );
        assert_eq!(trimmed_list(&[" a ", "", " b "]), "a |  | b");
        assert_eq!(trimmed_list(&[]), "");
        assert_eq!(trimmed_list(&["   "]), "");
    }

    #[test]
    fn d_pop_removes_a_whole_scalar() {
        let mut text = "a🦀".to_string();
        assert_eq!(remove_last(&mut text), Some('🦀'));
        assert_eq!(text, "a");
        assert_eq!(remove_last(&mut text), Some('a'));
        assert_eq!(remove_last(&mut text), None);
        let mut combining = "e\u{301}".to_string();
        assert_eq!(remove_last(&mut combining), Some('\u{301}'));
        assert_eq!(combining, "e");
    }

    #[test]
    fn e_raw_and_escaped_literals_can_mean_the_same_text() {
        let (escaped, raw) = literal_forms();
        assert_eq!(escaped, "Open \"draft.txt\" in C:\\notes");
        assert_eq!(escaped, raw);
    }

    #[test]
    fn b_append_tag_extends_the_existing_buffer() {
        let mut message = String::with_capacity(64);
        message.push_str("ready");
        let buffer = message.as_ptr();
        let (_, allocations) = count(|| append_tag(&mut message, "lab"));
        assert_eq!(message, "ready [lab]");
        assert_eq!(
            message.as_ptr(),
            buffer,
            "append into the existing buffer; do not build a new String"
        );
        assert_eq!(
            allocations, 0,
            "the String had spare capacity, so appending must not allocate"
        );
    }

    #[test]
    fn c_trimmed_list_borrows_before_joining() {
        let items: Vec<String> = (0..20).map(|i| format!("  item{i} ")).collect();
        let items: Vec<&str> = items.iter().map(String::as_str).collect();
        let (joined, allocations) = count(|| trimmed_list(&items));
        assert!(joined.starts_with("item0 | item1 | "));
        // A list of 20 borrowed pieces costs at most a few allocations, even
        // when it grows step by step. One String per item would cost 20 more.
        assert!(
            allocations <= 6,
            "trim returns a borrowed piece, so no String per item is needed. \
             Expected at most 6 allocations (the list of pieces and the joined \
             result), got {allocations}"
        );
    }

    #[test]
    fn d_remove_last_does_not_allocate() {
        let mut text = String::from("ok🦀");
        let (last, allocations) = count(|| remove_last(&mut text));
        assert_eq!(last, Some('🦀'));
        assert_eq!(
            allocations, 0,
            "removing the last scalar shortens the String in place; nothing to allocate"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Byte lengths of a raw literal and an escaped literal.
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (r"\n".len(), "\n".len()),
            "P1: byte lengths of the two literals. Left = your prediction, right = what Rust computes."
        );

        // P2. The exact text that Debug formatting produces for "tab\t".
        let p2: String = guess();
        assert_eq!(
            p2,
            format!("{:?}", "tab\t"),
            "P2: Debug output. Left = your prediction, right = what Rust computes."
        );

        // P3. How many pieces does split produce here?
        let p3: usize = guess();
        assert_eq!(
            p3,
            "a,b,,c".split(',').count(),
            "P3: number of pieces. Left = your prediction, right = what Rust computes."
        );

        // P4. `+` takes the left String by value. Does the result reuse its
        // buffer when it already has room?
        let mut s = String::with_capacity(16);
        s.push_str("ab");
        let buffer = s.as_ptr();
        let joined = s + "cd";
        let p4: bool = guess();
        assert_eq!(
            p4,
            joined.as_ptr() == buffer,
            "P4: buffer reuse by `String + &str`. Left = your prediction, right = what Rust computes."
        );
    }
}
