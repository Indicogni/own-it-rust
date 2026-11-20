//! One interpreter with its state: parse, resolve, evaluate.
//!
//! A script is one [`Session::run`]. The REPL calls `run` once per entry on
//! the same session, which is what keeps the top-level bindings alive.

use std::io::{self, BufRead, Write};

use crate::interp::{Interp, Unwind};
use crate::parser;
use crate::resolve::Resolver;
use crate::source::Diagnostic;
use crate::value::Value;

/// Why a run did not produce a value. The variants map one to one to the
/// exit statuses of SPEC 2.2.
#[derive(Debug)]
pub enum Failure {
    /// A syntax error or a name error: nothing was evaluated.
    Compile(Diagnostic),
    Runtime(Diagnostic),
    /// The script called `exit`.
    Exit(u8),
    /// Standard output could not be written.
    Output(io::Error),
}

pub struct Session<'io> {
    resolver: Resolver,
    interp: Interp<'io>,
}

impl<'io> Session<'io> {
    /// `out` receives what the program prints; `input` is what `input()`
    /// reads.
    pub fn new(out: &'io mut dyn Write, input: &'io mut dyn BufRead) -> Session<'io> {
        Session {
            resolver: Resolver::new(),
            interp: Interp::new(out, input),
        }
    }

    /// Runs `text` at the top level and returns the value of its tail
    /// expression (`nil` if it has none).
    pub fn run(&mut self, text: &str) -> Result<Value, Failure> {
        self.run_from(text, 0)
    }

    /// Runs the part of `text` that starts at byte offset `start`. Spans in
    /// errors are offsets into the whole of `text`.
    ///
    /// The REPL appends each entry to one text and runs the new part. A
    /// function from an earlier entry can fail while a later entry runs,
    /// and its spans then still point into the right text.
    pub fn run_from(&mut self, text: &str, start: usize) -> Result<Value, Failure> {
        let mut program = parser::parse(text, start).map_err(Failure::Compile)?;
        self.resolver
            .resolve(&mut program)
            .map_err(Failure::Compile)?;
        match self.interp.run(&program) {
            Ok(value) => Ok(value),
            Err(Unwind::Error(diagnostic)) => Err(Failure::Runtime(diagnostic)),
            Err(Unwind::Exit(code)) => Err(Failure::Exit(code)),
            Err(Unwind::Output(error)) => Err(Failure::Output(error)),
            // The resolver rejects `return`, `break` and `continue` that
            // have nothing to return from or to leave, so these do not
            // reach the top level. If one did, ending the program quietly
            // is the least surprising thing to do.
            Err(Unwind::Return(value)) => Ok(value),
            Err(Unwind::Break | Unwind::Continue) => Ok(Value::Nil),
        }
    }

    /// The program's output stream, for the driver's own writes.
    pub fn out(&mut self) -> &mut (dyn Write + 'io) {
        self.interp.out()
    }

    /// The next line of the program's input (see [`Interp::read_line`]).
    /// The REPL reads its entries here, so that `input()` and the REPL
    /// share one buffered reader and no line is lost between them.
    pub fn read_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.interp.read_line()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `text`; returns what it printed and how it ended.
    fn run_with_input(text: &str, input: &str) -> (String, Result<Value, Failure>) {
        let mut out = Vec::new();
        let mut input = input.as_bytes();
        let result = Session::new(&mut out, &mut input).run(text);
        (
            String::from_utf8(out).expect("the interpreter writes UTF-8"),
            result,
        )
    }

    fn output(text: &str) -> String {
        let (out, result) = run_with_input(text, "");
        if let Err(failure) = result {
            panic!("{text:?} failed: {failure:?}");
        }
        out
    }

    fn runtime_error(text: &str) -> String {
        match run_with_input(text, "").1 {
            Err(Failure::Runtime(d)) => d.message,
            other => panic!("{text:?}: expected a runtime error, got {other:?}"),
        }
    }

    fn compile_error(text: &str) -> String {
        match run_with_input(text, "").1 {
            Err(Failure::Compile(d)) => d.message,
            other => panic!("{text:?}: expected a compile error, got {other:?}"),
        }
    }

    impl std::fmt::Debug for Value {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.quoted())
        }
    }

    #[test]
    fn closures_share_the_binding_they_capture() {
        let text = "
            fn counter() {
                var n = 0;
                [fn() { n = n + 1; n }, fn() { n }]
            }
            let pair = counter();
            pair[0](); pair[0]();
            print(pair[1](), counter()[1]());
        ";
        assert_eq!(output(text), "2 0\n");
    }

    #[test]
    fn addresses_skip_blocks_without_bindings() {
        // The inner blocks declare nothing and get no environment; the
        // resolver must not count them when it measures the distance to `a`.
        let text = "
            let a = 1;
            fn f(x) { if x > 0 { { { a + x } } } else { 0 } }
            { let b = 10; { print(f(b), a); } }
        ";
        assert_eq!(output(text), "11 1\n");
    }

    #[test]
    fn a_name_belongs_to_its_whole_block() {
        assert_eq!(
            runtime_error("let a = 1; { print(a); let a = 2; }"),
            "`a` is used before it is initialized"
        );
        assert_eq!(
            output(
                "fn even(n) { n == 0 or odd(n - 1) } fn odd(n) { n != 0 and even(n - 1) } print(even(10));"
            ),
            "true\n"
        );
    }

    #[test]
    fn the_first_check_error_in_source_order_is_reported() {
        assert_eq!(
            compile_error("{ print(zz); let a = 1; let a = 2; }"),
            "unknown name `zz`"
        );
        assert_eq!(
            compile_error("{ let a = 1; let a = 2; print(zz); }"),
            "`a` is already declared in this block"
        );
    }

    #[test]
    fn a_failed_entry_leaves_no_names_behind() {
        let mut out = Vec::new();
        let mut input = "".as_bytes();
        let mut session = Session::new(&mut out, &mut input);
        assert!(matches!(
            session.run("let a = 1; let b = nope;"),
            Err(Failure::Compile(_))
        ));
        assert!(matches!(session.run("a"), Err(Failure::Compile(_))));
        assert!(session.run("let a = 2; let b = 3;").is_ok());
        // Declaring a top-level name again in a later entry is allowed.
        assert!(matches!(
            session.run("var a = a + b; a = a * 2; a"),
            Ok(Value::Int(10))
        ));
    }

    #[test]
    fn input_reads_lines_and_then_nil() {
        let (out, result) = run_with_input("print(input(), input(), input());", "one\r\ntwo");
        assert!(result.is_ok());
        assert_eq!(out, "one two nil\n");
    }

    #[test]
    fn exit_carries_its_code() {
        let (out, result) = run_with_input("print(1); exit(7); print(2);", "");
        assert_eq!(out, "1\n");
        assert!(matches!(result, Err(Failure::Exit(7))));
    }

    #[test]
    fn deep_recursion_is_an_error_not_a_crash() {
        // The evaluator needs the large stack that `main` gives it.
        let checked = crate::with_large_stack(|| {
            assert_eq!(
                output("fn down(n) { if n == 0 { 0 } else { down(n - 1) } } print(down(999));"),
                "0\n"
            );
            assert_eq!(
                runtime_error("fn f(n) { f(n + 1) } f(0);"),
                "stack overflow"
            );
            // The worst nesting the parser lets through, in every call:
            // `MAX_EVAL_DEPTH` must stop this before the stack runs out.
            let nested = format!("{}f(n + 1){}", "g(".repeat(90), ")".repeat(90));
            let deep = format!(
                "fn g(x) {{ x }} fn f(n) {{ {}{nested} }} f(0);",
                "- ".repeat(900)
            );
            assert_eq!(runtime_error(&deep), "stack overflow");
            // A tree as deep as the parser allows is resolved and freed
            // without trouble, even if evaluating it is refused.
            let group = format!("{})", " + 1".repeat(999));
            let chain = format!("{}0{}", "(".repeat(99), group.repeat(99));
            assert_eq!(runtime_error(&format!("print({chain});")), "stack overflow");
        });
        checked.expect("the thread starts");
    }
}
