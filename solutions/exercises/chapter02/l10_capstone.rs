//! LESSON 10 — Capstone: a text statistics tool
//! Run: cargo test --test chapter02 l10_
//! Read first: "Lesson 10" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. How many lines does the text "one\n" have? And the empty text?
//! 2. Which of your functions can fail, and what does each return then?
//!
//! IMPLEMENT the tool below. Only ONE function is given, and the tests call
//! only that one. Everything behind it is your design: the types, the helper
//! functions and their names. Write at least two tests of your own.
//! EXPLAIN afterward: Describe your design: which types and functions you
//! made, what each one is responsible for, and one thing you would change if
//! the tool had to grow to ten commands.
//!
//! COMPILER / TRANSFER LAB (optional): Turn the tool into a real program.
//! Outside the course directory, run `cargo new textstats`, copy your code
//! into its `src/main.rs`, and write a `main` that takes the command from the
//! program's arguments and the text from standard input. CONCEPTS.md,
//! "Lesson 10", shows the few lines that `main` needs.

/// Run one command of the text statistics tool on `text` and return what the
/// tool should print, or an error message.
///
/// A word is a run of characters without white space, as `split_whitespace`
/// gives them. The length of a word is its number of characters
/// (`chars().count()`), not its number of bytes.
///
/// | `command` | Result |
/// | --- | --- |
/// | `"count"` | `Ok` with three lines joined by `\n`: `lines: 2`, `words: 6`, `characters: 31` (your numbers). Characters include spaces and line ends. |
/// | `"longest"` | `Ok` with the longest word. When several words are equally long, the first of them. `Err("no words")` when the text has no word. |
/// | `"average"` | `Ok` with the average word length, one digit after the point: `average word length: 4.5`. `Err("no words")` when the text has no word. |
/// | `"shout"` | `Ok` with the text in capital letters. |
/// | anything else | `Err("unknown command: X")`, where X is the command as given. |
///
/// Design rules (a supplied test checks the first three):
/// 1. Define an `enum` for the four commands, and one function that turns
///    the command word into a value of it or into the error.
/// 2. Define a `struct` that holds the three counts.
/// 3. Keep `run` short: it decides, and helper functions do the work. Write
///    at least three functions besides `run`.
/// 4. Write at least two tests of your own in `mod my_tests`.
fn run(command: &str, text: &str) -> Result<String, String> {
    // `?` ends `run` with the error when the command word is unknown.
    let command = parse_command(command)?;
    match command {
        Command::Count => {
            let counts = count(text);
            Ok(format!(
                "lines: {}\nwords: {}\ncharacters: {}",
                counts.lines, counts.words, counts.characters
            ))
        }
        Command::Longest => match longest_word(text) {
            Some(word) => Ok(word.to_string()),
            None => Err(String::from("no words")),
        },
        Command::Average => match average_word_length(text) {
            Some(average) => Ok(format!("average word length: {average:.1}")),
            None => Err(String::from("no words")),
        },
        Command::Shout => Ok(text.to_uppercase()),
    }
}

/// The four things the tool can do.
#[derive(Debug, PartialEq)]
enum Command {
    Count,
    Longest,
    Average,
    Shout,
}

/// Turn the command word into a `Command`, or into the error message.
fn parse_command(word: &str) -> Result<Command, String> {
    match word {
        "count" => Ok(Command::Count),
        "longest" => Ok(Command::Longest),
        "average" => Ok(Command::Average),
        "shout" => Ok(Command::Shout),
        other => Err(format!("unknown command: {other}")),
    }
}

/// What the `count` command reports.
#[derive(Debug, PartialEq)]
struct Counts {
    lines: usize,
    words: usize,
    characters: usize,
}

fn count(text: &str) -> Counts {
    Counts {
        lines: text.lines().count(),
        words: text.split_whitespace().count(),
        characters: text.chars().count(),
    }
}

/// The longest word, measured in characters; the first one among equals.
/// The word is a piece of `text`, so it can be returned as a `&str` and
/// nothing is copied. Returning `Option<String>` is also fine at this stage.
fn longest_word(text: &str) -> Option<&str> {
    let mut longest: Option<&str> = None;
    let mut longest_length = 0;
    for word in text.split_whitespace() {
        let length = word.chars().count();
        // `>` and not `>=`: a later word of the same length does not win.
        if length > longest_length {
            longest = Some(word);
            longest_length = length;
        }
    }
    longest
}

/// The average number of characters per word, or `None` without words.
fn average_word_length(text: &str) -> Option<f64> {
    let mut words = 0;
    let mut characters = 0;
    for word in text.split_whitespace() {
        words += 1;
        characters += word.chars().count();
    }
    if words == 0 {
        // Dividing by zero words has no meaning, so there is no answer.
        None
    } else {
        // `as f64` converts the counts; they are far too small to lose digits.
        Some(characters as f64 / words as f64)
    }
}

/// Your own tests: at least two. Test your helper functions directly; that
/// is easier than testing everything through `run`.
#[cfg(test)]
mod my_tests {
    use super::*;

    // Write your tests below this line.

    #[test]
    fn command_words_are_recognised() {
        assert_eq!(parse_command("count"), Ok(Command::Count));
        assert_eq!(parse_command("shout"), Ok(Command::Shout));
        assert_eq!(
            parse_command("sing"),
            Err(String::from("unknown command: sing"))
        );
    }

    #[test]
    fn counts_of_a_text_without_a_final_line_end() {
        assert_eq!(
            count("one\ntwo words"),
            Counts {
                lines: 2,
                words: 3,
                characters: 13
            }
        );
    }

    #[test]
    fn longest_word_ignores_extra_spaces() {
        assert_eq!(longest_word("  hi   there  "), Some("there"));
        assert_eq!(longest_word(""), None);
    }

    #[test]
    fn average_of_one_word_is_its_length() {
        assert_eq!(average_word_length("four"), Some(4.0));
        assert_eq!(average_word_length(" \n"), None);
    }
}

// ---- supplied tests: keep everything below this line as it is ----

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "the quick brown fox\njumps over\n";

    fn ok(text: &str) -> Result<String, String> {
        Ok(String::from(text))
    }

    fn err(text: &str) -> Result<String, String> {
        Err(String::from(text))
    }

    #[test]
    fn a_count_reports_lines_words_and_characters() {
        assert_eq!(
            run("count", SAMPLE),
            ok("lines: 2\nwords: 6\ncharacters: 31")
        );
        assert_eq!(
            run("count", "solo"),
            ok("lines: 1\nwords: 1\ncharacters: 4")
        );
    }

    #[test]
    fn a_count_of_nothing_is_zero() {
        assert_eq!(run("count", ""), ok("lines: 0\nwords: 0\ncharacters: 0"));
        assert_eq!(
            run("count", "  \n"),
            ok("lines: 1\nwords: 0\ncharacters: 3"),
            "a line of spaces is a line with no words; its two spaces and its line end \
             are three characters"
        );
    }

    #[test]
    fn a_count_counts_characters_not_bytes() {
        assert_eq!(
            run("count", "née Zoë\n"),
            ok("lines: 1\nwords: 2\ncharacters: 8"),
            "`née Zoë` and its line end are 8 characters but 10 bytes: count with \
             chars().count(), not with len()"
        );
    }

    #[test]
    fn b_longest_takes_the_first_of_equal_words() {
        assert_eq!(
            run("longest", SAMPLE),
            ok("quick"),
            "quick, brown and jumps all have five characters: the first one wins"
        );
        assert_eq!(run("longest", "a bb ccc bb a"), ok("ccc"));
    }

    #[test]
    fn b_longest_counts_characters_not_bytes() {
        assert_eq!(
            run("longest", "ééé abcd"),
            ok("abcd"),
            "`ééé` has 3 characters but 6 bytes, and `abcd` has 4 characters: measure \
             a word with chars().count(), not with len()"
        );
    }

    #[test]
    fn c_average_has_one_decimal() {
        assert_eq!(run("average", "aa bbbb"), ok("average word length: 3.0"));
        assert_eq!(run("average", "a bb"), ok("average word length: 1.5"));
        assert_eq!(
            run("average", "one two three"),
            ok("average word length: 3.7"),
            "11 characters in 3 words: 3.666... is printed as 3.7"
        );
    }

    #[test]
    fn d_shout() {
        assert_eq!(run("shout", "Hello, world"), ok("HELLO, WORLD"));
        assert_eq!(run("shout", ""), ok(""));
    }

    #[test]
    fn e_text_without_words_is_an_error_for_longest_and_average() {
        assert_eq!(run("longest", ""), err("no words"));
        assert_eq!(run("longest", "  \n \n"), err("no words"));
        assert_eq!(run("average", ""), err("no words"));
        assert_eq!(run("average", "   "), err("no words"));
    }

    #[test]
    fn f_unknown_commands_are_named_in_the_error() {
        assert_eq!(run("dance", "some text"), err("unknown command: dance"));
        assert_eq!(run("", "some text"), err("unknown command: "));
        assert_eq!(
            run("Count", "some text"),
            err("unknown command: Count"),
            "commands are written in small letters; `Count` is not `count`"
        );
    }

    #[test]
    fn g_the_design_rules_are_followed() {
        // This test reads the text of this file, up to the supplied tests.
        let source = include_str!("l10_capstone.rs");
        let end = source
            .find("// ---- supplied tests")
            .expect("keep the line that starts with `// ---- supplied tests`");
        let yours = &source[..end];
        // Counts the lines that start with `word`, with or without `pub`.
        let starts = |word: &str| {
            yours
                .lines()
                .map(|line| line.trim_start())
                .map(|line| line.strip_prefix("pub ").unwrap_or(line))
                .filter(|line| line.starts_with(word))
                .count()
        };
        assert!(
            starts("enum ") >= 1,
            "design rule 1: define an `enum` for the commands"
        );
        assert!(
            starts("struct ") >= 1,
            "design rule 2: define a `struct` for the three counts"
        );
        let functions = starts("fn ");
        let tests = yours
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count();
        assert!(
            functions.saturating_sub(tests) >= 4,
            "design rule 3: besides `run`, write at least three functions that are not \
             tests; this file has {} so far",
            functions.saturating_sub(tests).saturating_sub(1)
        );
        assert!(
            tests >= 2,
            "design rule 4: write at least two tests of your own in `mod my_tests`; \
             this file has {tests} so far"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Two spaces between the words, and an empty line in the middle.
        // How many lines and how many words?
        let text = "a  b\n\nc";
        let p1: (usize, usize) = (3, 3); // the empty line is a line; extra spaces make no words
        assert_eq!(
            p1,
            (text.lines().count(), text.split_whitespace().count()),
            "P1: (lines, words) of \"a  b\\n\\nc\" (left: your prediction, right: actual)"
        );

        // P2. How many lines do these three texts have? Answer with three
        // numbers in square brackets, such as `[1, 2, 3]`.
        let p2: [usize; 3] = [0, 1, 2]; // a final line end does not start a new line
        assert_eq!(
            p2,
            [
                "".lines().count(),
                "one\n".lines().count(),
                "one\ntwo".lines().count(),
            ],
            "P2: lines of \"\", \"one\\n\" and \"one\\ntwo\" \
             (left: your prediction, right: actual)"
        );

        // P3. Bytes and characters of a word with an accent.
        let word = "née";
        let p3: (usize, usize) = (4, 3); // `é` takes two bytes
        assert_eq!(
            p3,
            (word.len(), word.chars().count()),
            "P3: (len(), chars().count()) of \"née\" (left: your prediction, right: actual)"
        );
    }
}
