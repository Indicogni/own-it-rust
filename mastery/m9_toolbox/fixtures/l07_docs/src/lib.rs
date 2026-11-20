//! Slugs.

/// Turns a title into a slug: lower-case ASCII letters and digits, words joined
/// by single dashes. Everything else separates words. See Slug for a checked
/// wrapper, and [`Slug::create`] for its constructor.
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
    /// Makes a slug from a title with slugify. None if nothing is left.
    pub fn new(title: &str) -> Option<Self> {
        let slug = slugify(title);
        if slug.is_empty() {
            None
        } else {
            Some(Self(slug))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Turns a slug back into a title: "own-it-hands-on-mastery-of-rust" becomes
/// "Own It Hands On Mastery Of Rust".
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
