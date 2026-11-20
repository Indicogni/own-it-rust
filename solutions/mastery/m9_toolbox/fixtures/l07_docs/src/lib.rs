#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Turns a title into a slug: lower-case ASCII letters and digits, words joined
/// by single dashes. Everything else separates words. See [`Slug`] for a checked
/// wrapper, and [`Slug::new`] for its constructor.
///
/// # Examples
///
/// ```
/// assert_eq!(slugs::slugify("Own It: Hands-On Mastery of Rust"), "own-it-hands-on-mastery-of-rust");
/// assert_eq!(slugs::slugify("a -- b"), "a-b");
/// assert_eq!(slugs::slugify("???"), "");
/// ```
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for word in title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if !slug.is_empty() {
            slug.push('-');
        }
        slug.push_str(&word.to_ascii_lowercase());
    }
    slug
}

/// A slug that is known to be non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    /// Makes a slug from a title with [`slugify`]. Returns `None` if the title
    /// has no ASCII letter or digit.
    ///
    /// # Examples
    ///
    /// ```
    /// use slugs::Slug;
    ///
    /// assert!(Slug::new("Chapter 1").is_some());
    /// assert!(Slug::new("...").is_none());
    /// ```
    pub fn new(title: &str) -> Option<Self> {
        let slug = slugify(title);
        if slug.is_empty() {
            None
        } else {
            Some(Self(slug))
        }
    }

    /// The slug as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Turns a slug back into a title: `"own-it-hands-on-mastery-of-rust"` becomes
/// `"Own It Hands On Mastery Of Rust"`.
///
/// The opposite direction is [`slugify`].
#[cfg(feature = "titles")]
pub fn title_case(slug: &Slug) -> String {
    let words: Vec<String> = slug
        .as_str()
        .split('-')
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + letters.as_str(),
                None => String::new(),
            }
        })
        .collect();
    words.join(" ")
}
