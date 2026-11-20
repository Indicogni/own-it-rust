//! The evaluator: walks the resolved tree (SPEC sections 6 to 10).
//!
//! Every construct that can stop early does so through the `Err` side of
//! [`Eval`]: a runtime error, but also `return`, `break`, `continue` and
//! `exit`. The `?` operator then unwinds the Rust call stack exactly as far
//! as the construct that handles it: a call handles `Return`, a loop handles
//! `Break` and `Continue`, and the driver handles the rest.

use std::fmt;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, FnDecl, ForIter, LogicalOp, Stmt, Target, UnaryOp,
};
use crate::builtins::{self, Builtin};
use crate::source::{Diagnostic, Span};
use crate::value::{self, Closure, Env, Value};

/// SPEC 13: at most this many calls of Tansy functions are active at once.
pub const MAX_CALL_DEPTH: usize = 1000;

/// How deeply `Interp::eval` may nest. A normal program never comes near
/// this: 1000 active calls of ordinary functions need a few thousand levels.
/// It exists so that a program built to nest absurdly (SPEC 13 calls that
/// "outside the limits") ends with `stack overflow` and not with a crash.
///
/// The number is tied to [`crate::STACK_SIZE`]: one level costs up to about
/// 2.6 KiB of stack in an unoptimised build (measured; see DESIGN.md), so
/// 50 000 levels use about half of the 256 MiB the evaluator runs on.
pub const MAX_EVAL_DEPTH: usize = 50_000;

/// Why evaluation stopped before producing a value.
pub enum Unwind {
    /// A runtime error.
    Error(Diagnostic),
    /// `return`, on its way to the call that is returning.
    Return(Value),
    /// `break` and `continue`, on their way to the enclosing loop.
    Break,
    Continue,
    /// `exit(code)`, on its way out of the program.
    Exit(u8),
    /// Standard output could not be written.
    Output(io::Error),
}

type Eval<T = Value> = Result<T, Unwind>;

fn error(span: Span, message: impl Into<String>) -> Unwind {
    Unwind::Error(Diagnostic::new(span, message))
}

fn fail<T>(span: Span, message: impl Into<String>) -> Eval<T> {
    Err(error(span, message))
}

/// What a loop does after one run of its body: `true` to go on.
fn after_body(result: Eval) -> Eval<bool> {
    match result {
        Ok(_) | Err(Unwind::Continue) => Ok(true),
        Err(Unwind::Break) => Ok(false),
        Err(other) => Err(other),
    }
}

fn arity_error(expected: usize, found: usize, call: Span) -> Unwind {
    let noun = if expected == 1 {
        "argument"
    } else {
        "arguments"
    };
    error(call, format!("expected {expected} {noun}, found {found}"))
}

/// The value of an index expression or of a range bound, which must be an
/// `int`. `expr` is the expression that `value` came from.
fn as_int(value: &Value, expr: &Expr) -> Eval<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        other => fail(
            expr.span,
            format!("expected int, found {}", other.type_name()),
        ),
    }
}

fn out_of_range(index: i64, len: usize) -> String {
    format!("index {index} is out of range for length {len}")
}

/// A one-character string.
fn char_value(c: char) -> Value {
    Value::Str(Rc::from(c.encode_utf8(&mut [0; 4]) as &str))
}

fn len_value(len: usize) -> Value {
    // A length that does not fit cannot exist in memory.
    Value::Int(i64::try_from(len).unwrap_or(i64::MAX))
}

pub struct Interp<'io> {
    /// The top-level environment. Its parent holds the built-in functions.
    globals: Rc<Env>,
    /// The environment of the code that is running now.
    env: Rc<Env>,
    out: &'io mut dyn Write,
    input: &'io mut dyn BufRead,
    /// Active calls of Tansy functions.
    calls: usize,
    /// Active calls of `eval`.
    depth: usize,
}

impl<'io> Interp<'io> {
    pub fn new(out: &'io mut dyn Write, input: &'io mut dyn BufRead) -> Interp<'io> {
        let builtins = Env::new(
            Builtin::ALL
                .iter()
                .map(|b| Some(Value::Builtin(*b)))
                .collect(),
            None,
        );
        let globals = Env::new(Vec::new(), Some(builtins));
        Interp {
            env: Rc::clone(&globals),
            globals,
            out,
            input,
            calls: 0,
            depth: 0,
        }
    }

    /// Runs a resolved program at the top level and returns the value of
    /// its tail expression. The top-level bindings stay for the next call.
    pub fn run(&mut self, program: &Block) -> Eval {
        self.globals.grow(program.slots);
        let result = self.sequence(program);
        // Every `in_env`, call and `eval` undoes its own bookkeeping, also
        // when it is left through `?`.
        debug_assert!(Rc::ptr_eq(&self.env, &self.globals));
        debug_assert_eq!((self.calls, self.depth), (0, 0));
        result
    }

    pub fn out(&mut self) -> &mut (dyn Write + 'io) {
        self.out
    }

    /// The next line of standard input without its line ending, or `None`
    /// at the end of input.
    pub fn read_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut line = Vec::new();
        if self.input.read_until(b'\n', &mut line)? == 0 {
            return Ok(None);
        }
        if line.last() == Some(&b'\n') {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }
        Ok(Some(line))
    }

    fn emit(&mut self, text: fmt::Arguments<'_>) -> Eval<()> {
        self.out.write_fmt(text).map_err(Unwind::Output)
    }

    // ----- scopes ---------------------------------------------------------

    /// Runs `body` with `env` as the current environment.
    fn in_env<T>(&mut self, env: Rc<Env>, body: impl FnOnce(&mut Self) -> Eval<T>) -> Eval<T> {
        let outer = std::mem::replace(&mut self.env, env);
        let result = body(self);
        self.env = outer;
        result
    }

    /// A block as an expression or a loop body: fresh bindings each time it
    /// is entered (SPEC 6.1).
    fn block(&mut self, block: &Block) -> Eval {
        if block.slots == 0 {
            return self.sequence(block);
        }
        let env = Env::new(vec![None; block.slots], Some(Rc::clone(&self.env)));
        self.in_env(env, |this| this.sequence(block))
    }

    /// The statements and the tail of `block`, in the current environment.
    fn sequence(&mut self, block: &Block) -> Eval {
        for stmt in &block.stmts {
            self.statement(stmt)?;
        }
        match &block.tail {
            Some(tail) => self.eval(tail),
            None => Ok(Value::Nil),
        }
    }

    // ----- statements -----------------------------------------------------

    fn statement(&mut self, stmt: &Stmt) -> Eval<()> {
        match stmt {
            Stmt::Let { init, slot, .. } => {
                let value = self.eval(init)?;
                self.env.set(*slot, value);
            }
            Stmt::Fn { decl, slot, .. } => {
                let function = self.closure(decl);
                self.env.set(*slot, function);
            }
            Stmt::Assign {
                target: Target::Name { name, addr },
                value,
            } => {
                let value = self.eval(value)?;
                let env = self.env.ancestor(addr.depth);
                if !env.is_initialized(addr.slot) {
                    return fail(
                        name.span,
                        format!("`{}` is used before it is initialized", name.name),
                    );
                }
                env.set(addr.slot, value);
            }
            Stmt::Assign {
                target:
                    Target::Index {
                        target,
                        index,
                        span,
                    },
                value,
            } => {
                let target_value = self.eval(target)?;
                let index_value = self.eval(index)?;
                let value = self.eval(value)?;
                let Value::List(list) = &target_value else {
                    return fail(
                        target.span,
                        format!("cannot assign into {}", target_value.type_name()),
                    );
                };
                let i = as_int(&index_value, index)?;
                let mut items = list.borrow_mut();
                let len = items.len();
                let Some(item) = usize::try_from(i).ok().and_then(|i| items.get_mut(i)) else {
                    return fail(*span, out_of_range(i, len));
                };
                let old = std::mem::replace(item, value);
                // End the borrow before the old element is dropped.
                drop(items);
                drop(old);
            }
            Stmt::Expr(expr) => {
                self.eval(expr)?;
            }
            Stmt::While { cond, body } => {
                while self.condition(cond)? {
                    if !after_body(self.block(body))? {
                        break;
                    }
                }
            }
            Stmt::For { iter, body, .. } => self.for_loop(iter, body)?,
            Stmt::Return { value, .. } => {
                let value = match value {
                    Some(value) => self.eval(value)?,
                    None => Value::Nil,
                };
                return Err(Unwind::Return(value));
            }
            Stmt::Break(_) => return Err(Unwind::Break),
            Stmt::Continue(_) => return Err(Unwind::Continue),
        }
        Ok(())
    }

    fn for_loop(&mut self, iter: &ForIter, body: &Block) -> Eval<()> {
        match iter {
            ForIter::Range(first, limit) => {
                // Evaluate both bounds, then check both (SPEC 8.4): the same
                // order as for the operands of an operator.
                let first_value = self.eval(first)?;
                let limit_value = self.eval(limit)?;
                let first = as_int(&first_value, first)?;
                let limit = as_int(&limit_value, limit)?;
                for i in first..limit {
                    if !self.iteration(body, Value::Int(i))? {
                        break;
                    }
                }
            }
            ForIter::Values(values) => match self.eval(values)? {
                Value::List(list) => {
                    // By index, and looking at the list anew each time: the
                    // body may change the list (SPEC 8.4).
                    let mut i = 0;
                    loop {
                        let item = list.borrow().get(i).cloned();
                        let Some(item) = item else { break };
                        if !self.iteration(body, item)? {
                            break;
                        }
                        i += 1;
                    }
                }
                Value::Str(text) => {
                    for c in text.chars() {
                        if !self.iteration(body, char_value(c))? {
                            break;
                        }
                    }
                }
                other => {
                    return fail(
                        values.span,
                        format!("cannot iterate over {}", other.type_name()),
                    );
                }
            },
        }
        Ok(())
    }

    /// One run of a `for` body, with a fresh loop variable in slot 0.
    fn iteration(&mut self, body: &Block, item: Value) -> Eval<bool> {
        let mut slots = Vec::with_capacity(body.slots);
        slots.push(Some(item));
        slots.resize(body.slots, None);
        let env = Env::new(slots, Some(Rc::clone(&self.env)));
        after_body(self.in_env(env, |this| this.sequence(body)))
    }

    // ----- expressions ----------------------------------------------------

    fn eval(&mut self, expr: &Expr) -> Eval {
        if self.depth == MAX_EVAL_DEPTH {
            return fail(expr.span, "stack overflow");
        }
        self.depth += 1;
        let result = match &expr.kind {
            ExprKind::Nil => Ok(Value::Nil),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Str(s) => Ok(Value::Str(Rc::clone(s))),
            ExprKind::Name { name, addr } => match self.env.ancestor(addr.depth).get(addr.slot) {
                Some(value) => Ok(value),
                None => fail(
                    expr.span,
                    format!("`{name}` is used before it is initialized"),
                ),
            },
            ExprKind::List(items) => self.list(items),
            ExprKind::Unary { op, operand } => self.unary(*op, operand, expr.span),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, expr.span),
            ExprKind::Logical { op, left, right } => self.logical(*op, left, right),
            ExprKind::Call { callee, args } => self.call(callee, args, expr.span),
            ExprKind::Index { target, index } => self.index(target, index, expr.span),
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => self.if_else(cond, then, otherwise.as_deref()),
            ExprKind::Block(block) => self.block(block),
            ExprKind::Fn(decl) => Ok(self.closure(decl)),
        };
        self.depth -= 1;
        result
    }

    /// Evaluates an expression that must be a `bool`.
    fn condition(&mut self, expr: &Expr) -> Eval<bool> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            other => fail(
                expr.span,
                format!("expected bool, found {}", other.type_name()),
            ),
        }
    }

    fn closure(&self, decl: &Rc<FnDecl>) -> Value {
        Value::Fn(Rc::new(Closure {
            decl: Rc::clone(decl),
            env: Rc::clone(&self.env),
        }))
    }

    fn list(&mut self, items: &[Expr]) -> Eval {
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            values.push(self.eval(item)?);
        }
        Ok(Value::list(values))
    }

    fn if_else(&mut self, cond: &Expr, then: &Block, otherwise: Option<&Expr>) -> Eval {
        if self.condition(cond)? {
            self.block(then)
        } else {
            match otherwise {
                Some(otherwise) => self.eval(otherwise),
                None => Ok(Value::Nil),
            }
        }
    }

    fn logical(&mut self, op: LogicalOp, left: &Expr, right: &Expr) -> Eval {
        let left = self.condition(left)?;
        let decided = match op {
            LogicalOp::And => !left,
            LogicalOp::Or => left,
        };
        Ok(Value::Bool(if decided {
            left
        } else {
            self.condition(right)?
        }))
    }

    fn unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Eval {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!self.condition(operand)?)),
            UnaryOp::Neg => match self.eval(operand)? {
                Value::Int(n) => n
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| error(span, "integer overflow")),
                other => fail(span, format!("cannot apply `-` to {}", other.type_name())),
            },
        }
    }

    fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, span: Span) -> Eval {
        let left = self.eval(left)?;
        let right = self.eval(right)?;
        let checked = |result: Option<i64>| {
            result
                .map(Value::Int)
                .ok_or_else(|| error(span, "integer overflow"))
        };

        match (op, &left, &right) {
            (BinaryOp::Eq, a, b) => Ok(Value::Bool(value::equal(a, b))),
            (BinaryOp::Ne, a, b) => Ok(Value::Bool(!value::equal(a, b))),

            (BinaryOp::Add, Value::Int(a), Value::Int(b)) => checked(a.checked_add(*b)),
            (BinaryOp::Sub, Value::Int(a), Value::Int(b)) => checked(a.checked_sub(*b)),
            (BinaryOp::Mul, Value::Int(a), Value::Int(b)) => checked(a.checked_mul(*b)),
            (BinaryOp::Div | BinaryOp::Rem, Value::Int(_), Value::Int(0)) => {
                fail(span, "division by zero")
            }
            // `i64::MIN / -1` does not fit.
            (BinaryOp::Div, Value::Int(a), Value::Int(b)) => checked(a.checked_div(*b)),
            // `i64::MIN % -1` is 0, but the hardware operation overflows.
            (BinaryOp::Rem, Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a.checked_rem(*b).unwrap_or(0)))
            }

            (BinaryOp::Add, Value::Str(a), Value::Str(b)) => {
                let mut joined = String::with_capacity(a.len() + b.len());
                joined.push_str(a);
                joined.push_str(b);
                Ok(Value::Str(joined.into()))
            }
            (BinaryOp::Add, Value::List(a), Value::List(b)) => {
                // `a` and `b` can be the same list; two shared borrows of
                // one `RefCell` are fine.
                let (a, b) = (a.borrow(), b.borrow());
                Ok(Value::list(a.iter().chain(b.iter()).cloned().collect()))
            }

            (BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge, a, b) => {
                let ordering = match (a, b) {
                    (Value::Int(a), Value::Int(b)) => a.cmp(b),
                    // `str` compares bytes, and for UTF-8 that is the same
                    // order as comparing code points.
                    (Value::Str(a), Value::Str(b)) => a.cmp(b),
                    _ => return fail(span, cannot_apply(op, a, b)),
                };
                Ok(Value::Bool(match op {
                    BinaryOp::Lt => ordering.is_lt(),
                    BinaryOp::Le => ordering.is_le(),
                    BinaryOp::Gt => ordering.is_gt(),
                    _ => ordering.is_ge(),
                }))
            }

            (_, a, b) => fail(span, cannot_apply(op, a, b)),
        }
    }

    fn index(&mut self, target: &Expr, index: &Expr, span: Span) -> Eval {
        let target_value = self.eval(target)?;
        let index_value = self.eval(index)?;
        // A negative index fails the conversion to `usize`: out of range.
        match &target_value {
            Value::List(list) => {
                let i = as_int(&index_value, index)?;
                let list = list.borrow();
                let item = usize::try_from(i).ok().and_then(|i| list.get(i));
                item.cloned()
                    .ok_or_else(|| error(span, out_of_range(i, list.len())))
            }
            Value::Str(text) => {
                let i = as_int(&index_value, index)?;
                let c = usize::try_from(i).ok().and_then(|i| text.chars().nth(i));
                c.map(char_value)
                    .ok_or_else(|| error(span, out_of_range(i, text.chars().count())))
            }
            other => fail(target.span, format!("cannot index {}", other.type_name())),
        }
    }

    // ----- calls ----------------------------------------------------------

    /// SPEC 7.6: the callee, then the arguments from left to right, then
    /// the checks, then the call.
    fn call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Eval {
        match self.eval(callee)? {
            Value::Fn(closure) => {
                let decl = &closure.decl;
                // The arguments are evaluated straight into the slots of
                // the new environment: parameters are its first bindings.
                let mut slots = Vec::with_capacity(args.len().max(decl.body.slots));
                for arg in args {
                    slots.push(Some(self.eval(arg)?));
                }
                if args.len() != decl.params.len() {
                    return Err(arity_error(decl.params.len(), args.len(), span));
                }
                if self.calls == MAX_CALL_DEPTH {
                    return fail(span, "stack overflow");
                }
                let env = if decl.body.slots == 0 {
                    // No parameters and no declarations: nothing to store.
                    Rc::clone(&closure.env)
                } else {
                    slots.resize(decl.body.slots, None);
                    Env::new(slots, Some(Rc::clone(&closure.env)))
                };
                self.calls += 1;
                let result = self.in_env(env, |this| this.sequence(&decl.body));
                self.calls -= 1;
                match result {
                    Err(Unwind::Return(value)) => Ok(value),
                    other => other,
                }
            }
            Value::Builtin(builtin) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg)?);
                }
                self.builtin(builtin, &values, args, span)
            }
            other => {
                for arg in args {
                    self.eval(arg)?;
                }
                fail(callee.span, format!("cannot call {}", other.type_name()))
            }
        }
    }

    /// `values[i]` is the value of the argument expression `exprs[i]`.
    fn builtin(&mut self, builtin: Builtin, values: &[Value], exprs: &[Expr], call: Span) -> Eval {
        let wrong_type = |index: usize, expected: &str| {
            let found = values[index].type_name();
            fail(
                exprs[index].span,
                format!("{}: expected {expected}, found {found}", builtin.name()),
            )
        };
        match (builtin, values) {
            (Builtin::Print, values) => {
                for (i, value) in values.iter().enumerate() {
                    let space = if i > 0 { " " } else { "" };
                    self.emit(format_args!("{space}{value}"))?;
                }
                self.emit(format_args!("\n"))?;
                Ok(Value::Nil)
            }
            (Builtin::Input, []) => {
                // Whatever the script printed as a prompt must be visible
                // before the read blocks.
                self.out.flush().map_err(Unwind::Output)?;
                match self.read_line() {
                    Ok(None) => Ok(Value::Nil),
                    Ok(Some(bytes)) => match String::from_utf8(bytes) {
                        Ok(line) => Ok(Value::Str(line.into())),
                        Err(_) => fail(call, "input: invalid UTF-8"),
                    },
                    Err(e) => fail(call, format!("input: {e}")),
                }
            }
            (Builtin::Len, [Value::Str(text)]) => Ok(len_value(text.chars().count())),
            (Builtin::Len, [Value::List(list)]) => Ok(len_value(list.borrow().len())),
            (Builtin::Len, [_]) => wrong_type(0, "str or list"),
            (Builtin::Push, [Value::List(list), item]) => {
                list.borrow_mut().push(item.clone());
                Ok(Value::Nil)
            }
            (Builtin::Push, [_, _]) => wrong_type(0, "list"),
            (Builtin::Pop, [Value::List(list)]) => {
                let last = list.borrow_mut().pop();
                last.ok_or_else(|| error(call, "pop: the list is empty"))
            }
            (Builtin::Pop, [_]) => wrong_type(0, "list"),
            (Builtin::Str, [Value::Str(text)]) => Ok(Value::Str(Rc::clone(text))),
            (Builtin::Str, [value]) => Ok(Value::Str(value.to_string().into())),
            (Builtin::Int, [Value::Int(n)]) => Ok(Value::Int(*n)),
            (Builtin::Int, [Value::Str(text)]) => {
                Ok(builtins::parse_int(text).map_or(Value::Nil, Value::Int))
            }
            (Builtin::Int, [_]) => wrong_type(0, "str or int"),
            (Builtin::Type, [value]) => Ok(Value::Str(value.type_name().into())),
            (Builtin::Exit, [Value::Int(code)]) => match u8::try_from(*code) {
                Ok(code) if code <= 125 => Err(Unwind::Exit(code)),
                _ => fail(
                    exprs[0].span,
                    format!("exit: expected a code from 0 to 125, found {code}"),
                ),
            },
            (Builtin::Exit, [_]) => wrong_type(0, "int"),
            // Every pattern above fixes the number of arguments, so what is
            // left is a call with the wrong number.
            (builtin, values) => Err(arity_error(
                builtin.arity().unwrap_or(values.len()),
                values.len(),
                call,
            )),
        }
    }
}

fn cannot_apply(op: BinaryOp, left: &Value, right: &Value) -> String {
    format!(
        "cannot apply `{}` to {} and {}",
        op.symbol(),
        left.type_name(),
        right.type_name()
    )
}
