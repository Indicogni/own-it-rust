//! Short names for addresses and files, made from titles.
//!
//! A *slug* is the part of an address that names a page:
//! `hello-world` in `example.org/posts/hello-world`. This crate makes slugs
//! with [`slug`], shortens them with [`truncate`], and checks hand-written
//! labels with [`Tag::parse`].
//!
//! # Example
//!
//! ```
//! let name = slugger::slug("Hello, World!");
//! assert_eq!(name, "hello-world");
//! assert_eq!(slugger::truncate(&name, 8), "hello-wo");
//!
//! let tag = slugger::Tag::parse("rust-2024")?;
//! assert_eq!(tag.as_str(), "rust-2024");
//! # Ok::<(), slugger::TagError>(())
//! ```

mod imp;

/// Turns a title into a slug.
///
/// ASCII letters and digits are kept, and letters become lower case. Every
/// run of other characters becomes one `-`. The result never starts or ends
/// with `-`, so a title without letters and digits gives an empty slug.
///
/// # Examples
///
/// ```
/// use slugger::slug;
///
/// assert_eq!(slug("Hello, World!"), "hello-world");
/// // A run of separators gives one dash, and none at the ends.
/// assert_eq!(slug("  Rust --  2024  "), "rust-2024");
/// assert_eq!(slug("Done!"), "done");
/// assert_eq!(slug("?!"), "");
/// ```
pub fn slug(title: &str) -> String {
    imp::slug(title)
}

/// Shortens a slug to at most `max` bytes.
///
/// The result is the longest prefix of `slug` that has at most `max` bytes
/// and does not end with `-`. No allocation is made: the result borrows from
/// `slug`.
///
/// # Panics
///
/// Panics if `max` is 0: no useful slug has length 0.
///
/// ```should_panic
/// slugger::truncate("hello-world", 0);
/// ```
///
/// # Examples
///
/// ```
/// use slugger::truncate;
///
/// assert_eq!(truncate("hello-world", 8), "hello-wo");
/// // Cutting after the dash would leave `hello-`; the dash goes too.
/// assert_eq!(truncate("hello-world", 6), "hello");
/// assert_eq!(truncate("hello", 100), "hello");
/// ```
pub fn truncate(slug: &str, max: usize) -> &str {
    imp::truncate(slug, max)
}

/// A label that was checked by [`Tag::parse`].
///
/// A value of this type always holds 1 to 20 characters, each of them a lower
/// case ASCII letter, a digit or `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag(String);

impl Tag {
    /// Checks `text` and wraps it.
    ///
    /// Unlike [`slug`], this function changes nothing: text that is not
    /// already a valid tag is an error.
    ///
    /// # Errors
    ///
    /// - [`TagError::Empty`] if `text` is empty.
    /// - [`TagError::TooLong`] if `text` has more than 20 characters.
    /// - [`TagError::InvalidChar`] with the first character that is not a
    ///   lower case ASCII letter, a digit or `-`.
    ///
    /// # Examples
    ///
    /// ```
    /// use slugger::{Tag, TagError};
    ///
    /// let tag = Tag::parse("rust-2024")?;
    /// assert_eq!(tag.as_str(), "rust-2024");
    ///
    /// assert_eq!(Tag::parse(""), Err(TagError::Empty));
    /// assert_eq!(Tag::parse("Rust"), Err(TagError::InvalidChar('R')));
    /// assert_eq!(Tag::parse("two words"), Err(TagError::InvalidChar(' ')));
    /// # Ok::<(), TagError>(())
    /// ```
    pub fn parse(text: &str) -> Result<Tag, TagError> {
        imp::check_tag(text)?;
        Ok(Tag(text.to_string()))
    }

    /// The text of the tag.
    ///
    /// ```
    /// let tag = slugger::Tag::parse("news")?;
    /// assert_eq!(tag.as_str(), "news");
    /// # Ok::<(), slugger::TagError>(())
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a text is not a valid [`Tag`].
///
/// More reasons may be added in a later version, so a `match` on this type
/// needs a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagError {
    /// The text is empty.
    Empty,
    /// The text has more characters than a tag may have.
    TooLong {
        /// The largest number of characters that is allowed.
        max: usize,
    },
    /// The text contains this character, which a tag cannot contain.
    InvalidChar(char),
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::Empty => f.write_str("a tag cannot be empty"),
            TagError::TooLong { max } => write!(f, "a tag has at most {max} characters"),
            TagError::InvalidChar(c) => write!(f, "a tag cannot contain {c:?}"),
        }
    }
}

impl std::error::Error for TagError {}
