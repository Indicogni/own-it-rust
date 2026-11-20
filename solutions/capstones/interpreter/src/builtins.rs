//! The built-in functions (SPEC section 10): their names and arities.
//!
//! What they do is in `interp.rs`, because several need the evaluator's
//! input and output.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Print,
    Input,
    Len,
    Push,
    Pop,
    Str,
    Int,
    Type,
    Exit,
}

impl Builtin {
    /// Every built-in function. The position in this array is the slot of
    /// the function in the outermost environment.
    pub const ALL: [Builtin; 9] = [
        Builtin::Print,
        Builtin::Input,
        Builtin::Len,
        Builtin::Push,
        Builtin::Pop,
        Builtin::Str,
        Builtin::Int,
        Builtin::Type,
        Builtin::Exit,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Print => "print",
            Builtin::Input => "input",
            Builtin::Len => "len",
            Builtin::Push => "push",
            Builtin::Pop => "pop",
            Builtin::Str => "str",
            Builtin::Int => "int",
            Builtin::Type => "type",
            Builtin::Exit => "exit",
        }
    }

    /// The number of arguments, or `None` for "any number".
    pub fn arity(self) -> Option<usize> {
        match self {
            Builtin::Print => None,
            Builtin::Input => Some(0),
            Builtin::Push => Some(2),
            Builtin::Len
            | Builtin::Pop
            | Builtin::Str
            | Builtin::Int
            | Builtin::Type
            | Builtin::Exit => Some(1),
        }
    }
}

/// `int("...")`: an optional `-`, then ASCII digits, nothing else.
pub fn parse_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // `parse` can still fail, on overflow.
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_accepts_only_plain_decimal() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("-0"), Some(0));
        assert_eq!(parse_int("007"), Some(7));
        assert_eq!(parse_int("-9223372036854775808"), Some(i64::MIN));
        for bad in [
            "",
            "-",
            "+5",
            " 5",
            "5 ",
            "1_000",
            "0x10",
            "1.0",
            "--1",
            "٣",
            "9223372036854775808",
        ] {
            assert_eq!(parse_int(bad), None, "{bad:?}");
        }
    }
}
