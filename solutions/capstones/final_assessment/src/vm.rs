//! Running the automaton: all live states at once, one character at a time.
//!
//! A backtracking matcher follows one path through the automaton and comes
//! back when it fails; on some patterns the number of paths doubles with
//! every character. This matcher never comes back. It keeps the *set* of
//! states the automaton can be in after the characters read so far, and
//! moves the whole set forward by one character. A set holds each state at
//! most once, so one step costs at most the size of the program, and a
//! search costs at most (program size) x (haystack length). That bound holds
//! for every pattern and every haystack.
//!
//! Two details turn "does it match?" into "which match would a backtracking
//! matcher report?" (SPEC 4.2):
//!
//! - The set is ordered. A state reached by a preferred path comes before a
//!   state reached by a less preferred one, and a state that is already in
//!   the set is not added again: the first arrival is the preferred one, and
//!   two arrivals at the same state at the same position have the same
//!   future.
//! - Each state remembers where its path started. New paths start at later
//!   positions only while nothing has matched, and they go to the back of
//!   the set, so an earlier start always wins.

use crate::compile::{Inst, Pc, Program};

/// The memory one search needs. It is sized by the program, not by the
/// haystack, so it can be used again for any number of searches.
#[derive(Debug)]
pub(crate) struct Scratch {
    current: States,
    next: States,
    /// Work list for [`follow`].
    stack: Vec<Pc>,
}

impl Scratch {
    pub(crate) fn new(program: &Program) -> Scratch {
        let size = program.insts.len();
        Scratch {
            current: States::new(size),
            next: States::new(size),
            // Enough for any walk of `follow`, so that no search ever
            // allocates: see the bound in its documentation.
            stack: Vec::with_capacity(program.splits + 1),
        }
    }
}

/// An ordered set of program counters with constant-time insert, lookup and
/// clear (a "sparse set"), plus the start offset of each member's path.
#[derive(Debug)]
struct States {
    /// The members in the order they were added: most preferred first.
    dense: Vec<Pc>,
    /// If `pc` is a member, `sparse[pc]` is its index in `dense`. For other
    /// values it holds anything, which `contains` detects.
    sparse: Box<[u32]>,
    /// `starts[pc]`: where the path that reached `pc` began.
    starts: Box<[usize]>,
}

impl States {
    fn new(size: usize) -> States {
        States {
            dense: Vec::with_capacity(size),
            sparse: vec![0; size].into_boxed_slice(),
            starts: vec![0; size].into_boxed_slice(),
        }
    }

    fn clear(&mut self) {
        self.dense.clear();
    }

    fn contains(&self, pc: Pc) -> bool {
        self.dense.get(self.sparse[pc as usize] as usize) == Some(&pc)
    }

    /// Adds `pc` unless it is already a member. Returns whether it was added.
    fn insert(&mut self, pc: Pc, start: usize) -> bool {
        if self.contains(pc) {
            return false;
        }
        // `dense` never grows beyond its capacity: each of the `size`
        // program counters is pushed at most once between two `clear`s.
        // Its length is therefore a valid `Pc` (the compiler checks that
        // the program size fits), and the cast cannot truncate.
        self.sparse[pc as usize] = self.dense.len() as u32;
        self.starts[pc as usize] = start;
        self.dense.push(pc);
        true
    }
}

/// The characters on both sides of a position (`None` at either end of the
/// haystack). Assertions look at nothing else.
#[derive(Clone, Copy)]
struct Around {
    prev: Option<char>,
    next: Option<char>,
}

/// Adds `pc` to `states`, and every state that can be reached from it
/// without consuming a character, in order of preference.
///
/// The walk is depth first with an explicit stack: a chain of ten thousand
/// alternatives must not become ten thousand nested calls. Pushing the
/// second target of a `Split` before the first makes the first one come off
/// the stack, and be followed to its end, before the second.
///
/// The stack never holds more than `program.splits + 1` entries. Every
/// entry below the top one is the second target of a `Split` that this walk
/// has just added to `states`, and a state is added only once. `Scratch`
/// reserves that much, so the pushes below never allocate.
fn follow(
    program: &Program,
    states: &mut States,
    stack: &mut Vec<Pc>,
    pc: Pc,
    start: usize,
    around: Around,
) {
    stack.push(pc);
    while let Some(pc) = stack.pop() {
        if !states.insert(pc, start) {
            continue;
        }
        match program.insts[pc as usize] {
            Inst::Jump(target) => stack.push(target),
            Inst::Split(first, second) => {
                stack.push(second);
                stack.push(first);
            }
            Inst::Assert(look) => {
                if look.holds(around.prev, around.next) {
                    stack.push(pc + 1);
                }
            }
            // These wait in the set for the next character (or, for
            // `Match`, for their turn in `search`).
            Inst::Char(_) | Inst::Any | Inst::Class(_) | Inst::Match => {}
        }
    }
}

/// What a search should report.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Want {
    /// Stop as soon as any match is certain. The offsets are of no use.
    Existence,
    /// The match of SPEC 4.3.
    Span,
}

/// Searches `haystack` for a match that starts at `from` or later.
///
/// `from` must be on a character boundary. Assertions see the whole
/// haystack, also the part before `from`.
pub(crate) fn search(
    program: &Program,
    scratch: &mut Scratch,
    haystack: &str,
    from: usize,
    want: Want,
) -> Option<(usize, usize)> {
    let Scratch {
        current,
        next,
        stack,
    } = scratch;
    current.clear();
    next.clear();

    let mut chars = haystack[from..].chars();
    // `at` is the position between `prev` and `cur`.
    let mut at = from;
    let mut prev = haystack[..from].chars().next_back();
    let mut cur = chars.next();
    let mut found = None;

    loop {
        // A new attempt may begin here if no earlier attempt has matched.
        // It goes behind everything already in the set.
        let may_start = found.is_none() && (at == 0 || !program.anchored_start);
        if may_start {
            follow(program, current, stack, 0, at, Around { prev, next: cur });
        }
        if current.dense.is_empty() && (found.is_some() || program.anchored_start) {
            // Nothing is alive and nothing new may begin.
            break;
        }

        let after = chars.next();
        let around_next = Around {
            prev: cur,
            next: after,
        };
        for i in 0..current.dense.len() {
            let pc = current.dense[i];
            let consumes = match program.insts[pc as usize] {
                Inst::Match => {
                    found = Some((current.starts[pc as usize], at));
                    if want == Want::Existence {
                        return found;
                    }
                    // Everything behind this state in the set is less
                    // preferred than a match that already exists. Drop it.
                    break;
                }
                Inst::Char(c) => cur == Some(c),
                Inst::Any => cur.is_some_and(|c| c != '\n'),
                Inst::Class(index) => {
                    cur.is_some_and(|c| program.classes[index as usize].contains(c))
                }
                // Left in the set by `follow` only to mark them as visited.
                Inst::Split(..) | Inst::Jump(_) | Inst::Assert(_) => false,
            };
            if consumes {
                follow(
                    program,
                    next,
                    stack,
                    pc + 1,
                    current.starts[pc as usize],
                    around_next,
                );
            }
        }

        let Some(c) = cur else { break };
        at += c.len_utf8();
        prev = cur;
        cur = after;
        std::mem::swap(current, next);
        next.clear();
    }
    found
}
