//! REVIEW 01 — Spaced retrieval: earlier chapters inside Chapter 17
//! Run: cargo test --test chapter17 r01_
//!
//! Do this WITHOUT reopening earlier chapters or their hints. Struggling to
//! recall is what makes the knowledge stick; look things up only after a real
//! attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (tree maps, iterator adapters, error enums
//! with `?`) with this chapter's text handling.

use std::collections::BTreeMap;
use std::num::ParseIntError;
use std::str::FromStr;

/// A. (Ch16 tree maps + Ch17 chars) Group words that are anagrams of each
/// other. A group's key is the word's Unicode scalars sorted ascending and
/// collected into a String. Each group keeps input order and borrows its words.
/// No normalization: "é" and "e\u{301}" are different scalars.
fn anagram_groups<'a>(words: &[&'a str]) -> BTreeMap<String, Vec<&'a str>> {
    let mut groups: BTreeMap<String, Vec<&'a str>> = BTreeMap::new();
    for &word in words {
        let mut scalars: Vec<char> = word.chars().collect();
        scalars.sort_unstable();
        let key: String = scalars.into_iter().collect();
        groups.entry(key).or_default().push(word);
    }
    groups
}

/// B. (Ch15 iterator adapters + Ch17 case mapping) Return the uppercased first
/// scalar of each whitespace-separated word, concatenated. Keep one-to-many
/// mappings whole: "ßen" contributes "SS".
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

#[derive(Debug, PartialEq)]
enum Command {
    Move(i32, i32),
    Say(String),
    Quit,
}

#[derive(Debug, PartialEq)]
enum CommandError {
    /// The input was blank.
    Empty,
    /// The first word is not a known command; holds that word.
    Unknown(String),
    /// A known command with the wrong number of arguments.
    WrongArity,
    /// A move argument that is not an i32.
    BadNumber(ParseIntError),
}

/// C1. (Ch7 From for `?`) Wrap the parse error so `?` can convert it.
impl From<ParseIntError> for CommandError {
    fn from(err: ParseIntError) -> Self {
        CommandError::BadNumber(err)
    }
}

impl FromStr for Command {
    type Err = CommandError;
    /// C2. (Ch7 `?` + Ch10 enums + Ch17 parsing) Trim the input, then parse
    /// "move DX DY" (exactly two i32 words), "say TEXT" (TEXT is the rest of
    /// the line after the first word, trimmed, nonempty, inner spaces kept), or
    /// "quit" (no arguments). Command words are case-sensitive and separated by
    /// any whitespace. Convert number errors with `?`.
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let line = line.trim();
        if line.is_empty() {
            return Err(CommandError::Empty);
        }
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match word {
            "move" => {
                // Check the number of words first, so that "move 1 2 x" is an
                // arity error and not a number error.
                let mut args = rest.split_whitespace();
                let (Some(dx), Some(dy), None) = (args.next(), args.next(), args.next()) else {
                    return Err(CommandError::WrongArity);
                };
                Ok(Command::Move(dx.parse()?, dy.parse()?))
            }
            "say" if !rest.is_empty() => Ok(Command::Say(rest.to_owned())),
            "quit" if rest.is_empty() => Ok(Command::Quit),
            "say" | "quit" => Err(CommandError::WrongArity),
            unknown => Err(CommandError::Unknown(unknown.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_anagrams_group_by_sorted_scalars() {
        let groups = anagram_groups(&["listen", "tée", "silent", "a", "éte", "enlist", "e\u{301}"]);
        assert_eq!(groups.len(), 4);
        assert_eq!(groups["eilnst"], ["listen", "silent", "enlist"]);
        assert_eq!(groups["eté"], ["tée", "éte"]);
        assert_eq!(groups["a"], ["a"]);
        assert_eq!(groups["e\u{301}"], ["e\u{301}"]);
        assert!(anagram_groups(&[]).is_empty());
        assert_eq!(anagram_groups(&[""])[""], [""]);
    }

    #[test]
    fn b_initials_uppercase_whole_mappings() {
        assert_eq!(initials("émile zola"), "ÉZ");
        assert_eq!(initials("  ada \t lovelace  "), "AL");
        assert_eq!(initials("ßen yu"), "SSY");
        assert_eq!(initials("🦀 rust"), "🦀R");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn c_commands_parse_with_typed_errors() {
        assert_eq!(" move -3 4 ".parse(), Ok(Command::Move(-3, 4)));
        assert_eq!(
            "say  hi  there ".parse(),
            Ok(Command::Say("hi  there".into()))
        );
        assert_eq!("say\té".parse(), Ok(Command::Say("é".into())));
        assert_eq!("quit".parse(), Ok(Command::Quit));
        assert_eq!("  ".parse::<Command>(), Err(CommandError::Empty));
        assert_eq!(
            "Quit".parse::<Command>(),
            Err(CommandError::Unknown("Quit".into()))
        );
        for line in ["move 1", "move 1 2 3", "say", "say   ", "quit now"] {
            assert_eq!(
                line.parse::<Command>(),
                Err(CommandError::WrongArity),
                "{line:?}"
            );
        }
        assert_eq!(
            "move 1 x".parse::<Command>(),
            Err(CommandError::BadNumber("x".parse::<i32>().unwrap_err()))
        );
        assert_eq!(
            "move 9999999999 0".parse::<Command>(),
            Err(CommandError::BadNumber(
                "9999999999".parse::<i32>().unwrap_err()
            ))
        );
    }

    /// P. Earlier-chapter recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1 (Ch16). Counting with entry into a BTreeMap, then iterating it.
        let mut counts = BTreeMap::new();
        for w in ["b", "a", "b"] {
            *counts.entry(w).or_insert(0) += 1;
        }
        let p1: Vec<(&str, i32)> = vec![("a", 1), ("b", 2)];
        assert_eq!(
            p1,
            counts.into_iter().collect::<Vec<_>>(),
            "P1: entry + order"
        );

        // P2 (Ch16). HashSet::insert twice, then the length.
        let mut set = std::collections::HashSet::new();
        let p2: (bool, bool, usize) = (true, false, 1);
        assert_eq!(
            p2,
            (set.insert(1), set.insert(1), set.len()),
            "P2: set insert"
        );

        // P3 (Ch15). fold, and chunks of a 4-element array.
        let p3: (i32, usize) = (123, 2);
        assert_eq!(
            p3,
            (
                (1..4).fold(0, |acc, x| acc * 10 + x),
                [1, 2, 3, 4].chunks(3).count()
            ),
            "P3: fold and chunks"
        );

        // P4 (Ch3). Remainder versus Euclidean remainder of -7 by 3.
        let p4: (i32, i32) = (-1, 2);
        assert_eq!(p4, (-7 % 3, (-7i32).rem_euclid(3)), "P4: % vs rem_euclid");
    }
}
