//! Run-time values and environments (SPEC sections 5 and 6).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::FnDecl;
use crate::builtins::Builtin;

/// A Tansy value. Cloning is cheap: at most a reference count goes up.
#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    /// Strings are immutable, so sharing one needs no `RefCell`.
    Str(Rc<str>),
    /// Lists are shared and mutable: two names can refer to one list.
    List(ListRef),
    Fn(Rc<Closure>),
    Builtin(Builtin),
}

pub type ListRef = Rc<RefCell<Vec<Value>>>;

/// A function value: the code, and the environment it was created in.
pub struct Closure {
    pub decl: Rc<FnDecl>,
    pub env: Rc<Env>,
}

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    /// The name of the value's type, as `type()` returns it and as error
    /// messages print it.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Fn(_) | Value::Builtin(_) => "fn",
        }
    }

    /// The quoted form (SPEC 5.3): like `Display`, but a string is written
    /// as a string literal.
    pub fn quoted(&self) -> Quoted<'_> {
        Quoted(self)
    }
}

/// `==` (SPEC 7.4). Values of different types are never equal.
pub fn equal(a: &Value, b: &Value) -> bool {
    equal_inner(a, b, &mut Vec::new())
}

/// `comparing` holds the pairs of lists whose comparison is under way.
/// Meeting such a pair again means the lists contain themselves; the pair
/// counts as equal, which lets the comparison of cyclic lists end. (The
/// specification does not require this; an endless loop would be allowed.)
fn equal_inner(a: &Value, b: &Value, comparing: &mut Vec<(*const (), *const ())>) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Fn(a), Value::Fn(b)) => Rc::ptr_eq(a, b),
        (Value::Builtin(a), Value::Builtin(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let pair = (Rc::as_ptr(a).cast::<()>(), Rc::as_ptr(b).cast::<()>());
            if comparing.contains(&pair) {
                return true;
            }
            comparing.push(pair);
            // Nothing can change the lists during the comparison, so both
            // may stay borrowed.
            let (a, b) = (a.borrow(), b.borrow());
            let same = a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| equal_inner(x, y, comparing));
            comparing.pop();
            same
        }
        _ => false,
    }
}

/// The display form (SPEC 5.3).
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(f, self, false, &mut Vec::new())
    }
}

/// See [`Value::quoted`].
pub struct Quoted<'a>(&'a Value);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(f, self.0, true, &mut Vec::new())
    }
}

/// `writing` holds the lists that are being written right now. A list that
/// turns up inside itself is written as `[...]`.
fn write_value(
    f: &mut fmt::Formatter<'_>,
    value: &Value,
    quoted: bool,
    writing: &mut Vec<*const RefCell<Vec<Value>>>,
) -> fmt::Result {
    match value {
        Value::Nil => f.write_str("nil"),
        Value::Bool(b) => write!(f, "{b}"),
        Value::Int(n) => write!(f, "{n}"),
        Value::Str(s) if quoted => write_quoted(f, s),
        Value::Str(s) => f.write_str(s),
        Value::List(list) => {
            let address = Rc::as_ptr(list);
            if writing.contains(&address) {
                return f.write_str("[...]");
            }
            writing.push(address);
            f.write_str("[")?;
            for (i, item) in list.borrow().iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write_value(f, item, true, writing)?;
            }
            writing.pop();
            f.write_str("]")
        }
        Value::Fn(closure) => match &closure.decl.name {
            Some(name) => write!(f, "<fn {name}>"),
            None => f.write_str("<fn>"),
        },
        Value::Builtin(builtin) => write!(f, "<fn {}>", builtin.name()),
    }
}

fn write_quoted(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    use fmt::Write;
    f.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\t' => f.write_str("\\t")?,
            '\r' => f.write_str("\\r")?,
            '\0' => f.write_str("\\0")?,
            other => f.write_char(other)?,
        }
    }
    f.write_char('"')
}

/// The bindings of one scope at run time, linked to the enclosing scope.
///
/// A slot is `None` until the declaration of its binding has run (SPEC 6.2).
/// Environments are reference counted because a closure can keep one alive
/// after the block that created it has ended.
pub struct Env {
    slots: RefCell<Vec<Option<Value>>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    pub fn new(slots: Vec<Option<Value>>, parent: Option<Rc<Env>>) -> Rc<Env> {
        Rc::new(Env {
            slots: RefCell::new(slots),
            parent,
        })
    }

    /// The environment `depth` steps outwards; 0 is `self`.
    pub fn ancestor(&self, depth: usize) -> &Env {
        let mut env = self;
        for _ in 0..depth {
            // The resolver computed `depth` from the same nesting that
            // created this chain.
            env = env
                .parent
                .as_deref()
                .expect("the resolver and the evaluator agree on scopes");
        }
        env
    }

    /// The value in `slot`, or `None` if its declaration has not run yet.
    pub fn get(&self, slot: usize) -> Option<Value> {
        self.slots.borrow()[slot].clone()
    }

    pub fn is_initialized(&self, slot: usize) -> bool {
        self.slots.borrow()[slot].is_some()
    }

    pub fn set(&self, slot: usize, value: Value) {
        // The old value is dropped after the borrow has ended.
        let old = self.slots.borrow_mut()[slot].replace(value);
        drop(old);
    }

    /// Makes room for `len` slots. Only the top level grows: each REPL
    /// entry can add names to it.
    pub fn grow(&self, len: usize) {
        let mut slots = self.slots.borrow_mut();
        if slots.len() < len {
            slots.resize(len, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.into())
    }

    #[test]
    fn display_and_quoted_forms() {
        let list = Value::list(vec![
            Value::Int(1),
            s("a\"b\n"),
            Value::Nil,
            Value::list(vec![Value::Bool(true)]),
        ]);
        assert_eq!(list.to_string(), r#"[1, "a\"b\n", nil, [true]]"#);
        assert_eq!(s("a\tb").to_string(), "a\tb");
        assert_eq!(s("a\tb").quoted().to_string(), r#""a\tb""#);
        assert_eq!(Value::Builtin(Builtin::Len).to_string(), "<fn len>");
    }

    #[test]
    fn a_list_inside_itself_is_written_as_dots() {
        let Value::List(inner) = Value::list(vec![Value::Int(1)]) else {
            unreachable!()
        };
        inner.borrow_mut().push(Value::List(Rc::clone(&inner)));
        let twice = Value::list(vec![
            Value::List(Rc::clone(&inner)),
            Value::List(Rc::clone(&inner)),
        ]);
        assert_eq!(Value::List(Rc::clone(&inner)).to_string(), "[1, [...]]");
        // The same list twice, side by side, is not a cycle.
        assert_eq!(twice.to_string(), "[[1, [...]], [1, [...]]]");
        // Break the cycle so that the test does not leak.
        inner.borrow_mut().clear();
    }

    #[test]
    fn equality_is_by_value_for_lists_and_strict_about_types() {
        let a = Value::list(vec![Value::Int(1), s("x")]);
        let b = Value::list(vec![Value::Int(1), s("x")]);
        assert!(equal(&a, &b));
        assert!(!equal(&a, &Value::list(vec![Value::Int(1)])));
        assert!(!equal(&Value::Int(0), &Value::Bool(false)));
        assert!(!equal(&Value::Nil, &Value::Bool(false)));
        assert!(!equal(&s("1"), &Value::Int(1)));
    }

    #[test]
    fn equality_of_cyclic_lists_ends() {
        let cyclic = || {
            let Value::List(list) = Value::list(vec![Value::Int(1)]) else {
                unreachable!()
            };
            list.borrow_mut().push(Value::List(Rc::clone(&list)));
            list
        };
        let (a, b) = (cyclic(), cyclic());
        assert!(equal(
            &Value::List(Rc::clone(&a)),
            &Value::List(Rc::clone(&b))
        ));
        a.borrow_mut().clear();
        b.borrow_mut().clear();
    }
}
