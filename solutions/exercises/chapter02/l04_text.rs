//! LESSON 04 — Text: &str, String and formatting
//! Run: cargo test --test chapter02 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A function has the parameter `name: &str`. Can you call it with a
//!    literal such as `"Ada"`? With a `String`?
//! 2. What is the difference between `{}` and `{:?}` when you print a text?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain when a function should take `&str` and when it
//! must return `String`, and what `format!` gives you that `println!` does
//! not.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write
//! `let name: String = "Ada";`. Read the error and its help line. Repair it
//! in two different ways, and leave the file compiling.

/// A. Return a greeting for `name`: for "Ada" return "Hello, Ada!".
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

/// B. Return a price tag: the item, a colon, a space, and the price with
/// exactly two digits after the decimal point. For ("tea", 2.5) return
/// "tea: 2.50".
fn price_tag(item: &str, price: f64) -> String {
    format!("{item}: {price:.2}")
}

/// C. Return `word` written `times` times with one space between the copies:
/// ("go", 3) gives "go go go". For 0 times return an empty String. There is
/// no space at the start or at the end.
/// Build the result in a `let mut` String with `push_str`.
fn repeat_word(word: &str, times: usize) -> String {
    let mut result = String::new();
    for i in 0..times {
        // The space goes BEFORE every copy but the first, so none is left
        // over at the end.
        if i > 0 {
            result.push(' ');
        }
        result.push_str(word);
    }
    result
}

/// D. Return one row of a score table: the name in a column 8 characters
/// wide, aligned to the left; then `|`; then the score in a column 4
/// characters wide, aligned to the right. For ("Ada", 92) return
/// "Ada     |  92".
fn table_row(name: &str, score: u32) -> String {
    format!("{name:<8}|{score:>4}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_greet() {
        assert_eq!(greet("Ada"), "Hello, Ada!");
        assert_eq!(greet("world"), "Hello, world!");
        // A String can be lent to a `&str` parameter with `&`.
        let owned = String::from("Grace");
        assert_eq!(greet(&owned), "Hello, Grace!");
        assert_eq!(greet(""), "Hello, !");
    }

    #[test]
    fn b_price_tag_has_two_decimals() {
        assert_eq!(price_tag("tea", 2.5), "tea: 2.50");
        assert_eq!(price_tag("bread", 3.0), "bread: 3.00");
        assert_eq!(
            price_tag("jam", 4.126),
            "jam: 4.13",
            "more than two decimals are rounded to two: 4.126 becomes 4.13"
        );
    }

    #[test]
    fn c_repeat_word() {
        assert_eq!(repeat_word("go", 3), "go go go");
        assert_eq!(repeat_word("once", 1), "once");
        assert_eq!(
            repeat_word("never", 0),
            "",
            "zero copies give an empty String"
        );
        assert!(
            !repeat_word("ho", 4).ends_with(' '),
            "there must be no space after the last copy"
        );
        assert_eq!(repeat_word("ho", 4), "ho ho ho ho");
    }

    #[test]
    fn d_table_row_has_two_columns() {
        assert_eq!(table_row("Ada", 92), "Ada     |  92");
        assert_eq!(table_row("Grace", 100), "Grace   | 100");
        assert_eq!(table_row("Tim", 7), "Tim     |   7");
        assert_eq!(
            table_row("Ada", 92).len(),
            13,
            "every row is 8 + 1 + 4 = 13 characters long"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Three formatted texts. Write each one exactly, spaces included.
        // `[&str; 3]` is an array of three texts: answer in square brackets,
        // in the form `["...", "...", "..."]`.
        let p1: [&str; 3] = ["[   42]", "[42   ]", "9.9"];
        assert_eq!(
            p1,
            [
                format!("[{:>5}]", 42),
                format!("[{:<5}]", 42),
                format!("{:.1}", 9.87654),
            ],
            "P1: [{{:>5}}], [{{:<5}}] and {{:.1}} (left: your prediction, right: actual)"
        );

        // P2. `{}` and `{:?}` for the same text, each between `<` and `>`.
        // How long is each result?
        let word = "hi";
        let plain = format!("<{}>", word);
        let debug = format!("<{:?}>", word);
        let p2: (usize, usize) = (4, 6); // {:?} adds the two quotation marks
        assert_eq!(
            p2,
            (plain.len(), debug.len()),
            "P2: lengths of <{{}}> and <{{:?}}> for the text hi \
             (left: your prediction, right: actual)"
        );

        // P3. A String that grows. How many bytes does it hold at the end?
        let mut s = String::from("ab");
        s.push_str("cd");
        s.push('!');
        let p3: usize = 5; // "abcd!"
        assert_eq!(
            p3,
            s.len(),
            "P3: s.len() after push_str and push (left: your prediction, right: actual)"
        );
    }
}
