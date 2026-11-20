//@ expect: run
//@ why: a word points into the source text, so it must stay usable while the lexer is used again, and after the lexer is gone.
use m1_types::l02_bounds_hrtb::Lexer;

fn main() {
    let source = String::from("let total = 42");

    let mut lexer = Lexer::new(&source);
    let first = lexer.next_word();
    let second = lexer.next_word();
    assert_eq!((first, second), (Some("let"), Some("total")));

    let mut words = Vec::new();
    while let Some(word) = lexer.next_word() {
        words.push(word);
    }
    assert_eq!(words, ["=", "42"]);

    let kept = {
        let mut short_lived_lexer = Lexer::new(&source);
        short_lived_lexer.next_word()
    };
    assert_eq!(kept, Some("let"));
}
