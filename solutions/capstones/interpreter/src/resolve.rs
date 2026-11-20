//! Names to addresses, and the checks of SPEC 6.4.
//!
//! This pass runs once over a parsed program, before any of it is evaluated.
//! It does two jobs with one walk:
//!
//! - It reports the errors that need no evaluation: unknown names, duplicate
//!   declarations, assignment to something that is not a `var`, and
//!   `return`, `break` and `continue` in the wrong place.
//! - It replaces every use of a name by an [`Addr`]: how many environments
//!   to go outwards, and which slot to take there. The evaluator then never
//!   hashes or compares a name.
//!
//! The scope of a declaration is its whole block (SPEC 6.2). So the pass
//! declares all names of a block before it looks at the first statement.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ast::{Addr, Block, Expr, ExprKind, FnDecl, ForIter, Ident, Stmt, Target};
use crate::builtins::Builtin;
use crate::source::{Diagnostic, Span};

#[derive(Clone, Copy)]
struct Binding {
    slot: usize,
    mutable: bool,
}

/// The names of one block. A scope without names has no environment at run
/// time and is skipped when depths are counted.
#[derive(Default)]
struct Scope {
    names: HashMap<Rc<str>, Binding>,
}

impl Scope {
    /// Adds `name` and returns its slot, or `None` if the scope has it
    /// already.
    fn declare(&mut self, name: &Rc<str>, mutable: bool) -> Option<usize> {
        if self.names.contains_key(name) {
            return None;
        }
        let slot = self.names.len();
        self.names
            .insert(Rc::clone(name), Binding { slot, mutable });
        Some(slot)
    }
}

/// What a name means at one point of the program.
struct Found {
    addr: Addr,
    mutable: bool,
}

/// Keeps the top-level names between REPL entries.
pub struct Resolver {
    builtins: Scope,
    globals: Scope,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Resolver {
        let mut builtins = Scope::default();
        for builtin in Builtin::ALL {
            // Slot numbers follow the order of `Builtin::ALL`; the evaluator
            // fills its outermost environment in the same order.
            builtins.declare(&Rc::from(builtin.name()), false);
        }
        Resolver {
            builtins,
            globals: Scope::default(),
        }
    }

    /// Checks `program` and fills in its addresses and slot counts.
    ///
    /// On success the top-level names of `program` are remembered for later
    /// calls. On failure nothing is remembered, and the error is the first
    /// one in source order.
    pub fn resolve(&mut self, program: &mut Block) -> Result<(), Diagnostic> {
        let before = self.globals.names.clone();
        let mut walk = Walk {
            scopes: vec![
                std::mem::take(&mut self.builtins),
                std::mem::take(&mut self.globals),
            ],
            errors: Vec::new(),
            functions: 0,
            loops: 0,
        };
        walk.program(program);
        let Walk {
            mut scopes, errors, ..
        } = walk;
        // Take the two outermost scopes back, in the order they were pushed.
        self.globals = scopes.pop().unwrap_or_default();
        self.builtins = scopes.pop().unwrap_or_default();

        match errors.into_iter().min_by_key(|e| e.span.start) {
            None => Ok(()),
            Some(first) => {
                self.globals.names = before;
                Err(first)
            }
        }
    }
}

/// The state of one walk over one program.
struct Walk {
    /// Innermost last. `scopes[0]` holds the built-in functions, `scopes[1]`
    /// the top level.
    scopes: Vec<Scope>,
    /// Every error found; the caller keeps the first in source order.
    errors: Vec<Diagnostic>,
    /// How many function bodies enclose the current position.
    functions: usize,
    /// How many loops enclose the current position, counted from the
    /// innermost function body.
    loops: usize,
}

impl Walk {
    fn error(&mut self, ident: &Ident, message: String) {
        self.errors.push(Diagnostic::new(ident.span, message));
    }

    fn duplicate(&mut self, ident: &Ident) {
        self.error(
            ident,
            format!("`{}` is already declared in this block", ident.name),
        );
    }

    // ----- scopes ---------------------------------------------------------

    /// The top level. Unlike a block it is never popped, and a name that an
    /// earlier REPL entry declared may be declared again (SPEC 12.4).
    fn program(&mut self, program: &mut Block) {
        self.declare_top_level(program);
        self.statements(program);
        program.slots = self.scopes[1].names.len();
    }

    fn declare_top_level(&mut self, program: &mut Block) {
        let mut declared_now: HashSet<Rc<str>> = HashSet::new();
        for stmt in &mut program.stmts {
            let (name, mutable, slot) = match stmt {
                Stmt::Let {
                    name,
                    mutable,
                    slot,
                    ..
                } => (&*name, *mutable, slot),
                Stmt::Fn { name, slot, .. } => (&*name, false, slot),
                _ => continue,
            };
            if !declared_now.insert(Rc::clone(&name.name)) {
                self.duplicate(name);
                continue;
            }
            let globals = &mut self.scopes[1];
            *slot = match globals.names.get_mut(&name.name) {
                // Declared by an earlier entry: the same slot, new rules.
                Some(binding) => {
                    binding.mutable = mutable;
                    binding.slot
                }
                None => globals.declare(&name.name, mutable).unwrap_or_default(),
            };
        }
    }

    /// Resolves `block` in a new scope that starts with `first` (the
    /// parameters of a function or the variable of a `for` loop).
    fn scoped(&mut self, block: &mut Block, first: &[Ident]) {
        let mut scope = Scope::default();
        for ident in first {
            if scope.declare(&ident.name, false).is_none() {
                self.duplicate(ident);
            }
        }
        for stmt in &mut block.stmts {
            let (name, mutable, slot) = match stmt {
                Stmt::Let {
                    name,
                    mutable,
                    slot,
                    ..
                } => (&*name, *mutable, slot),
                Stmt::Fn { name, slot, .. } => (&*name, false, slot),
                _ => continue,
            };
            match scope.declare(&name.name, mutable) {
                Some(new) => *slot = new,
                None => self.duplicate(name),
            }
        }
        block.slots = scope.names.len();
        self.scopes.push(scope);
        self.statements(block);
        self.scopes.pop();
    }

    /// Finds `name`, innermost scope first.
    fn find(&self, name: &str) -> Option<Found> {
        let mut depth = 0;
        for (index, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(binding) = scope.names.get(name) {
                return Some(Found {
                    addr: Addr {
                        depth,
                        slot: binding.slot,
                    },
                    mutable: binding.mutable,
                });
            }
            // Count the environments that exist at run time: always for the
            // two outermost scopes, and for a block only if it declares
            // something. (All names of a block are declared before its
            // statements are resolved, so "is empty" is already final.)
            if index <= 1 || !scope.names.is_empty() {
                depth += 1;
            }
        }
        None
    }

    fn lookup(&mut self, name: &str, span: Span) -> Option<Found> {
        let found = self.find(name);
        if found.is_none() {
            self.errors
                .push(Diagnostic::new(span, format!("unknown name `{name}`")));
        }
        found
    }

    // ----- the walk -------------------------------------------------------

    fn statements(&mut self, block: &mut Block) {
        for stmt in &mut block.stmts {
            self.statement(stmt);
        }
        if let Some(tail) = &mut block.tail {
            self.expr(tail);
        }
    }

    fn statement(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Let { init, .. } => self.expr(init),
            Stmt::Fn { decl, .. } => self.function(decl),
            Stmt::Assign { target, value } => {
                match target {
                    Target::Name { name, addr } => {
                        if let Some(found) = self.lookup(&name.name, name.span) {
                            *addr = found.addr;
                            if !found.mutable {
                                let message = format!(
                                    "cannot assign to `{}`: it is not declared with `var`",
                                    name.name
                                );
                                self.error(name, message);
                            }
                        }
                    }
                    Target::Index { target, index, .. } => {
                        self.expr(target);
                        self.expr(index);
                    }
                }
                self.expr(value);
            }
            Stmt::Expr(expr) => self.expr(expr),
            Stmt::While { cond, body } => {
                self.expr(cond);
                self.loops += 1;
                self.scoped(body, &[]);
                self.loops -= 1;
            }
            Stmt::For { var, iter, body } => {
                // The loop variable is not in scope in the header.
                match iter {
                    ForIter::Values(values) => self.expr(values),
                    ForIter::Range(first, limit) => {
                        self.expr(first);
                        self.expr(limit);
                    }
                }
                self.loops += 1;
                self.scoped(body, std::slice::from_ref(var));
                self.loops -= 1;
            }
            Stmt::Return { value, span } => {
                if self.functions == 0 {
                    self.errors
                        .push(Diagnostic::new(*span, "`return` outside of a function"));
                }
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            Stmt::Break(span) => {
                if self.loops == 0 {
                    self.errors
                        .push(Diagnostic::new(*span, "`break` outside of a loop"));
                }
            }
            Stmt::Continue(span) => {
                if self.loops == 0 {
                    self.errors
                        .push(Diagnostic::new(*span, "`continue` outside of a loop"));
                }
            }
        }
    }

    fn function(&mut self, decl: &mut Rc<FnDecl>) {
        // The parser made this `Rc` and nothing has cloned it yet: closures,
        // the only other owners, are created by the evaluator.
        let Some(decl) = Rc::get_mut(decl) else {
            unreachable!("a function is resolved before any closure shares it");
        };
        // A loop outside the function is not a loop inside it.
        let loops = std::mem::take(&mut self.loops);
        self.functions += 1;
        let FnDecl { params, body, .. } = decl;
        self.scoped(body, params);
        self.functions -= 1;
        self.loops = loops;
    }

    fn expr(&mut self, expr: &mut Expr) {
        match &mut expr.kind {
            ExprKind::Nil | ExprKind::Bool(_) | ExprKind::Int(_) | ExprKind::Str(_) => {}
            ExprKind::Name { name, addr } => {
                if let Some(found) = self.lookup(name, expr.span) {
                    *addr = found.addr;
                }
            }
            ExprKind::List(items) => items.iter_mut().for_each(|item| self.expr(item)),
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter_mut().for_each(|arg| self.expr(arg));
            }
            ExprKind::Index { target, index } => {
                self.expr(target);
                self.expr(index);
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                self.scoped(then, &[]);
                if let Some(otherwise) = otherwise {
                    self.expr(otherwise);
                }
            }
            ExprKind::Block(block) => self.scoped(block, &[]),
            ExprKind::Fn(decl) => self.function(decl),
        }
    }
}
