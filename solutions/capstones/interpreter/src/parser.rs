//! Tokens to tree (SPEC section 4).
//!
//! Statements are parsed by plain recursive descent, expressions by a Pratt
//! parser: one function, [`Parser::expr`], and a table of binding powers.
//!
//! The parser stops at the first token it cannot accept. It has no error
//! recovery, because the specification asks for exactly one error.

use std::rc::Rc;

use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, FnDecl, ForIter, Ident, LogicalOp, Stmt, Target, UnaryOp,
};
use crate::lexer::{self, Token, TokenKind};
use crate::source::{Diagnostic, Span};

/// SPEC 13: at most this many `(`, `[` and `{` may be open at once.
pub const MAX_OPEN_DELIMITERS: usize = 100;

/// Bounds on nesting that needs no delimiter: chains of prefix operators and
/// of `if`, and the number of operators applied one after another at one
/// level (`a + b + c + ...`, `f()()()...`). SPEC 13.2 allows rejecting such
/// runs when they are longer than 250. Together with `MAX_OPEN_DELIMITERS`
/// these bound the depth of the tree, and with it the recursion of every
/// later pass, including the one that frees the tree.
const MAX_PREFIX_NESTING: usize = 1000;
const MAX_CHAIN: usize = 1000;

// Binding powers, weakest first. An infix operator has a left and a right
// power; `left < right` makes it left-associative. A prefix operator has
// only a right power: how much of what follows it takes as its operand.
const NOT_POWER: u8 = 5;
const NEG_POWER: u8 = 13;

#[derive(Clone, Copy)]
enum Infix {
    Binary(BinaryOp),
    Logical(LogicalOp),
}

/// The infix operator a token stands for, with its left and right power.
fn infix(kind: &TokenKind) -> Option<(Infix, u8, u8)> {
    use Infix::{Binary, Logical};
    Some(match kind {
        TokenKind::Or => (Logical(LogicalOp::Or), 1, 2),
        TokenKind::And => (Logical(LogicalOp::And), 3, 4),
        // `not` sits here, at 5.
        TokenKind::EqEq => (Binary(BinaryOp::Eq), 7, 8),
        TokenKind::BangEq => (Binary(BinaryOp::Ne), 7, 8),
        TokenKind::Lt => (Binary(BinaryOp::Lt), 7, 8),
        TokenKind::LtEq => (Binary(BinaryOp::Le), 7, 8),
        TokenKind::Gt => (Binary(BinaryOp::Gt), 7, 8),
        TokenKind::GtEq => (Binary(BinaryOp::Ge), 7, 8),
        TokenKind::Plus => (Binary(BinaryOp::Add), 9, 10),
        TokenKind::Minus => (Binary(BinaryOp::Sub), 9, 10),
        TokenKind::Star => (Binary(BinaryOp::Mul), 11, 12),
        TokenKind::Slash => (Binary(BinaryOp::Div), 11, 12),
        TokenKind::Percent => (Binary(BinaryOp::Rem), 11, 12),
        // Unary `-` sits here, at 13. Calls and indexing bind tighter than
        // everything and are handled before this table is consulted.
        _ => return None,
    })
}

fn is_comparison(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
    )
}

type Parsed<T> = Result<T, Diagnostic>;

/// Parses a whole program: `text` from byte offset `start` to its end.
///
/// A script is parsed from offset 0. The REPL keeps everything that was
/// typed in one text and parses each entry from where it starts, so that
/// every span, also one inside a function from an earlier entry, is an
/// offset into the same text (SPEC 12.5).
///
/// Addresses and slot counts in the result are still zero; see `resolve.rs`.
pub fn parse(text: &str, start: usize) -> Parsed<Block> {
    if u32::try_from(text.len()).is_err() {
        return Err(Diagnostic::new(
            Span::default(),
            "the source text is too large",
        ));
    }
    let tokens = lexer::tokenize(text, start);
    let mut parser = Parser {
        text,
        tokens,
        pos: 0,
        last_end: 0,
        open: 0,
        prefixes: 0,
    };
    let (stmts, tail) = parser.sequence(false)?;
    Ok(Block {
        stmts,
        tail,
        slots: 0,
        span: Span::new(start, text.len()),
    })
}

struct Parser<'a> {
    text: &'a str,
    /// Never empty: the last token is `Eof`.
    tokens: Vec<Token>,
    /// Index of the next token. Never moves past the `Eof` token.
    pos: usize,
    /// Where the most recently consumed token ends.
    last_end: u32,
    /// How many delimiters are open right now.
    open: usize,
    /// How many prefix operators and `if`s are waiting for their end.
    prefixes: usize,
}

impl Parser<'_> {
    // ----- tokens ---------------------------------------------------------

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    /// The token after the next one (or `Eof`).
    fn peek_second(&self) -> &TokenKind {
        &self.tokens[(self.pos + 1).min(self.tokens.len() - 1)].kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn advance(&mut self) -> Span {
        let span = self.peek_span();
        self.last_end = span.end;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        span
    }

    /// The span from `start` to the end of the last consumed token.
    fn span_from(&self, start: Span) -> Span {
        Span {
            start: start.start,
            end: self.last_end,
        }
    }

    /// The error for "the next token cannot come here" (SPEC 11.5). If the
    /// next token is not a token at all, the lexical error is the message.
    fn unexpected(&self, what: &str) -> Diagnostic {
        let token = &self.tokens[self.pos];
        let message = match &token.kind {
            TokenKind::Invalid(error) => error.to_string(),
            TokenKind::Eof => format!("expected {what}, found end of input"),
            _ => format!(
                "expected {what}, found `{}`",
                &self.text[token.span.start as usize..token.span.end as usize]
            ),
        };
        Diagnostic::new(token.span, message)
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Parsed<Span> {
        if self.at(kind) {
            Ok(self.advance())
        } else {
            Err(self.unexpected(what))
        }
    }

    /// Consumes an opening delimiter and enforces the nesting limit.
    fn open(&mut self, kind: &TokenKind, what: &str) -> Parsed<Span> {
        let span = self.expect(kind, what)?;
        self.open += 1;
        if self.open > MAX_OPEN_DELIMITERS {
            return Err(Diagnostic::new(span, "nesting too deep"));
        }
        Ok(span)
    }

    fn close(&mut self, kind: &TokenKind, what: &str) -> Parsed<Span> {
        let span = self.expect(kind, what)?;
        self.open -= 1;
        Ok(span)
    }

    fn name(&mut self) -> Parsed<Ident> {
        let span = self.expect(&TokenKind::Ident, "a name")?;
        Ok(Ident {
            name: self.text[span.start as usize..span.end as usize].into(),
            span,
        })
    }

    /// Items separated by commas up to `closer`, which is left unconsumed.
    /// A comma after the last item is allowed.
    fn comma_separated<T>(
        &mut self,
        closer: &TokenKind,
        what_after_item: &str,
        mut item: impl FnMut(&mut Self) -> Parsed<T>,
    ) -> Parsed<Vec<T>> {
        let mut items = Vec::new();
        while !self.at(closer) {
            items.push(item(self)?);
            if self.at(&TokenKind::Comma) {
                self.advance();
            } else if !self.at(closer) {
                return Err(self.unexpected(what_after_item));
            }
        }
        Ok(items)
    }

    // ----- statements -----------------------------------------------------

    /// True at the token that ends the sequence being parsed: `}` inside a
    /// block, the end of input at the top level.
    fn at_sequence_end(&self, in_block: bool) -> bool {
        matches!(
            (self.peek(), in_block),
            (TokenKind::RBrace, true) | (TokenKind::Eof, false)
        )
    }

    /// Statements up to the end of the block or of the input, which is left
    /// unconsumed. The second result is the tail expression, if any.
    fn sequence(&mut self, in_block: bool) -> Parsed<(Vec<Stmt>, Option<Box<Expr>>)> {
        let mut stmts = Vec::new();
        loop {
            if self.at_sequence_end(in_block) {
                return Ok((stmts, None));
            }
            match self.peek() {
                // Only reachable inside a block: the block was never closed.
                TokenKind::Eof => return Err(self.unexpected("`}`")),
                TokenKind::Semicolon => {
                    self.advance();
                }
                TokenKind::Let | TokenKind::Var => stmts.push(self.let_statement()?),
                // `fn name` declares; `fn (` starts an expression.
                TokenKind::Fn if *self.peek_second() == TokenKind::Ident => {
                    stmts.push(self.fn_statement()?)
                }
                TokenKind::Return => stmts.push(self.return_statement()?),
                TokenKind::Break => {
                    let span = self.advance();
                    self.end_statement()?;
                    stmts.push(Stmt::Break(span));
                }
                TokenKind::Continue => {
                    let span = self.advance();
                    self.end_statement()?;
                    stmts.push(Stmt::Continue(span));
                }
                TokenKind::While => {
                    self.advance();
                    let cond = self.expression()?;
                    let body = self.block()?;
                    stmts.push(Stmt::While { cond, body });
                }
                TokenKind::For => stmts.push(self.for_statement()?),
                // A statement that starts with `if` or `{` ends at its last
                // `}`: no operator, call or `;` continues it (SPEC 4.3).
                TokenKind::If | TokenKind::LBrace => {
                    let expr = if self.at(&TokenKind::If) {
                        self.if_expression()?
                    } else {
                        self.block_expression()?
                    };
                    if self.at_sequence_end(in_block) {
                        return Ok((stmts, Some(Box::new(expr))));
                    }
                    stmts.push(Stmt::Expr(expr));
                }
                _ => {
                    let expr = self.expression()?;
                    match self.peek() {
                        TokenKind::Eq => {
                            let target = assignment_target(expr)?;
                            self.advance();
                            let value = self.expression()?;
                            self.end_statement()?;
                            stmts.push(Stmt::Assign { target, value });
                        }
                        TokenKind::Semicolon => {
                            self.advance();
                            stmts.push(Stmt::Expr(expr));
                        }
                        _ if self.at_sequence_end(in_block) => {
                            return Ok((stmts, Some(Box::new(expr))));
                        }
                        // The statement is complete; the next round of the
                        // loop reports the misplaced `}` or end of input.
                        TokenKind::RBrace | TokenKind::Eof => stmts.push(Stmt::Expr(expr)),
                        _ => return Err(self.unexpected("`;`")),
                    }
                }
            }
        }
    }

    /// After a simple statement: a `;`, or nothing if `}` or the end of
    /// input follows.
    fn end_statement(&mut self) -> Parsed<()> {
        match self.peek() {
            TokenKind::Semicolon => {
                self.advance();
                Ok(())
            }
            TokenKind::RBrace | TokenKind::Eof => Ok(()),
            _ => Err(self.unexpected("`;`")),
        }
    }

    fn let_statement(&mut self) -> Parsed<Stmt> {
        let mutable = self.at(&TokenKind::Var);
        self.advance();
        let name = self.name()?;
        self.expect(&TokenKind::Eq, "`=`")?;
        let init = self.expression()?;
        self.end_statement()?;
        Ok(Stmt::Let {
            name,
            mutable,
            init,
            slot: 0,
        })
    }

    fn fn_statement(&mut self) -> Parsed<Stmt> {
        self.advance();
        let name = self.name()?;
        let decl = self.function(Some(Rc::clone(&name.name)))?;
        Ok(Stmt::Fn {
            name,
            decl,
            slot: 0,
        })
    }

    fn return_statement(&mut self) -> Parsed<Stmt> {
        let span = self.advance();
        let value = match self.peek() {
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof => None,
            _ => Some(self.expression()?),
        };
        self.end_statement()?;
        Ok(Stmt::Return { value, span })
    }

    fn for_statement(&mut self) -> Parsed<Stmt> {
        self.advance();
        let var = self.name()?;
        self.expect(&TokenKind::In, "`in`")?;
        let first = self.expression()?;
        let iter = if self.at(&TokenKind::DotDot) {
            self.advance();
            ForIter::Range(first, self.expression()?)
        } else {
            ForIter::Values(first)
        };
        let body = self.block()?;
        Ok(Stmt::For { var, iter, body })
    }

    /// `( parameters ) { body }`, after `fn` and the optional name.
    fn function(&mut self, name: Option<Rc<str>>) -> Parsed<Rc<FnDecl>> {
        self.open(&TokenKind::LParen, "`(`")?;
        let params = self.comma_separated(&TokenKind::RParen, "`,` or `)`", Self::name)?;
        self.close(&TokenKind::RParen, "`,` or `)`")?;
        let body = self.block()?;
        Ok(Rc::new(FnDecl { name, params, body }))
    }

    fn block(&mut self) -> Parsed<Block> {
        let start = self.open(&TokenKind::LBrace, "`{`")?;
        let (stmts, tail) = self.sequence(true)?;
        self.close(&TokenKind::RBrace, "`}`")?;
        Ok(Block {
            stmts,
            tail,
            slots: 0,
            span: self.span_from(start),
        })
    }

    // ----- expressions ----------------------------------------------------

    fn expression(&mut self) -> Parsed<Expr> {
        self.expr(0)
    }

    /// The Pratt loop. Parses an expression made of operators that bind at
    /// least as strongly as `min_power`.
    fn expr(&mut self, min_power: u8) -> Parsed<Expr> {
        let start = self.peek_span();
        let mut left = self.prefix(min_power)?;
        // How many operators have been applied to `left` at this level.
        let mut chain = 0;
        loop {
            let kind = if self.at(&TokenKind::LParen) {
                self.chain_link(&mut chain)?;
                self.open(&TokenKind::LParen, "`(`")?;
                let args =
                    self.comma_separated(&TokenKind::RParen, "`,` or `)`", Self::expression)?;
                self.close(&TokenKind::RParen, "`,` or `)`")?;
                ExprKind::Call {
                    callee: Box::new(left),
                    args,
                }
            } else if self.at(&TokenKind::LBracket) {
                self.chain_link(&mut chain)?;
                self.open(&TokenKind::LBracket, "`[`")?;
                let index = self.expression()?;
                self.close(&TokenKind::RBracket, "`]`")?;
                ExprKind::Index {
                    target: Box::new(left),
                    index: Box::new(index),
                }
            } else if let Some((op, left_power, right_power)) = infix(self.peek())
                && left_power >= min_power
            {
                self.chain_link(&mut chain)?;
                let comparing = is_comparison(self.peek());
                self.advance();
                let right = Box::new(self.expr(right_power)?);
                // The right operand of a comparison stops before the next
                // comparison operator (its power is too low), so a chain
                // like `a < b < c` shows up here, as a second operator.
                if comparing && is_comparison(self.peek()) {
                    return Err(Diagnostic::new(
                        self.peek_span(),
                        "comparison operators cannot be chained",
                    ));
                }
                let left = Box::new(left);
                match op {
                    Infix::Logical(op) => ExprKind::Logical { op, left, right },
                    Infix::Binary(op) => ExprKind::Binary { op, left, right },
                }
            } else {
                return Ok(left);
            };
            // From the first token of the left operand to the last token
            // consumed: parentheses around operands are inside the span,
            // parentheses around the whole expression are not (SPEC 11.4).
            left = Expr {
                kind,
                span: self.span_from(start),
            };
        }
    }

    fn chain_link(&self, chain: &mut usize) -> Parsed<()> {
        *chain += 1;
        if *chain > MAX_CHAIN {
            Err(Diagnostic::new(self.peek_span(), "nesting too deep"))
        } else {
            Ok(())
        }
    }

    /// Enters a prefix operator or an `if`; `leave_prefix` undoes it.
    fn enter_prefix(&mut self) -> Parsed<()> {
        self.prefixes += 1;
        if self.prefixes > MAX_PREFIX_NESTING {
            Err(Diagnostic::new(self.peek_span(), "nesting too deep"))
        } else {
            Ok(())
        }
    }

    fn leave_prefix(&mut self) {
        self.prefixes -= 1;
    }

    /// A prefix operator with its operand, or a primary expression.
    fn prefix(&mut self, min_power: u8) -> Parsed<Expr> {
        let (op, power) = match self.peek() {
            TokenKind::Minus => (UnaryOp::Neg, NEG_POWER),
            // `not` binds more weakly than comparison and arithmetic, so it
            // cannot be the operand of one of those: `1 + not x` is an error.
            TokenKind::Not if min_power <= NOT_POWER => (UnaryOp::Not, NOT_POWER),
            _ => return self.primary(),
        };
        self.enter_prefix()?;
        let start = self.advance();
        let operand = Box::new(self.expr(power)?);
        self.leave_prefix();
        Ok(Expr {
            kind: ExprKind::Unary { op, operand },
            span: self.span_from(start),
        })
    }

    fn primary(&mut self) -> Parsed<Expr> {
        let start = self.peek_span();
        let kind = match self.peek() {
            TokenKind::Int(value) => ExprKind::Int(*value),
            TokenKind::Str(value) => ExprKind::Str(Rc::clone(value)),
            TokenKind::True => ExprKind::Bool(true),
            TokenKind::False => ExprKind::Bool(false),
            TokenKind::Nil => ExprKind::Nil,
            TokenKind::Ident => {
                let name = self.name()?;
                return Ok(Expr {
                    kind: ExprKind::Name {
                        name: name.name,
                        addr: Default::default(),
                    },
                    span: start,
                });
            }
            TokenKind::LParen => {
                self.open(&TokenKind::LParen, "`(`")?;
                let inner = self.expression()?;
                self.close(&TokenKind::RParen, "`)`")?;
                // The inner expression keeps its own span.
                return Ok(inner);
            }
            TokenKind::LBracket => {
                self.open(&TokenKind::LBracket, "`[`")?;
                let items =
                    self.comma_separated(&TokenKind::RBracket, "`,` or `]`", Self::expression)?;
                self.close(&TokenKind::RBracket, "`,` or `]`")?;
                return Ok(Expr {
                    kind: ExprKind::List(items),
                    span: self.span_from(start),
                });
            }
            TokenKind::Fn => {
                self.advance();
                let decl = self.function(None)?;
                return Ok(Expr {
                    kind: ExprKind::Fn(decl),
                    span: self.span_from(start),
                });
            }
            TokenKind::If => return self.if_expression(),
            TokenKind::LBrace => return self.block_expression(),
            _ => return Err(self.unexpected("expression")),
        };
        // The literals: one token each.
        self.advance();
        Ok(Expr { kind, span: start })
    }

    fn block_expression(&mut self) -> Parsed<Expr> {
        let block = self.block()?;
        let span = block.span;
        Ok(Expr {
            kind: ExprKind::Block(block),
            span,
        })
    }

    fn if_expression(&mut self) -> Parsed<Expr> {
        self.enter_prefix()?;
        let start = self.advance();
        let cond = Box::new(self.expression()?);
        let then = self.block()?;
        let otherwise = if self.at(&TokenKind::Else) {
            self.advance();
            Some(Box::new(match self.peek() {
                TokenKind::If => self.if_expression()?,
                TokenKind::LBrace => self.block_expression()?,
                _ => return Err(self.unexpected("`if` or `{`")),
            }))
        } else {
            None
        };
        self.leave_prefix();
        Ok(Expr {
            kind: ExprKind::If {
                cond,
                then,
                otherwise,
            },
            span: self.span_from(start),
        })
    }
}

/// Turns the expression left of `=` into an assignment target.
fn assignment_target(expr: Expr) -> Parsed<Target> {
    match expr.kind {
        ExprKind::Name { name, addr } => Ok(Target::Name {
            name: Ident {
                name,
                span: expr.span,
            },
            addr,
        }),
        ExprKind::Index { target, index } => Ok(Target::Index {
            target: *target,
            index: *index,
            span: expr.span,
        }),
        _ => Err(Diagnostic::new(expr.span, "invalid assignment target")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree as a fully parenthesised string, to make grouping visible.
    fn show(expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Nil => "nil".into(),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Str(s) => format!("{s:?}"),
            ExprKind::Name { name, .. } => name.to_string(),
            ExprKind::List(items) => {
                format!("[{}]", items.iter().map(show).collect::<Vec<_>>().join(" "))
            }
            ExprKind::Unary {
                op: UnaryOp::Neg,
                operand,
            } => format!("(- {})", show(operand)),
            ExprKind::Unary {
                op: UnaryOp::Not,
                operand,
            } => format!("(not {})", show(operand)),
            ExprKind::Binary { op, left, right } => {
                format!("({} {} {})", op.symbol(), show(left), show(right))
            }
            ExprKind::Logical {
                op: LogicalOp::And,
                left,
                right,
            } => format!("(and {} {})", show(left), show(right)),
            ExprKind::Logical {
                op: LogicalOp::Or,
                left,
                right,
            } => format!("(or {} {})", show(left), show(right)),
            ExprKind::Call { callee, args } => {
                format!(
                    "(call {}{})",
                    show(callee),
                    args.iter()
                        .map(|a| format!(" {}", show(a)))
                        .collect::<String>()
                )
            }
            ExprKind::Index { target, index } => {
                format!("(index {} {})", show(target), show(index))
            }
            ExprKind::If { .. } => "(if)".into(),
            ExprKind::Block(_) => "(block)".into(),
            ExprKind::Fn(_) => "(fn)".into(),
        }
    }

    fn tree(text: &str) -> String {
        let program =
            parse(text, 0).unwrap_or_else(|e| panic!("{text:?} does not parse: {}", e.message));
        show(program.tail.as_deref().expect("the text is one expression"))
    }

    fn error(text: &str) -> (String, usize, usize) {
        error_at(text, 0)
    }

    fn error_at(text: &str, start: usize) -> (String, usize, usize) {
        let e = parse(text, start).expect_err("the text must not parse");
        (e.message, e.span.start as usize, e.span.end as usize)
    }

    #[test]
    fn binary_operators_group_by_power_and_to_the_left() {
        assert_eq!(tree("1 + 2 * 3 - 4"), "(- (+ 1 (* 2 3)) 4)");
        assert_eq!(tree("8 / 4 / 2 % 3"), "(% (/ (/ 8 4) 2) 3)");
        assert_eq!(tree("a or b and c or d"), "(or (or a (and b c)) d)");
        assert_eq!(tree("1 + 2 < 3 * 4 and x"), "(and (< (+ 1 2) (* 3 4)) x)");
    }

    #[test]
    fn prefix_operators_take_what_their_power_allows() {
        assert_eq!(tree("-a * b"), "(* (- a) b)");
        assert_eq!(tree("- -a"), "(- (- a))");
        assert_eq!(tree("-f(x)[0]"), "(- (index (call f x) 0))");
        assert_eq!(tree("not a == b"), "(not (== a b))");
        assert_eq!(tree("not a and not not b"), "(and (not a) (not (not b)))");
    }

    #[test]
    fn postfix_operators_chain() {
        assert_eq!(
            tree("f(1)(2, 3,)[4]()"),
            "(call (index (call (call f 1) 2 3) 4))"
        );
        assert_eq!(tree("(a + b)(c)"), "(call (+ a b) c)");
    }

    #[test]
    fn not_cannot_be_an_arithmetic_or_comparison_operand() {
        assert_eq!(error("1 + not x").0, "expected expression, found `not`");
        assert_eq!(error("a == not b").0, "expected expression, found `not`");
        assert_eq!(error("-not b").0, "expected expression, found `not`");
    }

    #[test]
    fn comparisons_do_not_chain() {
        assert_eq!(
            error("a < b < c"),
            ("comparison operators cannot be chained".into(), 6, 7)
        );
        assert_eq!(error("a == b != c").1, 7);
        assert_eq!(tree("(a < b) == (c < d)"), "(== (< a b) (< c d))");
    }

    #[test]
    fn spans_include_operand_parentheses_but_not_their_own() {
        let program = parse("((1 + 2) * (3))", 0).unwrap();
        let product = program.tail.unwrap();
        assert_eq!((product.span.start, product.span.end), (1, 14));
        let ExprKind::Binary { left, right, .. } = &product.kind else {
            panic!("not a product")
        };
        assert_eq!((left.span.start, left.span.end), (2, 7));
        assert_eq!((right.span.start, right.span.end), (12, 13));
    }

    #[test]
    fn statements_and_tails() {
        let program = parse("let a = 1; a; { a }\n if a { } a", 0).unwrap();
        assert_eq!(program.stmts.len(), 4);
        assert!(program.tail.is_some());
        let program = parse("f();", 0).unwrap();
        assert!(program.tail.is_none());
        // A block-like statement is not continued by an operator.
        assert_eq!(
            error("if a { 1 } else { 2 } + 3;").0,
            "expected expression, found `+`"
        );
    }

    #[test]
    fn statement_ends() {
        assert!(parse("{ let a = 1 }", 0).is_ok());
        assert!(parse("fn f() { return }", 0).is_ok());
        assert_eq!(error("let a = 1 let b = 2").0, "expected `;`, found `let`");
        assert_eq!(error("{ f() ").0, "expected `}`, found end of input");
        assert_eq!(error("f() }").0, "expected expression, found `}`");
    }

    #[test]
    fn assignment_targets() {
        assert!(parse("a = 1; a[0][1] = 2; (a) = 3;", 0).is_ok());
        assert_eq!(
            error("a + 1 = 2;"),
            ("invalid assignment target".into(), 0, 5)
        );
        assert_eq!(error("a = b = c;").0, "expected `;`, found `=`");
    }

    #[test]
    fn a_lexical_error_is_reported_when_the_parser_reaches_it() {
        assert_eq!(error("let x = @;").0, "unexpected character '@'");
        // The syntax error comes first in the text, so it wins.
        assert_eq!(error("let = 1; @").0, "expected a name, found `=`");
    }

    #[test]
    fn parsing_from_an_offset_keeps_absolute_spans() {
        let text = "first entry @@@\nx + 1";
        let program = parse(text, 16).unwrap();
        let sum = program.tail.unwrap();
        assert_eq!((sum.span.start, sum.span.end), (16, 21));
        assert_eq!(
            error_at("old\n1 +\n", 4),
            ("expected expression, found end of input".into(), 8, 8)
        );
    }

    #[test]
    fn the_delimiter_limit_is_exact() {
        let nested = |n: usize| format!("{}1{}", "(".repeat(n), ")".repeat(n));
        assert!(parse(&nested(MAX_OPEN_DELIMITERS), 0).is_ok());
        assert_eq!(
            error(&nested(MAX_OPEN_DELIMITERS + 1)),
            ("nesting too deep".into(), 100, 101)
        );
    }

    #[test]
    fn nesting_without_delimiters_is_bounded_too() {
        // A recursive parser needs stack in proportion to the nesting; an
        // unoptimised build needs more than a test thread has.
        let checked = crate::with_large_stack(|| {
            assert!(parse(&format!("{}1", "-".repeat(MAX_PREFIX_NESTING)), 0).is_ok());
            assert!(parse(&format!("1{}", " + 1".repeat(MAX_CHAIN)), 0).is_ok());
            assert_eq!(
                error(&format!("{}1", "-".repeat(100_000))).0,
                "nesting too deep"
            );
            assert_eq!(
                error(&format!(
                    "{}x {}",
                    "if ".repeat(100_000),
                    "{} ".repeat(100_000)
                ))
                .0,
                "nesting too deep"
            );
            assert_eq!(
                error(&format!("1{}", " + 1".repeat(100_000))).0,
                "nesting too deep"
            );
            assert_eq!(
                error(&format!("f{}", "()".repeat(100_000))).0,
                "nesting too deep"
            );
        });
        checked.expect("the thread starts");
    }
}
