//! The syntax tree that the parser builds and the compiler reads.

use crate::class::{RangeSet, is_word};

/// A parsed pattern.
///
/// Concatenations and alternations hold all their parts in one `Vec`
/// instead of nesting pairs. A pattern such as `a|b|c|...` with thousands of
/// branches is then a flat node, and every function that walks the tree
/// recurses only as deep as groups are nested, which the parser limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Ast {
    /// Matches the empty string: an empty pattern, group or branch.
    Empty,
    /// One character.
    Literal(char),
    /// `.`: any character except line feed.
    Dot,
    /// `[...]`, `\d` and friends. `set` is the set as written; `negated`
    /// asks for its complement. They stay apart until compile time because
    /// case folding must happen before the complement (SPEC 3.9).
    Class { set: RangeSet, negated: bool },
    /// `^`, `$`, `\b`, `\B`.
    Look(Look),
    /// The parts in order.
    Concat(Vec<Ast>),
    /// The branches in order of preference.
    Alternate(Vec<Ast>),
    /// `sub` repeated at least `min` times and at most `max` times
    /// (`None`: no upper limit).
    Repeat {
        sub: Box<Ast>,
        min: u32,
        max: Option<u32>,
    },
}

/// A test on a position. It consumes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Look {
    /// `^`
    Start,
    /// `$`
    End,
    /// `\b`
    WordBoundary,
    /// `\B`
    NotWordBoundary,
}

impl Look {
    /// Does the assertion hold between `prev` and `next`? `None` stands for
    /// the start or the end of the haystack.
    #[inline]
    pub(crate) fn holds(self, prev: Option<char>, next: Option<char>) -> bool {
        let at_boundary = || prev.is_some_and(is_word) != next.is_some_and(is_word);
        match self {
            Look::Start => prev.is_none(),
            Look::End => next.is_none(),
            Look::WordBoundary => at_boundary(),
            Look::NotWordBoundary => !at_boundary(),
        }
    }
}

impl Ast {
    /// The parts of a concatenation, as one tree.
    ///
    /// Parts that are [`Ast::Empty`] are left out; see [`Ast::repeat`] for
    /// why that matters.
    pub(crate) fn concat(mut parts: Vec<Ast>) -> Ast {
        parts.retain(|part| !matches!(part, Ast::Empty));
        match parts.len() {
            0 => Ast::Empty,
            1 => parts.pop().expect("one part"),
            _ => Ast::Concat(parts),
        }
    }

    /// `sub` with a quantifier.
    ///
    /// `e{0}`, and `e{n}` for an `e` that is `Empty`, match exactly the
    /// empty string and cost nothing, so they become `Empty`. Together with
    /// [`Ast::concat`] this gives the trees of the parser a property that
    /// the compiler's running time rests on: **a tree costs 0 only if it is
    /// `Empty`**. Without it, `((){1000}){1000}...` nested twenty deep
    /// would cost 0, pass the cost limit, and keep the compiler busy for
    /// 1000^20 steps that emit nothing.
    pub(crate) fn repeat(sub: Ast, min: u32, max: Option<u32>) -> Ast {
        match max {
            Some(0) => Ast::Empty,
            Some(max) if max == min && matches!(sub, Ast::Empty) => Ast::Empty,
            _ => Ast::Repeat {
                sub: Box::new(sub),
                min,
                max,
            },
        }
    }

    /// The cost of SPEC 6.1, saturating at `u64::MAX`.
    ///
    /// A pattern is 100 kB at most in practice but its cost is not bounded
    /// by its length: seven nested `{1000}` already exceed `u64`. Every step
    /// therefore saturates, and the caller compares the result with the
    /// limit before anything of that size is built.
    pub(crate) fn cost(&self) -> u64 {
        match self {
            Ast::Empty => 0,
            Ast::Literal(_) | Ast::Dot | Ast::Class { .. } | Ast::Look(_) => 1,
            Ast::Concat(parts) => parts
                .iter()
                .fold(0, |sum: u64, part| sum.saturating_add(part.cost())),
            Ast::Alternate(branches) => branches
                .iter()
                .fold(branches.len() as u64 - 1, |sum, branch| {
                    sum.saturating_add(branch.cost())
                }),
            Ast::Repeat { sub, min, max } => {
                let one = sub.cost();
                match *max {
                    // `e{n,m}`: m copies and m - n choices.
                    Some(max) => one
                        .saturating_mul(u64::from(max))
                        .saturating_add(u64::from(max - min)),
                    // `e{n,}`: n copies (one for `e*`), the last one in a loop.
                    None => one.saturating_mul(u64::from(*min).max(1)).saturating_add(1),
                }
            }
        }
    }

    /// True if every match of this expression must begin at offset 0,
    /// because every way through it passes a `^`.
    ///
    /// The answer may be `false` for an expression that is in fact anchored
    /// (the check does not try to be clever); it is never `true` wrongly.
    pub(crate) fn is_anchored_at_start(&self) -> bool {
        match self {
            Ast::Look(Look::Start) => true,
            // Whatever comes before the anchored part can only match the
            // empty string at offset 0, so the whole match starts there.
            Ast::Concat(parts) => parts.iter().any(Ast::is_anchored_at_start),
            Ast::Alternate(branches) => branches.iter().all(Ast::is_anchored_at_start),
            Ast::Repeat { sub, min, .. } => *min >= 1 && sub.is_anchored_at_start(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    #[test]
    fn the_cost_examples_of_spec_6_1() {
        let examples: &[(&str, u64)] = &[
            ("abc", 3),
            ("a|b|c", 5),
            ("(a|b)*c", 5),
            (r"\d{2,4}", 6),
            ("[a-z0-9._]+@", 3),
            (r"^\s*(#.*)?$", 8),
            ("a{1000}", 1000),
            ("(a{100}){1000}", 100_000),
            ("(a{100}b){1000}", 101_000),
            ("((a{1000}){1000}){1000}", 1_000_000_000),
            ("(){1000}", 0),
            // One line for every row of the table.
            ("", 0),
            ("()", 0),
            (r"\.", 1),
            (".", 1),
            (r"[a-z\d]", 1),
            (r"\W", 1),
            (r"^$\b\B", 4),
            ("(ab)", 2),
            ("ab|c|", 5),
            ("(ab)?", 3),
            ("(ab)*", 3),
            ("(ab)+", 3),
            ("(ab){7}", 14),
            ("(ab){7,}", 15),
            ("(ab){0,}", 3),
            ("(ab){3,7}", 18),
            ("(ab){0,1}", 3),
        ];
        for &(pattern, cost) in examples {
            assert_eq!(parse(pattern).unwrap().cost(), cost, "cost of {pattern:?}");
        }
    }

    #[test]
    fn a_cost_beyond_u64_saturates() {
        let mut pattern = String::from("a");
        for _ in 0..7 {
            pattern = format!("({pattern}){{1000}}");
        }
        assert_eq!(parse(&pattern).unwrap().cost(), u64::MAX);
    }
}
