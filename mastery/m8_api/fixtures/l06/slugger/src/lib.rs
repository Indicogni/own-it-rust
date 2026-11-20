// Lesson 06: this library works and has no documentation. Write it.
// Do not change the signatures or the bodies; `src/imp.rs` holds the code.
// The contract is in `src/l06_docs.rs` of the lesson package.

mod imp;

pub fn slug(title: &str) -> String {
    imp::slug(title)
}

pub fn truncate(slug: &str, max: usize) -> &str {
    imp::truncate(slug, max)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag(String);

impl Tag {
    pub fn parse(text: &str) -> Result<Tag, TagError> {
        imp::check_tag(text)?;
        Ok(Tag(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagError {
    Empty,
    TooLong { max: usize },
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
