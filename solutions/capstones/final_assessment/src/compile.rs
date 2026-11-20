//! Syntax tree to automaton.
//!
//! The automaton is a nondeterministic finite automaton written down as a
//! small program: each state is one [`Inst`], and the state after a
//! consuming instruction is simply the next instruction. `Split` and `Jump`
//! are the transitions that consume nothing.
//!
//! The order of the two targets of a `Split` is the whole story of
//! preference: the matcher always follows the first target before the
//! second. The compiler puts "the alternative the pattern prefers" first:
//! the earlier branch of `|`, one more round of a loop before leaving it.

use crate::ast::{Ast, Look};
use crate::class::{CharSet, RangeSet};

/// Index of an instruction.
pub(crate) type Pc = u32;

/// One state of the automaton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inst {
    /// Consume exactly this character.
    Char(char),
    /// Consume any character except line feed.
    Any,
    /// Consume a character of the set `classes[n]`.
    Class(u32),
    /// Go on at the first target; if that leads nowhere, at the second.
    Split(Pc, Pc),
    /// Go on at the target.
    Jump(Pc),
    /// Go on at the next instruction if the assertion holds here.
    Assert(Look),
    /// The pattern has matched.
    Match,
}

/// A compiled pattern. Immutable, so any number of threads can read it.
#[derive(Debug)]
pub(crate) struct Program {
    pub(crate) insts: Box<[Inst]>,
    pub(crate) classes: Box<[CharSet]>,
    /// Every match starts at offset 0 (see [`Ast::is_anchored_at_start`]).
    pub(crate) anchored_start: bool,
    /// How many of the instructions are `Split`s. The matcher sizes its
    /// work list with this number, once, so that it never has to grow.
    pub(crate) splits: usize,
}

/// Compiles a tree whose cost has been checked against the limit.
///
/// The program has at most `2 * cost + 1` instructions: every unit of cost
/// is one consuming instruction, assertion or `Split`, and each `Split`
/// brings at most one more `Split` or `Jump` with it.
pub(crate) fn compile(ast: &Ast, case_insensitive: bool) -> Program {
    let mut compiler = Compiler {
        insts: Vec::new(),
        classes: Vec::new(),
        case_insensitive,
    };
    compiler.emit(ast);
    compiler.insts.push(Inst::Match);
    let splits = compiler
        .insts
        .iter()
        .filter(|inst| matches!(inst, Inst::Split(..)))
        .count();
    Program {
        insts: compiler.insts.into_boxed_slice(),
        classes: compiler.classes.into_boxed_slice(),
        anchored_start: ast.is_anchored_at_start(),
        splits,
    }
}

struct Compiler {
    insts: Vec<Inst>,
    classes: Vec<CharSet>,
    case_insensitive: bool,
}

impl Compiler {
    /// The index that the next pushed instruction will get.
    fn here(&self) -> Pc {
        // The cost limit keeps a program far below `u32::MAX` instructions.
        Pc::try_from(self.insts.len()).expect("the cost limit bounds the program size")
    }

    /// Pushes an instruction whose target is not known yet and returns its
    /// index for [`patch`](Self::patch).
    fn placeholder(&mut self) -> usize {
        self.insts.push(Inst::Jump(Pc::MAX));
        self.insts.len() - 1
    }

    fn patch(&mut self, at: usize, inst: Inst) {
        self.insts[at] = inst;
    }

    fn class(&mut self, set: &RangeSet, negated: bool) {
        // Fold first, then complement: with `-i`, `[^a]` must refuse `A` too.
        let folded;
        let mut set = set;
        if self.case_insensitive {
            folded = set.fold_ascii_case();
            set = &folded;
        }
        let complement;
        if negated {
            complement = set.complement();
            set = &complement;
        }
        let index = u32::try_from(self.classes.len()).expect("fewer classes than instructions");
        self.classes.push(CharSet::new(set));
        self.insts.push(Inst::Class(index));
    }

    /// Appends the code for `ast`. When that code matches, control arrives
    /// at the instruction after it.
    fn emit(&mut self, ast: &Ast) {
        match ast {
            Ast::Empty => {}
            Ast::Literal(c) if self.case_insensitive && c.is_ascii_alphabetic() => {
                self.class(&RangeSet::from_chars(&[(*c, *c)]), false);
            }
            Ast::Literal(c) => self.insts.push(Inst::Char(*c)),
            Ast::Dot => self.insts.push(Inst::Any),
            Ast::Class { set, negated } => self.class(set, *negated),
            Ast::Look(look) => self.insts.push(Inst::Assert(*look)),
            Ast::Concat(parts) => parts.iter().for_each(|part| self.emit(part)),
            Ast::Alternate(branches) => self.alternate(branches),
            Ast::Repeat { sub, min, max } => self.repeat(sub, *min, *max),
        }
    }

    /// ```text
    ///       split L1, L2
    ///   L1: <branch 1>
    ///       jump END
    ///   L2: split L3, L4
    ///   L3: <branch 2>
    ///       jump END
    ///   L4: <last branch>
    ///  END:
    /// ```
    fn alternate(&mut self, branches: &[Ast]) {
        let (last, earlier) = branches.split_last().expect("an alternation has branches");
        let mut jumps_to_end = Vec::with_capacity(earlier.len());
        for branch in earlier {
            let split = self.placeholder();
            let first = self.here();
            self.emit(branch);
            jumps_to_end.push(self.placeholder());
            let second = self.here();
            self.patch(split, Inst::Split(first, second));
        }
        self.emit(last);
        let end = self.here();
        for jump in jumps_to_end {
            self.patch(jump, Inst::Jump(end));
        }
    }

    fn repeat(&mut self, sub: &Ast, min: u32, max: Option<u32>) {
        if matches!(sub, Ast::Empty) {
            // Any number of copies of the empty string is the empty string.
            // (The tree keeps such a node because the SPEC gives it a cost;
            // there is nothing to run.) Returning here also keeps the work
            // of the compiler proportional to the size of the program: the
            // loops below would run up to a thousand times to emit nothing.
            return;
        }
        match max {
            // `e{n,}` is n - 1 copies and then `e+`:
            //
            //   BODY: <e>
            //         split BODY, END
            //    END:
            //
            // After a round the loop prefers another round. `e*` is
            // `(e+)?`: one more `split` in front decides between entering
            // the loop and passing it by.
            //
            // The parser has refused every loop whose body can match the
            // empty string (SPEC 3.7), so a round always consumes a
            // character. The automaton therefore has no cycle that consumes
            // nothing, and the matcher never has to ask what such a cycle
            // would mean.
            None => {
                let skip = (min == 0).then(|| self.placeholder());
                for _ in 1..min {
                    self.emit(sub);
                }
                let body = self.here();
                self.emit(sub);
                let again = self.placeholder();
                let end = self.here();
                self.patch(again, Inst::Split(body, end));
                if let Some(skip) = skip {
                    self.patch(skip, Inst::Split(body, end));
                }
            }
            // `e{n,m}`: n copies, then m - n optional copies, each inside
            // the one before it, as in `ee(e(e)?)?`. Leaving at any point
            // skips all the copies that remain.
            Some(max) => {
                for _ in 0..min {
                    self.emit(sub);
                }
                let mut splits = Vec::with_capacity((max - min) as usize);
                for _ in min..max {
                    splits.push(self.placeholder());
                    self.emit(sub);
                }
                let end = self.here();
                for split in splits {
                    let body = Pc::try_from(split + 1).expect("checked by `here`");
                    self.patch(split, Inst::Split(body, end));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn insts(pattern: &str) -> Vec<Inst> {
        compile(&parse(pattern).unwrap(), false).insts.into_vec()
    }

    #[test]
    fn alternation_prefers_the_earlier_branch() {
        use Inst::*;
        assert_eq!(
            insts("a|b|c"),
            [
                Split(1, 3),
                Char('a'),
                Jump(7),
                Split(4, 6),
                Char('b'),
                Jump(7),
                Char('c'),
                Match
            ]
        );
    }

    #[test]
    fn a_loop_prefers_one_more_round() {
        use Inst::*;
        assert_eq!(insts("a*"), [Split(1, 3), Char('a'), Split(1, 3), Match]);
        assert_eq!(insts("a+"), [Char('a'), Split(0, 2), Match]);
        assert_eq!(
            insts("a{3,}"),
            [Char('a'), Char('a'), Char('a'), Split(2, 4), Match]
        );
        assert_eq!(
            insts("a{1,3}"),
            [
                Char('a'),
                Split(2, 5),
                Char('a'),
                Split(4, 5),
                Char('a'),
                Match
            ]
        );
    }

    #[test]
    fn an_instruction_is_twelve_bytes() {
        // DESIGN.md gives the size of the largest program from this number.
        assert_eq!(std::mem::size_of::<Inst>(), 12);
    }

    #[test]
    fn the_program_stays_within_twice_the_cost() {
        for pattern in [
            "",
            "a",
            "a|b|c",
            "(a|b)*c{2,5}",
            "((a|b)+|[x-z]?){3}$",
            "(a{2,}|b*c)+",
            r"\b(foo|bar|baz){0,7}\B",
        ] {
            let ast = parse(pattern).unwrap();
            let program = compile(&ast, true);
            assert!(
                program.insts.len() as u64 <= 2 * ast.cost() + 1,
                "{pattern}: {} instructions for cost {}",
                program.insts.len(),
                ast.cost()
            );
        }
    }
}
