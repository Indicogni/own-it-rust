//! The syntax tree (SPEC section 4).
//!
//! The parser builds this tree with every [`Addr`], `slot` and `slots` field
//! set to zero. The resolver (`resolve.rs`) then fills them in, so that the
//! evaluator never looks a name up by its text.

use std::rc::Rc;

use crate::source::Span;

/// A name as written in the source, with its position.
#[derive(Clone, Debug)]
pub struct Ident {
    pub name: Rc<str>,
    pub span: Span,
}

/// Where a binding lives at run time, relative to the place that uses it:
/// go `depth` environments outwards, then take slot number `slot`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Addr {
    pub depth: usize,
    pub slot: usize,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    /// From the first to the last token of the expression (SPEC 11.4).
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Nil,
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Name {
        name: Rc<str>,
        addr: Addr,
    },
    List(Vec<Expr>),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `and` and `or`: the right operand is not always evaluated.
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// `otherwise` is a `Block` or another `If`.
    If {
        cond: Box<Expr>,
        then: Block,
        otherwise: Option<Box<Expr>>,
    },
    Block(Block),
    /// A function literal. Shared, because every closure made from it keeps
    /// the body alive, also after the REPL entry that contained it is gone.
    Fn(Rc<FnDecl>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    /// The operator as written in the source.
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

/// `{ statements; tail }`, a function body, a loop body, or a whole program.
#[derive(Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The final expression without `;`: the value of the block.
    pub tail: Option<Box<Expr>>,
    /// How many bindings the scope of this block holds, parameters or the
    /// loop variable included. Zero means the block gets no environment of
    /// its own at run time.
    pub slots: usize,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmt {
    /// `let` or `var`. `slot` is in the scope of the enclosing block.
    Let {
        name: Ident,
        mutable: bool,
        init: Expr,
        slot: usize,
    },
    /// `fn name(...) { ... }`.
    Fn {
        name: Ident,
        decl: Rc<FnDecl>,
        slot: usize,
    },
    Assign {
        target: Target,
        value: Expr,
    },
    /// An expression whose value is thrown away.
    Expr(Expr),
    While {
        cond: Expr,
        body: Block,
    },
    /// The loop variable is slot 0 of the body's scope.
    For {
        var: Ident,
        iter: ForIter,
        body: Block,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
}

/// The left side of `=`.
#[derive(Debug)]
pub enum Target {
    Name {
        name: Ident,
        addr: Addr,
    },
    Index {
        target: Expr,
        index: Expr,
        span: Span,
    },
}

#[derive(Debug)]
pub enum ForIter {
    /// `for x in values`: a list or a string.
    Values(Expr),
    /// `for i in first..limit`.
    Range(Expr, Expr),
}

/// A function as written: the part that all its closures share.
#[derive(Debug)]
pub struct FnDecl {
    /// `None` for a function literal.
    pub name: Option<Rc<str>>,
    /// Parameters are slots `0..params.len()` of the body's scope.
    pub params: Vec<Ident>,
    pub body: Block,
}
