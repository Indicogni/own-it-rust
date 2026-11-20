// `guess` is no longer used in a lesson once every prediction is answered, so
// unused imports are allowed too.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_blocks;
mod l02_branching;
mod l03_let_else;
mod l04_loops;
mod l05_labels;
mod l06_const_unsafe;
mod l07_operators;
mod l08_casts_closures;
mod l09_capstone;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;

/// Source checks. You do not need to edit this module.
///
/// Some tasks in this chapter name the construct to use (a labeled block, an
/// inline const block, `let else`, no `unsafe`). A behaviour test cannot see
/// syntax, so each of those tasks has one extra test that reads the lesson
/// file and looks inside the body of the task's own function.
mod source {
    /// The body of `fn name` in `file`, without comments and without the
    /// contents of string and char literals. Each run of whitespace becomes
    /// one space, so the checks do not depend on how the code is formatted.
    #[track_caller]
    pub fn body(file: &str, name: &str) -> String {
        let start = [format!("fn {name}("), format!("fn {name}<")]
            .iter()
            .filter_map(|needle| file.find(needle.as_str()))
            .min()
            .unwrap_or_else(|| panic!("`fn {name}` is missing; keep the function's name"));
        let chars: Vec<char> = file[start..].chars().collect();
        let mut out = String::new();
        let mut depth = 0usize;
        let mut i = 0;
        while i < chars.len() {
            let next = chars.get(i + 1).copied();
            match chars[i] {
                '/' if next == Some('/') => {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                '/' if next == Some('*') => {
                    i += 2;
                    while i + 1 < chars.len() && (chars[i], chars[i + 1]) != ('*', '/') {
                        i += 1;
                    }
                    i += 1;
                }
                '"' => {
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        i += if chars[i] == '\\' { 2 } else { 1 };
                    }
                    out.push_str("\"\"");
                }
                // A char literal such as '/' or '\n'. A label or lifetime
                // ('outer) has no closing quote two characters later.
                '\'' if next == Some('\\') || chars.get(i + 2) == Some(&'\'') => {
                    i += if next == Some('\\') { 3 } else { 2 };
                    while i < chars.len() && chars[i] != '\'' {
                        i += 1;
                    }
                    out.push_str("''");
                }
                '{' => {
                    depth += 1;
                    if depth > 1 {
                        out.push('{');
                    }
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                    out.push('}');
                }
                c if c.is_whitespace() => {
                    if depth > 0 && !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                c => {
                    if depth > 0 {
                        out.push(c);
                    }
                }
            }
            i += 1;
        }
        out
    }

    /// True if `word` appears in `code` as a whole keyword or identifier.
    pub fn has_word(code: &str, word: &str) -> bool {
        code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|found| found == word)
    }

    /// True if `code` contains `pattern` when spaces are ignored in both.
    pub fn has(code: &str, pattern: &str) -> bool {
        code.replace(' ', "").contains(&pattern.replace(' ', ""))
    }

    /// True if `code` contains a labeled block: `'name: { ... }`.
    pub fn has_labeled_block(code: &str) -> bool {
        let compact = code.replace(' ', "");
        compact.match_indices(":{").any(|(at, _)| {
            let before = compact[..at].trim_end_matches(|c: char| c.is_alphanumeric() || c == '_');
            before.len() < at && before.ends_with('\'')
        })
    }

    /// True if `code` contains `let PATTERN = VALUE else { ... }`.
    pub fn has_let_else(code: &str) -> bool {
        code.match_indices("let ").any(|(at, _)| {
            let statement = &code[at..];
            let mut depth = 0usize;
            for (i, c) in statement.char_indices() {
                match c {
                    // The `{` of the else block follows `else` directly, and
                    // the value before `else` cannot end in `}` (that would
                    // be the `else` of an ordinary `if`).
                    '{' if depth == 0 => {
                        let before = statement[..i].trim_end();
                        if let Some(value) = before.strip_suffix("else") {
                            let own_word =
                                !value.ends_with(|c: char| c.is_alphanumeric() || c == '_');
                            return own_word && !value.trim_end().ends_with('}');
                        }
                        depth += 1;
                    }
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' if depth == 0 => return false,
                    ')' | ']' | '}' => depth -= 1,
                    ';' if depth == 0 => return false,
                    _ => {}
                }
            }
            false
        })
    }
}
