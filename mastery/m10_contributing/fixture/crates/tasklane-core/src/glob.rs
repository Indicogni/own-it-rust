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
    while p < pattern.len() {
        match pattern[p] {
            '*' => {
                // The star takes everything up to the character that follows it in
                // the pattern.
                let stop = pattern.get(p + 1).copied();
                while t < path.len() && path[t] != '/' && Some(path[t]) != stop {
                    t += 1;
                }
            }
            '?' => {
                if t >= path.len() || path[t] == '/' {
                    return false;
                }
                t += 1;
            }
            literal => {
                if t >= path.len() || path[t] != literal {
                    return false;
                }
                t += 1;
            }
        }
        p += 1;
    }
    t == path.len()
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
    fn question_mark() {
        assert!(matches("v?", "v1"));
        assert!(!matches("v?", "v"));
        assert!(!matches("v?", "v12"));
        assert!(!matches("a?b", "a/b"));
    }
}
