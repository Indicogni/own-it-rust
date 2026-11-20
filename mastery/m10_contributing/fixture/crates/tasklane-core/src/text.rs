//! Small text helpers used by the parser: names, words and quoted arguments.

/// Whether `text` is a valid task, variable or step name: one or more ASCII letters,
/// digits, `_`, `-` or `.`.
pub fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Splits off the first word of `text`. Returns the word and the rest, with the
/// white space between them removed.
///
/// ```
/// use tasklane_core::text::split_word;
///
/// assert_eq!(split_word("step  write a.txt"), ("step", "write a.txt"));
/// assert_eq!(split_word("flags"), ("flags", ""));
/// ```
pub fn split_word(text: &str) -> (&str, &str) {
    match text.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim_start()),
        None => (text, ""),
    }
}

/// Splits step arguments. Arguments are separated by white space. Double quotes keep
/// white space inside one argument; inside quotes, `\"` is a quote and `\\` is a
/// backslash. An empty pair of quotes gives an empty argument.
///
/// The error is a message without a line number; the parser adds the line.
///
/// ```
/// use tasklane_core::text::split_args;
///
/// assert_eq!(
///     split_args(r#"write a.txt "two words" x"#).unwrap(),
///     ["write", "a.txt", "two words", "x"]
/// );
/// ```
pub fn split_args(text: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut in_quotes = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => in_quotes = false,
                '\\' => match chars.next() {
                    Some(escaped @ ('"' | '\\')) => current.push(escaped),
                    Some(other) => return Err(format!("unknown escape `\\{other}`")),
                    None => return Err("unterminated quote".to_string()),
                },
                other => current.push(other),
            }
        } else if c == '"' {
            in_quotes = true;
            in_word = true;
        } else if c.is_whitespace() {
            if in_word {
                args.push(std::mem::take(&mut current));
                in_word = false;
            }
        } else {
            current.push(c);
            in_word = true;
        }
    }
    if in_quotes {
        return Err("unterminated quote".to_string());
    }
    if in_word {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(is_name("build-all_2.0"));
        assert!(!is_name(""));
        assert!(!is_name("a b"));
        assert!(!is_name("caf\u{e9}"));
    }

    #[test]
    fn plain_words() {
        assert_eq!(split_args("  a  b\tc ").unwrap(), ["a", "b", "c"]);
        assert!(split_args("   ").unwrap().is_empty());
    }

    #[test]
    fn quotes_group_and_join() {
        assert_eq!(split_args(r#"a"b c"d"#).unwrap(), ["ab cd"]);
        assert_eq!(split_args(r#""" x"#).unwrap(), ["", "x"]);
        assert_eq!(
            split_args(r#""say \"hi\" \\ bye""#).unwrap(),
            [r#"say "hi" \ bye"#]
        );
    }

    #[test]
    fn quote_errors() {
        assert_eq!(split_args(r#"a "b"#), Err("unterminated quote".to_string()));
        assert_eq!(
            split_args(r#""\n""#),
            Err("unknown escape `\\n`".to_string())
        );
    }
}
