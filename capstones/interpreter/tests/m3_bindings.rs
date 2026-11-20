//! Milestone 3: bindings, blocks and control flow.
//!
//! SPEC sections 4.3 (statements and `;`), 6 (bindings and scope), 7.7
//! (`if`), 7.8 (blocks) and 8.1 to 8.3 (assignment, `while`, `break`,
//! `continue`).

mod common;

use common::{diagnostic, expect_compile_error, expect_output, expect_runtime_error};

// ----- let, var, assignment -----------------------------------------------

#[test]
fn let_binds_a_name_to_a_value() {
    expect_output("let x = 2;\nlet y = x * 21;\nprint(x, y);", "2 42\n");
}

#[test]
fn var_bindings_can_be_assigned() {
    expect_output("var n = 1;\nn = n + 1;\nn = n * 10;\nprint(n);", "20\n");
}

#[test]
fn a_binding_can_hold_any_type_over_time() {
    expect_output(
        "var v = 1; print(v); v = \"text\"; print(v); v = nil; print(v);",
        "1\ntext\nnil\n",
    );
}

#[test]
fn the_semicolon_before_a_closing_brace_or_the_end_is_optional() {
    expect_output("{ let a = 1 }\n{ var b = 2; b = 3 }\nlet c = 4", "");
    expect_output("var x = 1; { x = 2 } print(x)", "2\n");
}

#[test]
fn the_right_side_is_evaluated_before_the_assignment() {
    expect_output("var a = 1; a = a + a; a = a + a; print(a);", "4\n");
}

// ----- blocks --------------------------------------------------------------

#[test]
fn a_block_is_an_expression_with_the_value_of_its_tail() {
    expect_output(
        "let v = { let a = 20; a + 1 };\nprint(v, { 1; 2; 3 });",
        "21 3\n",
    );
}

#[test]
fn a_block_without_a_tail_is_nil() {
    expect_output("print({ 1; }, {}, { let a = 1; });", "nil nil nil\n");
}

#[test]
fn a_semicolon_after_a_block_or_an_if_is_an_empty_statement() {
    // SPEC 4.3, rule 4: an `if` or a block that starts a statement is the
    // tail only if the sequence ends directly after it. With a `;` behind
    // it, the sequence ends after an empty statement and has no tail.
    expect_output(
        "print({ if true { 1 } else { 2 }; }, { { 3 }; }, { { 4 } });",
        "nil nil 4\n",
    );
}

#[test]
fn an_inner_block_can_shadow_an_outer_name() {
    expect_output(
        "let a = 1;\n{\n  let a = 2;\n  print(a);\n  { let a = 3; print(a); }\n  print(a);\n}\nprint(a);",
        "2\n3\n2\n1\n",
    );
}

#[test]
fn an_inner_block_reads_and_assigns_outer_bindings() {
    expect_output(
        "var total = 0;\nlet step = 5;\n{ total = total + step; { total = total + step; } }\nprint(total);",
        "10\n",
    );
}

#[test]
fn a_shadowing_binding_may_differ_in_kind() {
    // The outer `a` cannot be assigned; the inner one is a different
    // binding and can.
    expect_output(
        "let a = 1; { var a = 2; a = 3; print(a); } print(a);",
        "3\n1\n",
    );
}

#[test]
fn blocks_nest_fifty_deep() {
    let script = format!(
        "var n = 0;\n{}n = n + 1;{}\nprint(n);",
        "{ let a = 1; ".repeat(50),
        " }".repeat(50)
    );
    expect_output(&script, "1\n");
}

// ----- if ------------------------------------------------------------------

#[test]
fn if_chooses_a_branch() {
    expect_output(
        "if 1 < 2 { print(\"yes\"); } else { print(\"no\"); }\nif 1 > 2 { print(\"yes\"); } else { print(\"no\"); }",
        "yes\nno\n",
    );
}

#[test]
fn if_without_else_does_nothing_when_the_condition_is_false() {
    expect_output(
        "if false { print(\"never\"); }\nprint(\"after\");",
        "after\n",
    );
}

#[test]
fn else_if_chains() {
    let script = "
        var n = 0;
        while n < 4 {
            if n == 0 { print(\"zero\"); } else if n == 1 { print(\"one\"); } else if n == 2 { print(\"two\"); } else { print(\"many\"); }
            n = n + 1;
        }";
    expect_output(script, "zero\none\ntwo\nmany\n");
}

#[test]
fn long_chains_of_if_are_accepted() {
    // SPEC 13.2: 250 links of `else if`, and 250 `if`s each in the
    // condition of the one before, need no open delimiter and must work.
    let chain = format!(
        "let n = 249;\nprint({}{{ \"none\" }});",
        (0..250)
            .map(|i| format!("if n == {i} {{ {i} }} else "))
            .collect::<String>()
    );
    expect_output(&chain, "249\n");
    let conditions = format!(
        "print({}true{} {{ \"innermost\" }});",
        "if ".repeat(250),
        " { true }".repeat(249)
    );
    expect_output(&conditions, "innermost\n");
}

#[test]
fn if_is_an_expression() {
    expect_output(
        "let a = 7;\nlet size = if a > 5 { \"big\" } else { \"small\" };\nprint(size);",
        "big\n",
    );
    expect_output(
        "print(if false { 1 } else if false { 2 } else { 3 }, 10 + if true { 1 } else { 2 });",
        "3 11\n",
    );
}

#[test]
fn an_if_without_else_whose_condition_is_false_is_nil() {
    expect_output("print(if false { 1 }, if true { 1 });", "nil 1\n");
}

#[test]
fn an_if_at_the_end_of_a_block_is_its_value() {
    expect_output(
        "let v = { let n = 3; if n > 2 { \"a\" } else { \"b\" } };\nprint(v);",
        "a\n",
    );
}

#[test]
fn the_condition_must_be_a_bool() {
    expect_runtime_error(
        "if 1 { print(\"x\"); }",
        "",
        &diagnostic(
            1,
            4,
            "expected bool, found int",
            "if 1 { print(\"x\"); }",
            1,
        ),
    );
    expect_runtime_error(
        "print(\"a\");\nif \"\" + \"s\" { }",
        "a\n",
        &diagnostic(2, 4, "expected bool, found str", "if \"\" + \"s\" { }", 8),
    );
    // Parentheses around the condition are not part of its span.
    expect_runtime_error(
        "if (nil) { }",
        "",
        &diagnostic(1, 5, "expected bool, found nil", "if (nil) { }", 3),
    );
}

#[test]
fn only_the_chosen_branch_is_evaluated() {
    expect_output(
        "print(if true { 1 } else { 1 / 0 }, if false { 1 / 0 } else { 2 });",
        "1 2\n",
    );
}

// ----- statements that start with `if` or `{` ------------------------------

#[test]
fn a_statement_that_starts_with_if_or_a_brace_ends_at_its_closing_brace() {
    // Two statements each time: the second one starts with `-` or `(`.
    expect_output("if true { 1 } else { 2 }\n-3;\nprint(\"ok\");", "ok\n");
    expect_output("var n = 0;\n{ n = 1 }\n(n);\nprint(n);", "1\n");
}

#[test]
fn such_a_statement_is_not_continued_by_an_operator() {
    expect_compile_error(
        "{ 1 } + 2;",
        &diagnostic(1, 7, "expected expression, found `+`", "{ 1 } + 2;", 1),
    );
    expect_compile_error(
        "if true { 1 } else { 2 } == 1;",
        &diagnostic(
            1,
            26,
            "expected expression, found `==`",
            "if true { 1 } else { 2 } == 1;",
            2,
        ),
    );
    // In the middle of an expression there is no such rule.
    expect_output(
        "let v = if true { 1 } else { 2 } + 10;\nprint(v, 0 + { 5 } * 2);",
        "11 10\n",
    );
}

// ----- while, break, continue ---------------------------------------------

#[test]
fn while_repeats_while_the_condition_holds() {
    expect_output(
        "var i = 1;\nvar sum = 0;\nwhile i <= 100 { sum = sum + i; i = i + 1; }\nprint(sum, i);",
        "5050 101\n",
    );
}

#[test]
fn while_with_a_false_condition_never_runs() {
    expect_output(
        "while false { print(\"never\"); }\nprint(\"done\");",
        "done\n",
    );
}

#[test]
fn break_leaves_the_innermost_loop() {
    let script = "
        var i = 0;
        while true {
            if i == 3 { break; }
            var j = 0;
            while true {
                if j == 2 { break }
                print(i, j);
                j = j + 1;
            }
            i = i + 1;
        }
        print(\"end\", i);";
    expect_output(script, "0 0\n0 1\n1 0\n1 1\n2 0\n2 1\nend 3\n");
}

#[test]
fn continue_starts_the_next_round() {
    let script = "
        var i = 0;
        while i < 6 {
            i = i + 1;
            if i % 2 == 0 { continue; }
            print(i);
        }";
    expect_output(script, "1\n3\n5\n");
}

#[test]
fn break_works_from_inside_an_expression() {
    // `break` is a statement, but the block that contains it can be
    // anywhere, here in an argument.
    expect_output(
        "var n = 0;\nwhile true { n = n + 1; print(n, if n == 2 { break; } else { \"go\" }); }\nprint(\"out\");",
        "1 go\nout\n",
    );
}

#[test]
fn the_condition_of_while_must_be_a_bool() {
    expect_runtime_error(
        "while nil { }",
        "",
        &diagnostic(1, 7, "expected bool, found nil", "while nil { }", 3),
    );
    // It is checked every round.
    expect_runtime_error(
        "var c = true;\nwhile c { print(\"round\"); c = 0; }",
        "round\n",
        &diagnostic(
            2,
            7,
            "expected bool, found int",
            "while c { print(\"round\"); c = 0; }",
            1,
        ),
    );
}

#[test]
fn each_entry_into_a_block_makes_fresh_bindings() {
    // `seen` starts again in every round.
    expect_output(
        "var i = 0;\nwhile i < 3 { var seen = 0; seen = seen + 1; print(seen); i = i + 1; }",
        "1\n1\n1\n",
    );
}

#[test]
fn factorial_overflows_at_twenty_one() {
    let script = "var f = 1;\nvar n = 1;\nwhile n <= 25 {\n  f = f * n;\n  if n >= 19 { print(n, f); }\n  n = n + 1;\n}";
    expect_runtime_error(
        script,
        "19 121645100408832000\n20 2432902008176640000\n",
        &diagnostic(4, 7, "integer overflow", "  f = f * n;", 5),
    );
}

// ----- evaluation order ----------------------------------------------------

#[test]
fn and_and_or_evaluate_the_right_side_only_when_needed() {
    let script = "
        var log = \"\";
        let a = false and { log = log + \"1\"; true };
        let b = true  and { log = log + \"2\"; true };
        let c = true  or  { log = log + \"3\"; true };
        let d = false or  { log = log + \"4\"; false };
        print(a, b, c, d, log);";
    expect_output(script, "false true true false 24\n");
}

#[test]
fn operands_and_arguments_are_evaluated_from_left_to_right() {
    let script = "
        var log = \"\";
        let sum = { log = log + \"a\"; 1 } + { log = log + \"b\"; 2 } * { log = log + \"c\"; 3 };
        print({ log = log + \"d\"; sum }, { log = log + \"e\"; log });";
    expect_output(script, "7 abcde\n");
}

// ----- assignment is a statement ------------------------------------------

#[test]
fn assignment_is_not_an_expression() {
    expect_compile_error(
        "var a = 1;\nvar b = 2;\na = b = 3;",
        &diagnostic(3, 7, "expected `;`, found `=`", "a = b = 3;", 1),
    );
    expect_compile_error(
        "var a = 1;\nprint((a = 2));",
        &diagnostic(2, 10, "expected `)`, found `=`", "print((a = 2));", 1),
    );
    expect_compile_error(
        "var a = 1;\nif a = 1 { }",
        &diagnostic(2, 6, "expected `{`, found `=`", "if a = 1 { }", 1),
    );
}

#[test]
fn only_a_name_or_an_element_can_be_assigned() {
    expect_compile_error(
        "1 + 2 = 3;",
        &diagnostic(1, 1, "invalid assignment target", "1 + 2 = 3;", 5),
    );
    expect_compile_error(
        "print(1) = 2;",
        &diagnostic(1, 1, "invalid assignment target", "print(1) = 2;", 8),
    );
    expect_compile_error(
        "var a = 1;\n-a = 2;",
        &diagnostic(2, 1, "invalid assignment target", "-a = 2;", 2),
    );
    // The span is the expression, without the parentheses around it.
    expect_compile_error(
        "(1 + 2) = 3;",
        &diagnostic(1, 2, "invalid assignment target", "(1 + 2) = 3;", 5),
    );
}

#[test]
fn parentheses_around_an_assignment_target_do_not_matter() {
    expect_output("var x = 1;\n(x) = 2;\n((x)) = x + 1;\nprint(x);", "3\n");
}

#[test]
fn syntax_errors_in_declarations() {
    expect_compile_error(
        "let 5 = 1;",
        &diagnostic(1, 5, "expected a name, found `5`", "let 5 = 1;", 1),
    );
    expect_compile_error(
        "let if = 1;",
        &diagnostic(1, 5, "expected a name, found `if`", "let if = 1;", 2),
    );
    expect_compile_error(
        "var x 5;",
        &diagnostic(1, 7, "expected `=`, found `5`", "var x 5;", 1),
    );
    expect_compile_error(
        "let x;",
        &diagnostic(1, 6, "expected `=`, found `;`", "let x;", 1),
    );
    expect_compile_error(
        "let x = ;",
        &diagnostic(1, 9, "expected expression, found `;`", "let x = ;", 1),
    );
    expect_compile_error(
        "let x = 1 let y = 2;",
        &diagnostic(
            1,
            11,
            "expected `;`, found `let`",
            "let x = 1 let y = 2;",
            3,
        ),
    );
}

#[test]
fn syntax_errors_in_blocks_and_control_flow() {
    expect_compile_error(
        "if true print(1);",
        &diagnostic(1, 9, "expected `{`, found `print`", "if true print(1);", 5),
    );
    expect_compile_error(
        "if true { } else print(1);",
        &diagnostic(
            1,
            18,
            "expected `if` or `{`, found `print`",
            "if true { } else print(1);",
            5,
        ),
    );
    expect_compile_error(
        "while { }",
        &diagnostic(1, 10, "expected `{`, found end of input", "while { }", 1),
    );
    expect_compile_error(
        "{ print(1);\n",
        &diagnostic(2, 1, "expected `}`, found end of input", "", 1),
    );
    expect_compile_error(
        "{ print(1)",
        &diagnostic(1, 11, "expected `}`, found end of input", "{ print(1)", 1),
    );
    expect_compile_error(
        "let v = while true { };",
        &diagnostic(
            1,
            9,
            "expected expression, found `while`",
            "let v = while true { };",
            5,
        ),
    );
    expect_compile_error(
        "else { }",
        &diagnostic(1, 1, "expected expression, found `else`", "else { }", 4),
    );
}

// ----- name errors (found before anything runs) ----------------------------

#[test]
fn an_unknown_name_is_found_before_the_script_runs() {
    expect_compile_error(
        "print(\"start\");\nprint(y + 1);",
        &diagnostic(2, 7, "unknown name `y`", "print(y + 1);", 1),
    );
    expect_compile_error(
        "total = 1;",
        &diagnostic(1, 1, "unknown name `total`", "total = 1;", 5),
    );
    // Even in code that would never run.
    expect_compile_error(
        "if false { prnt(1); }",
        &diagnostic(1, 12, "unknown name `prnt`", "if false { prnt(1); }", 4),
    );
}

#[test]
fn a_name_is_not_known_outside_its_block() {
    expect_compile_error(
        "{ let inner = 1; }\nprint(inner);",
        &diagnostic(2, 7, "unknown name `inner`", "print(inner);", 5),
    );
    expect_compile_error(
        "if true { let a = 1; } else { print(a); }",
        &diagnostic(
            1,
            37,
            "unknown name `a`",
            "if true { let a = 1; } else { print(a); }",
            1,
        ),
    );
}

#[test]
fn a_name_can_be_declared_only_once_in_a_block() {
    expect_compile_error(
        "let a = 1;\nlet a = 2;",
        &diagnostic(
            2,
            5,
            "`a` is already declared in this block",
            "let a = 2;",
            1,
        ),
    );
    expect_compile_error(
        "var count = 1;\nlet count = 2;",
        &diagnostic(
            2,
            5,
            "`count` is already declared in this block",
            "let count = 2;",
            5,
        ),
    );
    expect_compile_error(
        "{ let a = 1; { let a = 2; } var a = 3; }",
        &diagnostic(
            1,
            33,
            "`a` is already declared in this block",
            "{ let a = 1; { let a = 2; } var a = 3; }",
            1,
        ),
    );
}

#[test]
fn the_first_of_two_declarations_is_the_one_that_counts() {
    // `n` is a `var` (its first declaration), so line 2 is correct. The only
    // error is the second declaration, although it comes later in the text.
    expect_compile_error(
        "var n = 1;\nn = 2;\nlet n = 3;",
        &diagnostic(
            3,
            5,
            "`n` is already declared in this block",
            "let n = 3;",
            1,
        ),
    );
    // The other way round, the assignment on line 3 is wrong as well, but
    // the second declaration starts first in the text.
    expect_compile_error(
        "let n = 1;\nvar n = 2;\nn = 3;",
        &diagnostic(
            2,
            5,
            "`n` is already declared in this block",
            "var n = 2;",
            1,
        ),
    );
}

#[test]
fn only_var_bindings_can_be_assigned() {
    expect_compile_error(
        "let a = 1;\na = 2;",
        &diagnostic(
            2,
            1,
            "cannot assign to `a`: it is not declared with `var`",
            "a = 2;",
            1,
        ),
    );
    expect_compile_error(
        "let limit = 1;\n{ { limit = limit + 1; } }",
        &diagnostic(
            2,
            5,
            "cannot assign to `limit`: it is not declared with `var`",
            "{ { limit = limit + 1; } }",
            5,
        ),
    );
    // The built-in functions are bindings too, and not `var` ones.
    expect_compile_error(
        "print = 1;",
        &diagnostic(
            1,
            1,
            "cannot assign to `print`: it is not declared with `var`",
            "print = 1;",
            5,
        ),
    );
}

#[test]
fn break_and_continue_need_a_loop() {
    expect_compile_error(
        "print(1);\nbreak;",
        &diagnostic(2, 1, "`break` outside of a loop", "break;", 5),
    );
    expect_compile_error(
        "if true { continue; }",
        &diagnostic(
            1,
            11,
            "`continue` outside of a loop",
            "if true { continue; }",
            8,
        ),
    );
    expect_compile_error(
        "while false { }\nbreak",
        &diagnostic(2, 1, "`break` outside of a loop", "break", 5),
    );
    // The condition of a loop is not inside the loop's body.
    expect_compile_error(
        "while { break; true } { }",
        &diagnostic(
            1,
            9,
            "`break` outside of a loop",
            "while { break; true } { }",
            5,
        ),
    );
}

#[test]
fn a_syntax_error_is_reported_before_any_name_error() {
    // The name error comes first in the text, but names are only checked
    // in a script that parses.
    expect_compile_error(
        "print(nope);\nprint(1 +);",
        &diagnostic(2, 10, "expected expression, found `)`", "print(1 +);", 1),
    );
}

#[test]
fn the_first_name_error_in_the_text_is_reported() {
    expect_compile_error(
        "{\n  print(first);\n  let a = 1;\n  let a = 2;\n  break;\n}",
        &diagnostic(2, 9, "unknown name `first`", "  print(first);", 5),
    );
    expect_compile_error(
        "{\n  let a = 1;\n  let a = 2;\n  print(second);\n}",
        &diagnostic(
            3,
            7,
            "`a` is already declared in this block",
            "  let a = 2;",
            1,
        ),
    );
    expect_compile_error(
        "break;\nprint(third);\nlet b = 1; b = 2;",
        &diagnostic(1, 1, "`break` outside of a loop", "break;", 5),
    );
}

// ----- a name belongs to its whole block -----------------------------------

#[test]
fn a_name_refers_to_the_declaration_in_its_own_block_even_before_it() {
    // Inside the block, `a` is the block's `a` from the first line on. It
    // has no value yet when `print` runs.
    expect_runtime_error(
        "let a = \"outer\";\n{\n  print(a);\n  let a = \"inner\";\n}",
        "",
        &diagnostic(
            3,
            9,
            "`a` is used before it is initialized",
            "  print(a);",
            1,
        ),
    );
}

#[test]
fn an_initializer_cannot_use_the_binding_it_initializes() {
    expect_runtime_error(
        "let x = 1;\n{ let x = x + 1; }",
        "",
        &diagnostic(
            2,
            11,
            "`x` is used before it is initialized",
            "{ let x = x + 1; }",
            1,
        ),
    );
    expect_runtime_error(
        "let y = y;",
        "",
        &diagnostic(
            1,
            9,
            "`y` is used before it is initialized",
            "let y = y;",
            1,
        ),
    );
}

#[test]
fn assigning_before_the_declaration_has_run_is_an_error() {
    expect_runtime_error(
        "print(\"a\");\n{ late = 1; var late = 0; }",
        "a\n",
        &diagnostic(
            2,
            3,
            "`late` is used before it is initialized",
            "{ late = 1; var late = 0; }",
            4,
        ),
    );
    // The right side is evaluated first; the binding is looked at when the
    // value is stored (SPEC 8.1).
    expect_runtime_error(
        "{ late = { print(\"right side\"); 1 }; var late = 0; }",
        "right side\n",
        &diagnostic(
            1,
            3,
            "`late` is used before it is initialized",
            "{ late = { print(\"right side\"); 1 }; var late = 0; }",
            4,
        ),
    );
}

#[test]
fn what_counts_is_when_the_code_runs_not_where_it_is_written() {
    // The loop is written, and runs, before the declaration of `step`. The
    // name is known (the whole top level is its scope), so the script
    // starts; the second round then reads a binding without a value.
    let script = "var round = 0;
var total = 0;
while round < 3 {
    if round > 0 { total = total + step; }
    round = round + 1;
}
let step = 5;
print(total);";
    expect_runtime_error(
        script,
        "",
        &diagnostic(
            4,
            36,
            "`step` is used before it is initialized",
            "    if round > 0 { total = total + step; }",
            4,
        ),
    );
}

#[test]
fn a_top_level_name_can_shadow_a_built_in_function() {
    expect_output(
        "{ let len = 3; print(len); }\nlet type = \"mine\";\nprint(type);",
        "3\nmine\n",
    );
}

#[test]
fn a_shadowed_built_in_function_is_hidden_in_the_whole_block() {
    // The top level declares its own `print` on line 2, so `print` on line
    // 1 is that binding (SPEC 6.2, 6.3), and it has no value yet.
    expect_runtime_error(
        "print(\"never printed\");\nlet print = 5;",
        "",
        &diagnostic(
            1,
            1,
            "`print` is used before it is initialized",
            "print(\"never printed\");",
            5,
        ),
    );
}
