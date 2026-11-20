//! Milestone 5: lists, strings as sequences, `for`, and the built-in
//! functions that work on them.
//!
//! SPEC sections 5 (values, display and quoted form), 7.4 (equality), 7.9
//! (indexing), 8.1 (assigning to an element), 8.4 (`for`) and 10 (built-in
//! functions, except `input` and `exit`).

mod common;

use common::{diagnostic, expect_compile_error, expect_output, expect_runtime_error};

// ----- list values ---------------------------------------------------------

#[test]
fn list_literals() {
    expect_output(
        "print([1, 2, 3], [], [1 + 1, 2 * 2,], [[1], [2, [3]]]);",
        "[1, 2, 3] [] [2, 4] [[1], [2, [3]]]\n",
    );
}

#[test]
fn inside_a_list_strings_are_quoted() {
    expect_output(
        r#"print(["a", 1, true, nil], "a");"#,
        "[\"a\", 1, true, nil] a\n",
    );
    expect_output(
        r#"print(["say \"hi\"", "back\\slash", "new\nline", "tab\t", "cr\r", "nul\0"]);"#,
        "[\"say \\\"hi\\\"\", \"back\\\\slash\", \"new\\nline\", \"tab\\t\", \"cr\\r\", \"nul\\0\"]\n",
    );
    expect_output("print([\"żółw\", \"\"]);", "[\"żółw\", \"\"]\n");
}

#[test]
fn functions_inside_a_list() {
    expect_output(
        "fn f() { }\nprint([f, fn() { }, print]);",
        "[<fn f>, <fn>, <fn print>]\n",
    );
}

#[test]
fn elements_are_evaluated_from_left_to_right() {
    expect_output(
        "var n = 0;\nfn next() { n = n + 1; n }\nprint([next(), next(), next()]);",
        "[1, 2, 3]\n",
    );
}

// ----- indexing ------------------------------------------------------------

#[test]
fn indexing_a_list() {
    expect_output(
        "let xs = [10, 20, 30];\nprint(xs[0], xs[2], xs[1 + 0], [5, 6][1]);",
        "10 30 20 6\n",
    );
    expect_output(
        "let grid = [[1, 2], [3, 4]];\nprint(grid[1][0], grid[0], grid[1][1] * 10);",
        "3 [1, 2] 40\n",
    );
}

#[test]
fn indexing_a_string_gives_a_string_of_one_character() {
    expect_output(
        "let s = \"tansy\";\nprint(s[0], s[4], s[1] + s[2], type(s[0]), len(s[0]));",
        "t y an str 1\n",
    );
    // Characters, not bytes.
    expect_output(
        "let s = \"żółw🐢\";\nprint(s[0], s[3], s[4], len(s));",
        "ż w 🐢 5\n",
    );
}

#[test]
fn a_run_of_250_index_operations_is_accepted() {
    // SPEC 13.2. The list contains itself, so every `[0]` gives the list.
    let script = format!(
        "let a = [];\npush(a, a);\nprint(len(a{}));",
        "[0]".repeat(250)
    );
    expect_output(&script, "1\n");
}

#[test]
fn an_index_out_of_range() {
    expect_runtime_error(
        "let xs = [1, 2, 3];\nprint(xs[3]);",
        "",
        &diagnostic(
            2,
            7,
            "index 3 is out of range for length 3",
            "print(xs[3]);",
            5,
        ),
    );
    expect_runtime_error(
        "let xs = [1];\nxs[-1];",
        "",
        &diagnostic(2, 1, "index -1 is out of range for length 1", "xs[-1];", 6),
    );
    expect_runtime_error(
        "[][0];",
        "",
        &diagnostic(1, 1, "index 0 is out of range for length 0", "[][0];", 5),
    );
    expect_runtime_error(
        "\"abc\"[5];",
        "",
        &diagnostic(
            1,
            1,
            "index 5 is out of range for length 3",
            "\"abc\"[5];",
            8,
        ),
    );
    expect_runtime_error(
        "\"żółw\"[4];",
        "",
        &diagnostic(
            1,
            1,
            "index 4 is out of range for length 4",
            "\"żółw\"[4];",
            9,
        ),
    );
}

#[test]
fn what_cannot_be_indexed_and_what_cannot_be_an_index() {
    expect_runtime_error(
        "let n = 5;\nn[0];",
        "",
        &diagnostic(2, 1, "cannot index int", "n[0];", 1),
    );
    expect_runtime_error(
        "print(nil[0]);",
        "",
        &diagnostic(1, 7, "cannot index nil", "print(nil[0]);", 3),
    );
    expect_runtime_error(
        "[1, 2][\"0\"];",
        "",
        &diagnostic(1, 8, "expected int, found str", "[1, 2][\"0\"];", 3),
    );
    expect_runtime_error(
        "\"abc\"[true];",
        "",
        &diagnostic(1, 7, "expected int, found bool", "\"abc\"[true];", 4),
    );
    // The thing that is indexed is checked first.
    expect_runtime_error(
        "5[\"x\"];",
        "",
        &diagnostic(1, 1, "cannot index int", "5[\"x\"];", 1),
    );
}

// ----- assigning to elements; sharing --------------------------------------

#[test]
fn assigning_to_an_element() {
    expect_output(
        "let xs = [1, 2, 3];\nxs[0] = \"one\";\nxs[2] = xs[1] * 10;\nprint(xs);",
        "[\"one\", 2, 20]\n",
    );
    expect_output(
        "let grid = [[0, 0], [0, 0]];\ngrid[1][0] = 7;\nprint(grid);",
        "[[0, 0], [7, 0]]\n",
    );
}

#[test]
fn a_list_is_shared_not_copied() {
    let script = "
        let a = [1, 2];
        let b = a;
        b[0] = 99;
        push(b, 3);
        print(a, b, a == b);
        fn clear_first(xs) { xs[0] = nil; }
        clear_first(a);
        print(b);";
    expect_output(script, "[99, 2, 3] [99, 2, 3] true\n[nil, 2, 3]\n");
}

#[test]
fn the_same_list_can_be_an_element_twice() {
    expect_output(
        "let row = [0];\nlet grid = [row, row];\ngrid[0][0] = 5;\nprint(grid);",
        "[[5], [5]]\n",
    );
}

#[test]
fn a_let_binding_can_hold_a_list_that_changes() {
    // `let` fixes which list the name refers to, not what is in the list.
    expect_output(
        "let xs = [];\npush(xs, 1);\nxs[0] = 2;\nprint(xs);",
        "[2]\n",
    );
}

#[test]
fn errors_when_assigning_to_an_element() {
    expect_runtime_error(
        "let xs = [1];\nxs[1] = 2;",
        "",
        &diagnostic(
            2,
            1,
            "index 1 is out of range for length 1",
            "xs[1] = 2;",
            5,
        ),
    );
    expect_runtime_error(
        "let s = \"abc\";\ns[0] = \"x\";",
        "",
        &diagnostic(2, 1, "cannot assign into str", "s[0] = \"x\";", 1),
    );
    expect_runtime_error(
        "var n = 1;\nn[0] = 2;",
        "",
        &diagnostic(2, 1, "cannot assign into int", "n[0] = 2;", 1),
    );
    expect_runtime_error(
        "let xs = [1];\nxs[nil] = 2;",
        "",
        &diagnostic(2, 4, "expected int, found nil", "xs[nil] = 2;", 3),
    );
}

#[test]
fn target_index_and_value_are_evaluated_in_that_order_before_any_check() {
    let script = "
        var log = \"\";
        fn note(s, v) { log = log + s; v }
        let xs = [0, 0];
        note(\"t\", xs)[note(\"i\", 1)] = note(\"v\", 5);
        print(xs, log);";
    expect_output(script, "[0, 5] tiv\n");
    // The value is evaluated although the target is no list.
    expect_runtime_error(
        "let n = 1;\nn[0] = 1 / 0;",
        "",
        &diagnostic(2, 8, "division by zero", "n[0] = 1 / 0;", 5),
    );
}

// ----- len, push, pop ------------------------------------------------------

#[test]
fn len_counts_elements_or_characters() {
    expect_output(
        "print(len([]), len([1, [2, 3]]), len(\"\"), len(\"abc\"), len(\"żółw 🐢\"));",
        "0 2 0 3 6\n",
    );
}

#[test]
fn push_adds_at_the_end_and_returns_nil() {
    expect_output(
        "let xs = [];\nprint(push(xs, 1));\npush(xs, \"two\");\npush(xs, [3]);\nprint(xs, len(xs));",
        "nil\n[1, \"two\", [3]] 3\n",
    );
}

#[test]
fn pop_removes_and_returns_the_last_element() {
    expect_output(
        "let xs = [1, 2, 3];\nprint(pop(xs), pop(xs), xs);\nprint(pop(xs), xs);",
        "3 2 [1]\n1 []\n",
    );
}

#[test]
fn pop_on_an_empty_list_is_an_error() {
    expect_runtime_error(
        "let xs = [1];\npop(xs);\nprint(pop(xs));",
        "",
        &diagnostic(3, 7, "pop: the list is empty", "print(pop(xs));", 7),
    );
}

#[test]
fn built_in_functions_check_the_type_of_their_argument() {
    expect_runtime_error(
        "len(5);",
        "",
        &diagnostic(1, 5, "len: expected str or list, found int", "len(5);", 1),
    );
    expect_runtime_error(
        "len(nil);",
        "",
        &diagnostic(1, 5, "len: expected str or list, found nil", "len(nil);", 3),
    );
    expect_runtime_error(
        "push(\"abc\", 1);",
        "",
        &diagnostic(
            1,
            6,
            "push: expected list, found str",
            "push(\"abc\", 1);",
            5,
        ),
    );
    expect_runtime_error(
        "pop(1 + 1);",
        "",
        &diagnostic(1, 5, "pop: expected list, found int", "pop(1 + 1);", 5),
    );
    expect_runtime_error(
        "int([1]);",
        "",
        &diagnostic(1, 5, "int: expected str or int, found list", "int([1]);", 3),
    );
    expect_runtime_error(
        "int(true);",
        "",
        &diagnostic(
            1,
            5,
            "int: expected str or int, found bool",
            "int(true);",
            4,
        ),
    );
}

#[test]
fn built_in_functions_check_the_number_of_arguments_first() {
    expect_runtime_error(
        "len();",
        "",
        &diagnostic(1, 1, "expected 1 argument, found 0", "len();", 5),
    );
    expect_runtime_error(
        "len(1, 2);",
        "",
        &diagnostic(1, 1, "expected 1 argument, found 2", "len(1, 2);", 9),
    );
    expect_runtime_error(
        "push([]);",
        "",
        &diagnostic(1, 1, "expected 2 arguments, found 1", "push([]);", 8),
    );
    expect_runtime_error(
        "print(str());",
        "",
        &diagnostic(1, 7, "expected 1 argument, found 0", "print(str());", 5),
    );
    expect_runtime_error(
        "type(1, 2, 3);",
        "",
        &diagnostic(1, 1, "expected 1 argument, found 3", "type(1, 2, 3);", 13),
    );
}

#[test]
fn built_in_functions_are_values() {
    let script = "
        let add = push;
        let xs = [];
        add(xs, 1);
        fn apply(f, x) { f(x) }
        print(xs, apply(len, \"four\"), [len, add], len == len, len == add);";
    expect_output(script, "[1] 4 [<fn len>, <fn push>] true false\n");
}

#[test]
fn a_top_level_function_can_replace_a_built_in_one() {
    expect_output(
        "fn len(x) { \"my len\" }\nprint(len([1, 2, 3]));",
        "my len\n",
    );
}

// ----- str, int, type ------------------------------------------------------

#[test]
fn str_gives_the_display_form() {
    expect_output(
        "print(str(12) + str(true) + str(nil) + str(\"s\") + str([1, \"a\"]) + str(print), len(str(-100)));",
        "12truenils[1, \"a\"]<fn print> 4\n",
    );
}

#[test]
fn int_parses_a_decimal_integer() {
    expect_output(
        "print(int(\"42\"), int(\"-7\"), int(\"007\"), int(\"-0\"), int(5), int(\"9223372036854775807\") - 1);",
        "42 -7 7 0 5 9223372036854775806\n",
    );
    expect_output(
        "print(int(\"-9223372036854775808\"));",
        "-9223372036854775808\n",
    );
}

#[test]
fn int_gives_nil_for_anything_else() {
    expect_output(
        "print(int(\"\"), int(\"abc\"), int(\"12a\"), int(\" 12\"), int(\"12 \"), int(\"+12\"), int(\"-\"), int(\"1.5\"), int(\"--1\"));",
        "nil nil nil nil nil nil nil nil nil\n",
    );
    expect_output(
        "print(int(\"9223372036854775808\"), int(\"1_000\"), int(\"0x10\"), int(\"١٢\"));",
        "nil nil nil nil\n",
    );
}

#[test]
fn type_names() {
    expect_output(
        "fn f() { }\nprint(type(nil), type(true), type(1), type(\"s\"), type([1]), type(f), type(print), type(fn() { }));",
        "nil bool int str list fn fn fn\n",
    );
    expect_output("print(type(type(1)), type(1) == \"int\");", "str true\n");
}

// ----- + and == on lists ---------------------------------------------------

#[test]
fn plus_makes_a_new_list() {
    let script = "
        let a = [1, 2];
        let b = [3];
        let c = a + b;
        push(a, \"changed\");
        print(c, a + a, [] + [], len(c + c));";
    expect_output(
        script,
        "[1, 2, 3] [1, 2, \"changed\", 1, 2, \"changed\"] [] 6\n",
    );
}

#[test]
fn plus_does_not_mix_types() {
    expect_runtime_error(
        "[1] + 1;",
        "",
        &diagnostic(1, 1, "cannot apply `+` to list and int", "[1] + 1;", 7),
    );
    expect_runtime_error(
        "\"a\" + [\"b\"];",
        "",
        &diagnostic(
            1,
            1,
            "cannot apply `+` to str and list",
            "\"a\" + [\"b\"];",
            11,
        ),
    );
    expect_runtime_error(
        "\"n = \" + 5;",
        "",
        &diagnostic(1, 1, "cannot apply `+` to str and int", "\"n = \" + 5;", 10),
    );
    expect_output("print(\"n = \" + str(5));", "n = 5\n");
}

#[test]
fn lists_are_equal_when_their_elements_are() {
    expect_output(
        "print([1, 2] == [1, 2], [1, 2] == [2, 1], [] == [], [1] == [1, 1], [[1], \"a\"] == [[1], \"a\"], [1] != [2]);",
        "true false true false true true\n",
    );
    expect_output(
        "let a = [1];\nlet b = a;\nprint(a == b, [a] == [[1]], [1] == \"[1]\", [nil] == [false]);",
        "true true false false\n",
    );
}

#[test]
fn lists_cannot_be_ordered() {
    expect_runtime_error(
        "[1] < [2];",
        "",
        &diagnostic(1, 1, "cannot apply `<` to list and list", "[1] < [2];", 9),
    );
}

#[test]
fn a_list_that_contains_itself_is_printed_with_dots() {
    expect_output("let a = [1];\npush(a, a);\nprint(a);", "[1, [...]]\n");
    expect_output(
        "let a = [];\nlet b = [a];\npush(a, b);\nprint(a, b);",
        "[[[...]]] [[[...]]]\n",
    );
    // The same list twice, side by side, is not inside itself.
    expect_output("let x = [0];\nprint([x, x]);", "[[0], [0]]\n");
    expect_output(
        "let a = [1];\npush(a, a);\nprint(str(a), [a, a]);",
        "[1, [...]] [[1, [...]], [1, [...]]]\n",
    );
}

#[test]
fn lists_nested_one_thousand_deep_work() {
    // SPEC 13.5: a chain of 1000 values must work. Building, printing,
    // comparing and freeing a nested list all walk it.
    let script = "
        fn nest(levels) {
            var list = [];
            for i in 1..levels { list = [list]; }
            list
        }
        let a = nest(1000);
        let b = nest(1000);
        let text = str(a);
        print(len(text), text[999] + text[1000], a == b, a == nest(999));";
    expect_output(script, "2000 [] true false\n");
}

// ----- for -----------------------------------------------------------------

#[test]
fn for_over_a_list() {
    expect_output("for x in [1, \"two\", [3]] { print(x); }", "1\ntwo\n[3]\n");
    expect_output(
        "var sum = 0;\nfor n in [1, 2, 3, 4] { sum = sum + n; }\nprint(sum);",
        "10\n",
    );
    expect_output(
        "for x in [] { print(\"never\"); }\nprint(\"done\");",
        "done\n",
    );
}

#[test]
fn for_over_a_string_gives_its_characters() {
    expect_output(
        "for c in \"aż🐢\" { print(c, len(c), type(c)); }",
        "a 1 str\nż 1 str\n🐢 1 str\n",
    );
    expect_output(
        "var reversed = \"\";\nfor c in \"tansy\" { reversed = c + reversed; }\nprint(reversed);",
        "ysnat\n",
    );
}

#[test]
fn for_over_a_range_stops_before_the_limit() {
    expect_output("for i in 0..4 { print(i); }", "0\n1\n2\n3\n");
    expect_output("for i in -2..1 { print(i); }", "-2\n-1\n0\n");
    expect_output(
        "for i in 3..3 { print(i); }\nfor i in 5..1 { print(i); }\nprint(\"none\");",
        "none\n",
    );
}

#[test]
fn range_bounds_are_expressions_evaluated_once() {
    let script = "
        var calls = 0;
        fn limit() { calls = calls + 1; 3 }
        var n = 1;
        for i in n - 1..limit() + n { n = 100; print(i); }
        print(calls);";
    expect_output(script, "0\n1\n2\n3\n1\n");
}

#[test]
fn a_range_near_the_largest_integer_does_not_overflow() {
    expect_output(
        "for i in 9223372036854775805..9223372036854775807 { print(i); }",
        "9223372036854775805\n9223372036854775806\n",
    );
}

#[test]
fn break_and_continue_in_for() {
    let script = "
        for i in 0..10 {
            if i == 1 { continue; }
            if i == 4 { break; }
            print(i);
        }
        for x in [1, 2, 3] { for y in [10, 20, 30] { if y == 20 { break; } print(x + y); } }";
    expect_output(script, "0\n2\n3\n11\n12\n13\n");
}

#[test]
fn return_from_inside_for() {
    let script = "
        fn index_of(xs, wanted) {
            var i = 0;
            for x in xs { if x == wanted { return i; } i = i + 1; }
            -1
        }
        print(index_of([5, 6, 7], 7), index_of([5, 6, 7], 8), index_of(\"hello\", \"l\"));";
    expect_output(script, "2 -1 2\n");
}

#[test]
fn each_round_has_its_own_loop_variable() {
    let script = "
        let fs = [];
        for i in 0..3 { push(fs, fn() { i * 10 }); }
        for word in [\"a\", \"b\"] { push(fs, fn() { word }); }
        for f in fs { print(f()); }";
    expect_output(script, "0\n10\n20\na\nb\n");
}

#[test]
fn the_loop_variable_cannot_be_assigned_and_ends_with_the_loop() {
    expect_compile_error(
        "for i in 0..3 { i = 5; }",
        &diagnostic(
            1,
            17,
            "cannot assign to `i`: it is not declared with `var`",
            "for i in 0..3 { i = 5; }",
            1,
        ),
    );
    expect_compile_error(
        "for item in [1] { }\nprint(item);",
        &diagnostic(2, 7, "unknown name `item`", "print(item);", 4),
    );
    expect_compile_error(
        "for i in 0..3 { let i = 1; }",
        &diagnostic(
            1,
            21,
            "`i` is already declared in this block",
            "for i in 0..3 { let i = 1; }",
            1,
        ),
    );
    // The variable is not in scope in the header: this `i` is unknown.
    expect_compile_error(
        "for i in 0..i { }",
        &diagnostic(1, 13, "unknown name `i`", "for i in 0..i { }", 1),
    );
}

#[test]
fn a_list_that_changes_during_the_loop() {
    // The loop looks at the list again before every round.
    expect_output(
        "let xs = [1, 2, 3];\nfor x in xs { if x < 3 { push(xs, x + 10); } }\nprint(xs);",
        "[1, 2, 3, 11, 12]\n",
    );
    expect_output(
        "let xs = [1, 2, 3, 4];\nfor x in xs { print(x); pop(xs); }\nprint(xs);",
        "1\n2\n[1, 2]\n",
    );
    expect_output(
        "let xs = [1, 2, 3];\nfor x in xs { xs[2] = \"changed\"; print(x); }",
        "1\n2\nchanged\n",
    );
}

#[test]
fn what_for_cannot_iterate() {
    expect_runtime_error(
        "for x in 5 { }",
        "",
        &diagnostic(1, 10, "cannot iterate over int", "for x in 5 { }", 1),
    );
    expect_runtime_error(
        "for x in nil { }",
        "",
        &diagnostic(1, 10, "cannot iterate over nil", "for x in nil { }", 3),
    );
    expect_runtime_error(
        "for i in 0..\"9\" { }",
        "",
        &diagnostic(1, 13, "expected int, found str", "for i in 0..\"9\" { }", 3),
    );
    expect_runtime_error(
        "for i in nil..3 { }",
        "",
        &diagnostic(1, 10, "expected int, found nil", "for i in nil..3 { }", 3),
    );
}

#[test]
fn both_bounds_of_a_range_are_evaluated_before_either_is_checked() {
    // As everywhere in Tansy: first evaluate the parts, then check them.
    // The limit is evaluated (and fails) although the first bound is wrong.
    expect_runtime_error(
        "for i in nil..1 / 0 { }",
        "",
        &diagnostic(1, 15, "division by zero", "for i in nil..1 / 0 { }", 5),
    );
    // When both bounds are wrong, the first one is reported.
    expect_runtime_error(
        "for i in \"a\"..nil { }",
        "",
        &diagnostic(
            1,
            10,
            "expected int, found str",
            "for i in \"a\"..nil { }",
            3,
        ),
    );
}

#[test]
fn syntax_errors_in_lists_indexing_and_for() {
    expect_compile_error(
        "print([1 2]);",
        &diagnostic(1, 10, "expected `,` or `]`, found `2`", "print([1 2]);", 1),
    );
    expect_compile_error(
        "print([1, 2);",
        &diagnostic(1, 12, "expected `,` or `]`, found `)`", "print([1, 2);", 1),
    );
    expect_compile_error(
        "print([,]);",
        &diagnostic(1, 8, "expected expression, found `,`", "print([,]);", 1),
    );
    expect_compile_error(
        "let xs = [1];\nxs[0;",
        &diagnostic(2, 5, "expected `]`, found `;`", "xs[0;", 1),
    );
    expect_compile_error(
        "let xs = [1];\nxs[];",
        &diagnostic(2, 4, "expected expression, found `]`", "xs[];", 1),
    );
    expect_compile_error(
        "for in [1] { }",
        &diagnostic(1, 5, "expected a name, found `in`", "for in [1] { }", 2),
    );
    expect_compile_error(
        "for x [1] { }",
        &diagnostic(1, 7, "expected `in`, found `[`", "for x [1] { }", 1),
    );
    expect_compile_error(
        "for x in [1] print(x);",
        &diagnostic(
            1,
            14,
            "expected `{`, found `print`",
            "for x in [1] print(x);",
            5,
        ),
    );
    // A range is part of `for`, not an expression.
    expect_compile_error(
        "let r = 0..3;",
        &diagnostic(1, 10, "expected `;`, found `..`", "let r = 0..3;", 2),
    );
}

// ----- small programs ------------------------------------------------------

#[test]
fn map_filter_and_fold_written_in_tansy() {
    let script = "
        fn map(xs, f) { let out = []; for x in xs { push(out, f(x)); } out }
        fn filter(xs, keep) { let out = []; for x in xs { if keep(x) { push(out, x); } } out }
        fn fold(xs, start, f) { var acc = start; for x in xs { acc = f(acc, x); } acc }
        let numbers = [1, 2, 3, 4, 5, 6];
        let squares = map(numbers, fn(n) { n * n });
        let even = filter(squares, fn(n) { n % 2 == 0 });
        print(squares, even, fold(even, 0, fn(a, b) { a + b }));
        print(fold(map(numbers, str), \"\", fn(a, b) { a + b }));";
    expect_output(script, "[1, 4, 9, 16, 25, 36] [4, 16, 36] 56\n123456\n");
}

#[test]
fn sorting_a_list_in_place() {
    let script = "
        fn sort(xs) {
            for i in 0..len(xs) {
                var smallest = i;
                for j in i + 1..len(xs) { if xs[j] < xs[smallest] { smallest = j; } }
                let held = xs[i];
                xs[i] = xs[smallest];
                xs[smallest] = held;
            }
        }
        let numbers = [5, 3, 9, 1, 5, -2];
        let words = [\"pear\", \"apple\", \"fig\"];
        sort(numbers);
        sort(words);
        print(numbers, words);";
    expect_output(
        script,
        "[-2, 1, 3, 5, 5, 9] [\"apple\", \"fig\", \"pear\"]\n",
    );
}

#[test]
fn splitting_and_joining_text() {
    let script = "
        fn split(text, separator) {
            let parts = [];
            var current = \"\";
            for c in text {
                if c == separator { push(parts, current); current = \"\"; } else { current = current + c; }
            }
            push(parts, current);
            parts
        }
        fn join(parts, separator) {
            var out = \"\";
            for i in 0..len(parts) {
                if i > 0 { out = out + separator; }
                out = out + parts[i];
            }
            out
        }
        let parts = split(\"a,bb,,ccc\", \",\");
        print(parts, len(parts), join(parts, \" | \"));";
    expect_output(script, "[\"a\", \"bb\", \"\", \"ccc\"] 4 a | bb |  | ccc\n");
}
