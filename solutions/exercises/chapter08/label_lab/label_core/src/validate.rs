use crate::{LabelError, MAX_LEN};

/// 1A. Trim whitespace from both ends of `raw` and return the rest as a
/// borrow of `raw`. Return `Err(LabelError::Empty)` when nothing is left
/// and `Err(LabelError::TooLong { len })` when the trimmed text has more
/// than `MAX_LEN` characters; `len` is that number of characters.
// `pub(crate)`: the crate's other modules and its unit tests may call this
// function; no other crate can. Plain `pub` would behave the same today,
// because the module itself is private, but `pub(crate)` states the intent
// and stays correct if the module is ever made public.
pub(crate) fn clean(raw: &str) -> Result<&str, LabelError> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(LabelError::Empty);
    }
    // Count characters: `text.len()` would count UTF-8 bytes.
    let len = text.chars().count();
    if len > MAX_LEN {
        return Err(LabelError::TooLong { len });
    }
    Ok(text)
}
