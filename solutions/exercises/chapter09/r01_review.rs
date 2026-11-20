//! REVIEW 01 — Spaced retrieval: Chapters 3, 5, 7 and 8 inside Chapter 9
//! Run: cargo test --test chapter09 r01_
//!
//! Do this WITHOUT reopening earlier chapters or their hints. Struggling to
//! recall is what makes the knowledge stick; look things up only after a real
//! attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (module privacy, error types, borrowed
//! lifetimes, UTF-8) with this chapter's structs and methods.

mod tags {
    /// Longest accepted tag, in bytes.
    pub const MAX_LEN: usize = 16;

    /// A tag is 1..=MAX_LEN bytes of ASCII lowercase letters, digits and '-'.
    /// The field is private, so code outside this module cannot build an
    /// invalid Tag; the tests below can only use the public API.
    #[derive(Debug, PartialEq)]
    pub struct Tag {
        text: String,
    }

    impl Tag {
        /// A. (Ch8 modules and consts + Ch9 constructors) Return Some(Tag)
        /// only for valid text, using MAX_LEN rather than a literal. Allocate
        /// at most once, and only for valid input.
        pub fn parse(text: &str) -> Option<Tag> {
            // Validate on the borrowed text; allocate only for a valid tag.
            let valid = (1..=MAX_LEN).contains(&text.len())
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
            valid.then(|| Tag {
                text: text.to_owned(),
            })
        }

        /// Borrow the tag's text.
        pub fn as_str(&self) -> &str {
            &self.text
        }
    }
}

#[derive(Debug, PartialEq)]
enum TransferError {
    Insufficient { available: u32 },
    WouldOverflow,
}

#[derive(Debug, PartialEq)]
struct Account {
    cents: u32,
}

impl Account {
    /// B. (Ch7 custom errors + Ch9 &mut self methods) Move `amount` from self
    /// to `to`. On failure return the matching error and change NEITHER
    /// account: check both sides before mutating. No wrapping arithmetic.
    fn transfer(&mut self, to: &mut Account, amount: u32) -> Result<(), TransferError> {
        // Work out both new balances before writing either one.
        let remaining = self
            .cents
            .checked_sub(amount)
            .ok_or(TransferError::Insufficient {
                available: self.cents,
            })?;
        let received = to
            .cents
            .checked_add(amount)
            .ok_or(TransferError::WouldOverflow)?;
        self.cents = remaining;
        to.cents = received;
        Ok(())
    }
}

/// C. (Ch3 UTF-8 + Ch5 lifetimes + Ch9 structs with lifetimes) A cursor over
/// borrowed text. `pos` is a byte offset that always sits on a char boundary.
struct Words<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Words<'a> {
    fn new(text: &'a str) -> Self {
        Words { text, pos: 0 }
    }

    /// Return the next whitespace-separated word (Unicode whitespace), or None
    /// at the end. The word borrows the source text for 'a, not the cursor.
    /// Do not allocate.
    fn next_word(&mut self) -> Option<&'a str> {
        // `self.text` is a `&'a str`, so slices of it live for 'a.
        let rest = self.text[self.pos..].trim_start();
        let start = self.text.len() - rest.len();
        if rest.is_empty() {
            self.pos = start;
            return None;
        }
        let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        // Byte offsets from trim_start and find always sit on char boundaries.
        self.pos = start + len;
        Some(&rest[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::tags::{MAX_LEN, Tag};
    use super::*;
    use practice::alloc::count;

    #[test]
    fn a_tag_enforces_its_invariant() {
        assert_eq!(Tag::parse("rust-2024").unwrap().as_str(), "rust-2024");
        assert_eq!(Tag::parse(&"a".repeat(MAX_LEN)).unwrap().as_str().len(), 16);
        assert!(Tag::parse(&"a".repeat(MAX_LEN + 1)).is_none());
        for bad in ["", "Rust", "a b", "naïve", "tag_1", "ｒｕｓｔ"] {
            assert!(Tag::parse(bad).is_none(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn a_tag_allocates_once_and_only_when_valid() {
        let (tag, allocations) = count(|| Tag::parse("ok"));
        assert!(tag.is_some());
        assert!(allocations <= 1, "one allocation, not {allocations}");
        let (tag, allocations) = count(|| Tag::parse("NOT OK"));
        assert!(tag.is_none());
        assert_eq!(allocations, 0, "validate before allocating");
    }

    #[test]
    fn b_transfer_is_all_or_nothing() {
        let mut a = Account { cents: 500 };
        let mut b = Account { cents: 10 };
        assert_eq!(a.transfer(&mut b, 200), Ok(()));
        assert_eq!((a.cents, b.cents), (300, 210));
        assert_eq!(
            a.transfer(&mut b, 301),
            Err(TransferError::Insufficient { available: 300 })
        );
        assert_eq!(
            (a.cents, b.cents),
            (300, 210),
            "a failed transfer must change neither account"
        );
        let mut full = Account { cents: u32::MAX };
        assert_eq!(a.transfer(&mut full, 1), Err(TransferError::WouldOverflow));
        assert_eq!(
            (a.cents, full.cents),
            (300, u32::MAX),
            "a failed transfer must change neither account"
        );
        assert_eq!(a.transfer(&mut b, 0), Ok(()));
        assert_eq!(a.transfer(&mut b, 300), Ok(()));
        assert_eq!((a.cents, b.cents), (0, 510));
    }

    #[test]
    fn c_words_borrow_the_source() {
        let text = String::from("  héllo\u{3000}wörld 🦀\n");
        let found: Vec<&str> = {
            let mut words = Words::new(&text);
            let mut found = Vec::new();
            while let Some(word) = words.next_word() {
                found.push(word);
                assert!(
                    found.len() <= 3,
                    "the text has 3 words but next_word keeps returning more: \
                     does it move `pos` past the word it returns?"
                );
            }
            assert_eq!(words.next_word(), None, "the end stays the end");
            found
        }; // the cursor is gone; the words still borrow `text`
        assert_eq!(found, ["héllo", "wörld", "🦀"]);
        let first = found[0];
        assert_eq!(
            first.as_ptr(),
            text[2..].as_ptr(),
            "a word must be a slice of the source text, not a copy"
        );
        assert_eq!(Words::new("").next_word(), None);
        assert_eq!(Words::new(" \t ").next_word(), None);
    }

    #[test]
    fn c_words_do_not_allocate() {
        let text = "one two three four";
        let (n, allocations) = count(|| {
            let mut words = Words::new(text);
            let mut n = 0;
            while words.next_word().is_some() {
                n += 1;
                assert!(n <= 4, "next_word must stop after the fourth word");
            }
            n
        });
        assert_eq!(n, 4);
        assert_eq!(allocations, 0, "words are borrowed slices");
    }

    /// P. Earlier-chapter recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1 (Ch8). The module path of this test function's module.
        let p1: &str = "chapter09::r01_review::tests";
        assert_eq!(p1, module_path!(), "P1: crate and module paths");

        // P2 (Ch7). Does `parse` accept a leading '+' or a leading space?
        let p2: (Option<u8>, Option<u8>) = (Some(7), None);
        assert_eq!(
            p2,
            ("+7".parse::<u8>().ok(), " 7".parse::<u8>().ok()),
            "P2: what parse accepts"
        );

        // P3 (Ch6). `as` casts between numeric types.
        let p3: (u8, u8, i32) = (44, 0, 3);
        assert_eq!(
            p3,
            (300_i32 as u8, -1.5_f64 as u8, 3.99_f64 as i32),
            "P3: truncating and saturating casts"
        );

        // P4 (Ch5). Equal values in different places.
        let (a, b) = (5, 5);
        let (ra, rb) = (&a, &b);
        let p4: (bool, bool) = (true, false);
        assert_eq!(
            p4,
            (ra == rb, std::ptr::eq(ra, rb)),
            "P4: == compares referents"
        );
    }
}
