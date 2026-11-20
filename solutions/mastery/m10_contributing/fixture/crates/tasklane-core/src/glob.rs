//! Path patterns for the steps that work on several files.

/// Whether `path` matches `pattern`.
///
/// - `*` matches any number of characters, including none, but never a `/`.
/// - `?` matches exactly one character, but never a `/`.
/// - Every other character matches itself.
///
/// The whole path must match: `*.txt` does not match `a.txt.bak`.
///
/// ```
/// use tasklane_core::glob::matches;
///
/// assert!(matches("docs/*.md", "docs/intro.md"));
/// assert!(!matches("docs/*.md", "docs/api/intro.md"));
/// assert!(matches("v?.txt", "v2.txt"));
/// ```
pub fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let path: Vec<char> = path.chars().collect();
    let mut p = 0;
    let mut t = 0;
    // The position after the last `*` seen, and how far into `path` that star reaches.
    // When the rest of the pattern fails, the star takes one more character and the
    // match continues from there. Only the last star ever needs to grow: giving more
    // to an earlier star cannot help, because the text between the two stars was
    // matched at its leftmost possible place.
    let mut star: Option<(usize, usize)> = None;
    while t < path.len() {
        match pattern.get(p) {
            Some('*') => {
                p += 1;
                star = Some((p, t));
            }
            Some('?') if path[t] != '/' => {
                p += 1;
                t += 1;
            }
            Some(literal) if *literal == path[t] => {
                p += 1;
                t += 1;
            }
            _ => match star {
                Some((after_star, reach)) if path[reach] != '/' => {
                    star = Some((after_star, reach + 1));
                    p = after_star;
                    t = reach + 1;
                }
                _ => return false,
            },
        }
    }
    // The path is used up. Whatever is left of the pattern must match nothing.
    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals() {
        assert!(matches("a/b.txt", "a/b.txt"));
        assert!(!matches("a/b.txt", "a/b.txt2"));
        assert!(!matches("a/b.txt", "a/b.tx"));
        assert!(matches("", ""));
    }

    #[test]
    fn star_stays_inside_one_segment() {
        assert!(matches("*", "readme"));
        assert!(matches("*", ""));
        assert!(!matches("*", "a/b"));
        assert!(matches("build/*", "build/out.txt"));
        assert!(!matches("build/*", "build/cache/out.txt"));
    }

    #[test]
    fn star_with_suffix() {
        assert!(matches("*.md", "intro.md"));
        assert!(!matches("*.md", "intro.txt"));
        assert!(matches("build/*.tmp", "build/a.tmp"));
    }

    #[test]
    fn star_gives_back_what_the_rest_needs() {
        // Regression tests for #25.
        assert!(matches("*.md", "notes.md.md"));
        assert!(matches("a*b", "aXbYb"));
        assert!(matches("*?", "ab"));
        assert!(matches("*a*b", "aab"));
        assert!(!matches("*.md", "notes.md.txt"));
        assert!(!matches("a*b/c", "aXb/Yb/c"));
    }

    #[test]
    fn question_mark() {
        assert!(matches("v?", "v1"));
        assert!(!matches("v?", "v"));
        assert!(!matches("v?", "v12"));
        assert!(!matches("a?b", "a/b"));
    }
}
