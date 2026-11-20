//! Milestone 4: functions and closures.
//!
//! SPEC sections 7.6 (calls), 8.5 (`return`), 9 (functions and closures),
//! 6.2 (a name belongs to its whole block) and 13 (the call depth limit).

mod common;

use common::{diagnostic, expect_compile_error, expect_output, expect_runtime_error};

// ----- declaring and calling ----------------------------------------------

#[test]
fn a_function_returns_the_value_of_its_tail() {
    expect_output(
        "fn add(a, b) { a + b }\nprint(add(2, 3), add(add(1, 1), 40));",
        "5 42\n",
    );
}

#[test]
fn return_ends_the_call_with_a_value() {
    let script = "
        fn sign(n) {
            if n < 0 { return \"negative\"; }
            if n == 0 { return \"zero\" }
            \"positive\"
        }
        print(sign(-5), sign(0), sign(5));";
    expect_output(script, "negative zero positive\n");
}

#[test]
fn a_function_without_a_tail_or_with_a_bare_return_gives_nil() {
    expect_output(
        "fn a() { }\nfn b() { 1; }\nfn c() { return; }\nfn d() { return }\nprint(a(), b(), c(), d());",
        "nil nil nil nil\n",
    );
}

#[test]
fn return_leaves_nested_blocks_and_loops() {
    let script = "
        fn first_multiple(of, above) {
            var n = above;
            while true {
                n = n + 1;
                if n % of == 0 { { return n; } }
            }
        }
        print(first_multiple(7, 50));";
    expect_output(script, "56\n");
}

#[test]
fn a_function_body_runs_at_each_call() {
    expect_output(
        "fn hello() { print(\"hello\"); }\nhello();\nhello();",
        "hello\nhello\n",
    );
}

#[test]
fn trailing_commas_are_allowed_in_parameters_and_arguments() {
    expect_output("fn sub(a, b,) { a - b }\nprint(sub(5, 3,));", "2\n");
}

#[test]
fn parameters_shadow_outer_names() {
    expect_output(
        "let x = \"outer\";\nfn show(x) { print(x); }\nshow(\"inner\");\nprint(x);",
        "inner\nouter\n",
    );
}

#[test]
fn recursion() {
    let script = "
        fn fact(n) { if n <= 1 { 1 } else { n * fact(n - 1) } }
        fn fib(n) { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        print(fact(10), fact(20), fib(20));";
    expect_output(script, "3628800 2432902008176640000 6765\n");
}

// ----- functions are values -----------------------------------------------

#[test]
fn functions_can_be_stored_passed_and_returned() {
    let script = "
        fn double(x) { x * 2 }
        fn apply(f, x) { f(x) }
        fn pick(flag) { if flag { double } else { print } }
        let d = double;
        print(d(4), apply(double, 5), apply(d, 6), pick(true)(7));
        pick(false)(\"printed by a returned function\");";
    expect_output(script, "8 10 12 14\nprinted by a returned function\n");
}

#[test]
fn a_function_literal_has_no_name() {
    let script = "
        let square = fn(x) { x * x };
        print(square(9));
        print(fn(a, b) { a + b }(1, 2));
        fn apply(f, x) { f(x) }
        print(apply(fn(s) { s + \"!\" }, \"hey\"));";
    expect_output(script, "81\n3\nhey!\n");
}

#[test]
fn a_statement_can_start_with_a_function_literal() {
    // `fn` followed by `(` starts an expression, here one that is called at
    // once. `fn` followed by a name declares.
    expect_output(
        "fn() { print(\"called\"); }();\nfn(x) { print(x); }(7);",
        "called\n7\n",
    );
}

#[test]
fn how_functions_are_printed() {
    let script = "
        fn named() { }
        let anonymous = fn() { };
        { fn inner() { } print(named, anonymous, inner, print); }";
    expect_output(script, "<fn named> <fn> <fn inner> <fn print>\n");
}

#[test]
fn a_run_of_250_calls_is_accepted() {
    // SPEC 13.2. Each call returns the function itself, so only one call is
    // active at a time.
    let script = format!("fn me() {{ me }}\nprint(me{});", "()".repeat(250));
    expect_output(&script, "<fn me>\n");
}

#[test]
fn a_function_equals_only_itself() {
    let script = "
        fn f() { }
        fn g() { }
        fn make() { fn() { } }
        let same = f;
        print(f == f, f == same, f == g, make() == make(), print == print, print == f, f != g);";
    expect_output(script, "true true false false true false true\n");
}

// ----- closures ------------------------------------------------------------

#[test]
fn a_closure_keeps_the_bindings_of_the_place_it_was_made() {
    let script = "
        fn make_adder(n) { fn(x) { x + n } }
        let add2 = make_adder(2);
        let add10 = make_adder(10);
        print(add2(1), add10(1), add2(add10(0)));";
    expect_output(script, "3 11 12\n");
}

#[test]
fn a_closure_shares_the_binding_and_does_not_copy_the_value() {
    let script = "
        fn make_counter() {
            var count = 0;
            fn() { count = count + 1; count }
        }
        let a = make_counter();
        let b = make_counter();
        a(); a();
        print(a(), b(), a());";
    expect_output(script, "3 1 4\n");
}

#[test]
fn two_closures_from_one_call_share_one_binding() {
    let script = "
        var get = nil;
        var set = nil;
        {
            var hidden = \"first\";
            get = fn() { hidden };
            set = fn(v) { hidden = v; };
        }
        print(get());
        set(\"second\");
        print(get());";
    expect_output(script, "first\nsecond\n");
}

#[test]
fn a_closure_sees_later_assignments_to_an_outer_binding() {
    expect_output(
        "var mood = \"calm\";\nfn show() { print(mood); }\nshow();\nmood = \"busy\";\nshow();",
        "calm\nbusy\n",
    );
}

#[test]
fn a_closure_is_resolved_where_it_is_written_not_where_it_is_called() {
    // `speak` was written where `word` means the top-level binding. A
    // binding with the same name at the place of the call changes nothing.
    let script = "
        let word = \"top\";
        fn speak() { print(word); }
        fn other() {
            let word = \"local to other\";
            speak();
        }
        other();
        { let word = \"in a block\"; speak(); }";
    expect_output(script, "top\ntop\n");
}

#[test]
fn each_entry_into_a_block_gives_closures_a_new_binding() {
    let script = "
        var first = nil;
        var second = nil;
        var i = 0;
        while i < 2 {
            let captured = i * 10;
            let f = fn() { captured };
            if i == 0 { first = f; } else { second = f; }
            i = i + 1;
        }
        print(first(), second());";
    expect_output(script, "0 10\n");
}

#[test]
fn closures_nest() {
    let script = "
        fn outer(a) {
            fn(b) {
                fn(c) { a + b + c }
            }
        }
        print(outer(1)(20)(300));";
    expect_output(script, "321\n");
}

#[test]
fn a_parameter_can_be_captured() {
    let script = "
        fn compose(f, g) { fn(x) { f(g(x)) } }
        fn inc(x) { x + 1 }
        fn twice(x) { x * 2 }
        print(compose(inc, twice)(5), compose(twice, inc)(5));";
    expect_output(script, "11 12\n");
}

// ----- a name belongs to its whole block -----------------------------------

#[test]
fn functions_can_call_functions_declared_later_in_the_same_block() {
    let script = "
        fn is_even(n) { if n == 0 { true } else { is_odd(n - 1) } }
        fn is_odd(n) { if n == 0 { false } else { is_even(n - 1) } }
        print(is_even(10), is_odd(10), is_even(7));
        {
            fn ping(n) { if n == 0 { \"ping\" } else { pong(n - 1) } }
            fn pong(n) { if n == 0 { \"pong\" } else { ping(n - 1) } }
            print(ping(3), ping(4));
        }";
    expect_output(script, "true false false\npong ping\n");
}

#[test]
fn a_function_can_use_a_binding_declared_after_it_once_that_has_run() {
    expect_output(
        "fn show() { print(late); }\nlet late = \"here now\";\nshow();",
        "here now\n",
    );
}

#[test]
fn calling_a_function_before_its_declaration_has_run_is_an_error() {
    expect_runtime_error(
        "print(\"start\");\nlater();\nfn later() { }",
        "start\n",
        &diagnostic(
            2,
            1,
            "`later` is used before it is initialized",
            "later();",
            5,
        ),
    );
    expect_runtime_error(
        "fn show() { print(late); }\nshow();\nlet late = 1;",
        "",
        &diagnostic(
            1,
            19,
            "`late` is used before it is initialized",
            "fn show() { print(late); }",
            4,
        ),
    );
}

#[test]
fn a_function_is_not_known_outside_its_block() {
    expect_compile_error(
        "{ fn hidden() { } }\nhidden();",
        &diagnostic(2, 1, "unknown name `hidden`", "hidden();", 6),
    );
    expect_compile_error(
        "fn f(p) { }\nprint(p);",
        &diagnostic(2, 7, "unknown name `p`", "print(p);", 1),
    );
}

// ----- checks before the script runs ---------------------------------------

#[test]
fn parameters_and_function_names_cannot_be_assigned() {
    expect_compile_error(
        "fn f(a) { a = 1; }",
        &diagnostic(
            1,
            11,
            "cannot assign to `a`: it is not declared with `var`",
            "fn f(a) { a = 1; }",
            1,
        ),
    );
    expect_compile_error(
        "fn f() { }\nf = 1;",
        &diagnostic(
            2,
            1,
            "cannot assign to `f`: it is not declared with `var`",
            "f = 1;",
            1,
        ),
    );
}

#[test]
fn parameters_belong_to_the_block_of_the_body() {
    expect_compile_error(
        "fn f(a, b, a) { }",
        &diagnostic(
            1,
            12,
            "`a` is already declared in this block",
            "fn f(a, b, a) { }",
            1,
        ),
    );
    expect_compile_error(
        "fn f(a) { let a = 1; }",
        &diagnostic(
            1,
            15,
            "`a` is already declared in this block",
            "fn f(a) { let a = 1; }",
            1,
        ),
    );
    expect_compile_error(
        "fn f() { }\nlet f = 1;",
        &diagnostic(
            2,
            5,
            "`f` is already declared in this block",
            "let f = 1;",
            1,
        ),
    );
    // A nested block is a new scope.
    expect_output(
        "fn g(a) { if true { var a = 5; a = a + 1; a } }\nprint(g(1));",
        "6\n",
    );
}

#[test]
fn return_needs_a_function() {
    expect_compile_error(
        "print(1);\nreturn 5;",
        &diagnostic(2, 1, "`return` outside of a function", "return 5;", 6),
    );
    expect_compile_error(
        "while true { return; }",
        &diagnostic(
            1,
            14,
            "`return` outside of a function",
            "while true { return; }",
            6,
        ),
    );
}

#[test]
fn a_loop_outside_a_function_is_not_a_loop_inside_it() {
    expect_compile_error(
        "while true { fn f() { break; } }",
        &diagnostic(
            1,
            23,
            "`break` outside of a loop",
            "while true { fn f() { break; } }",
            5,
        ),
    );
    expect_compile_error(
        "while true { let f = fn() { continue; }; }",
        &diagnostic(
            1,
            29,
            "`continue` outside of a loop",
            "while true { let f = fn() { continue; }; }",
            8,
        ),
    );
    // The other way round is fine: a loop inside the function.
    expect_output(
        "fn f() { while true { break; } \"ok\" }\nprint(f());",
        "ok\n",
    );
}

#[test]
fn syntax_errors_in_functions() {
    expect_compile_error(
        "fn f(a b) { }",
        &diagnostic(1, 8, "expected `,` or `)`, found `b`", "fn f(a b) { }", 1),
    );
    expect_compile_error(
        "fn f(1) { }",
        &diagnostic(1, 6, "expected a name, found `1`", "fn f(1) { }", 1),
    );
    expect_compile_error(
        "fn f(a,,) { }",
        &diagnostic(1, 8, "expected a name, found `,`", "fn f(a,,) { }", 1),
    );
    expect_compile_error(
        "fn f { }",
        &diagnostic(1, 6, "expected `(`, found `{`", "fn f { }", 1),
    );
    expect_compile_error(
        "fn f() print(1);",
        &diagnostic(1, 8, "expected `{`, found `print`", "fn f() print(1);", 5),
    );
    // In an expression, `fn` cannot be followed by a name.
    expect_compile_error(
        "let f = fn g() { };",
        &diagnostic(1, 12, "expected `(`, found `g`", "let f = fn g() { };", 1),
    );
    expect_compile_error(
        "fn 5() { }",
        &diagnostic(1, 4, "expected `(`, found `5`", "fn 5() { }", 1),
    );
    expect_compile_error(
        "print(1,, 2);",
        &diagnostic(1, 9, "expected expression, found `,`", "print(1,, 2);", 1),
    );
}

// ----- calls ---------------------------------------------------------------

#[test]
fn the_number_of_arguments_must_match() {
    expect_runtime_error(
        "fn pair(a, b) { }\npair(1);",
        "",
        &diagnostic(2, 1, "expected 2 arguments, found 1", "pair(1);", 7),
    );
    expect_runtime_error(
        "fn one(a) { }\nprint(one());",
        "",
        &diagnostic(2, 7, "expected 1 argument, found 0", "print(one());", 5),
    );
    expect_runtime_error(
        "fn none() { }\nnone(1, 2);",
        "",
        &diagnostic(2, 1, "expected 0 arguments, found 2", "none(1, 2);", 10),
    );
    expect_runtime_error(
        "fn(a) { }();",
        "",
        &diagnostic(1, 1, "expected 1 argument, found 0", "fn(a) { }();", 11),
    );
}

#[test]
fn only_functions_can_be_called() {
    expect_runtime_error(
        "let x = 3;\nx(1);",
        "",
        &diagnostic(2, 1, "cannot call int", "x(1);", 1),
    );
    expect_runtime_error(
        "\"text\"();",
        "",
        &diagnostic(1, 1, "cannot call str", "\"text\"();", 6),
    );
    expect_runtime_error(
        "fn f() { }\nf()();",
        "",
        &diagnostic(2, 1, "cannot call nil", "f()();", 3),
    );
    expect_runtime_error(
        "(1 + 2)(3);",
        "",
        &diagnostic(1, 2, "cannot call int", "(1 + 2)(3);", 5),
    );
}

#[test]
fn the_callee_then_the_arguments_are_evaluated_before_any_check() {
    let script = "
        var log = \"\";
        fn note(s, v) { log = log + s; v }
        fn three(a, b, c) { a + b + c }
        print(note(\"f\", three)(note(\"1\", 1), note(\"2\", 2), note(\"3\", 3)), log);";
    expect_output(script, "6 f123\n");

    // The arguments are evaluated although `3` cannot be called ...
    expect_runtime_error(
        "3(1 / 0);",
        "",
        &diagnostic(1, 3, "division by zero", "3(1 / 0);", 5),
    );
    // ... and although there are too many of them.
    expect_runtime_error(
        "fn f() { }\nf(1 / 0);",
        "",
        &diagnostic(2, 3, "division by zero", "f(1 / 0);", 5),
    );
    // "Can it be called?" is asked before "how many arguments?".
    expect_runtime_error(
        "nil(1, 2);",
        "",
        &diagnostic(1, 1, "cannot call nil", "nil(1, 2);", 3),
    );
}

#[test]
fn an_error_inside_a_function_points_into_the_function() {
    expect_runtime_error(
        "fn half(n) {\n  n / 0\n}\nprint(\"before\");\nprint(half(4));",
        "before\n",
        &diagnostic(2, 3, "division by zero", "  n / 0", 5),
    );
}

// ----- the call depth limit ------------------------------------------------

const DOWN: &str = "fn down(n) { if n == 0 { \"bottom\" } else { down(n - 1) } }\n";

#[test]
fn one_thousand_active_calls_are_allowed() {
    // down(999) calls down(998) ... down(0): 1000 calls active at the end.
    expect_output(&format!("{DOWN}print(down(999));"), "bottom\n");
}

#[test]
fn the_call_that_would_be_number_1001_fails() {
    expect_runtime_error(
        &format!("{DOWN}print(\"start\");\nprint(down(1000));"),
        "start\n",
        &diagnostic(1, 44, "stack overflow", DOWN.trim_end(), 11),
    );
}

#[test]
fn endless_recursion_is_an_error_and_not_a_crash() {
    expect_runtime_error(
        "fn forever(n) {\n  forever(n + 1)\n}\nforever(0);",
        "",
        &diagnostic(2, 3, "stack overflow", "  forever(n + 1)", 14),
    );
}

#[test]
fn calls_that_have_returned_do_not_count() {
    let script = "
        fn id(x) { x }
        var i = 0;
        var sum = 0;
        while i < 5000 { sum = sum + id(i); i = i + 1; }
        print(sum);";
    expect_output(script, "12497500\n");
}

#[test]
fn built_in_functions_do_not_count_as_calls() {
    // With 1000 calls active, `print` can still be called.
    let script =
        "fn down(n) { if n == 0 { print(\"at the bottom\"); 0 } else { down(n - 1) } }\ndown(999);";
    expect_output(script, "at the bottom\n");
}

#[test]
fn deep_recursion_can_be_used_again_after_it_has_returned() {
    expect_output(
        &format!("{DOWN}print(down(900));\nprint(down(900));\nprint(down(999));"),
        "bottom\nbottom\nbottom\n",
    );
}
