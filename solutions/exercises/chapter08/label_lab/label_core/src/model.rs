use crate::validate;
use std::fmt;

/// Why some text cannot be a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelError {
    /// Nothing is left after trimming.
    Empty,
    /// The trimmed text has `len` characters, more than
    /// [`MAX_LEN`](crate::MAX_LEN).
    TooLong {
        /// Number of characters after trimming.
        len: usize,
    },
}

impl fmt::Display for LabelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabelError::Empty => write!(f, "label is empty"),
            LabelError::TooLong { len } => write!(
                f,
                "label has {len} characters; the limit is {}",
                crate::MAX_LEN
            ),
        }
    }
}

impl std::error::Error for LabelError {}

/// Label text that has passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    // Private: `Label::new` is the only way in, so every `Label` that exists
    // has been validated.
    text: String,
}

impl Label {
    /// 1B. Validate `raw` with `validate::clean` and own the cleaned text.
    pub fn new(raw: &str) -> Result<Label, LabelError> {
        let text = validate::clean(raw)?.to_owned();
        Ok(Label { text })
    }

    /// The validated text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// 1C. Return `"tag:"` followed by the text.
    pub fn tag(&self) -> String {
        format!("tag:{}", self.text)
    }
}
