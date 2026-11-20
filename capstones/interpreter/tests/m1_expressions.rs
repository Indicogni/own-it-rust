//! Milestone 1: the script runner, `print`, literals and operators.
//!
//! SPEC sections 2 (running a script), 3 (lexical structure), 4.2
//! (precedence), 5 (values and how they are printed) and 7.1 to 7.5
//! (operators).
//!
//! Nothing here needs variables, functions or lists. The only call is
//! `print(...)`.

mod common;

use common::expect_output;

// ----- the runner ----------------------------------------------------------

#[test]
fn prints_an_integer() {
    expect_output("print(42);", "42\n");
}

#[test]
fn an_empty_script_prints_nothing_and_succeeds() {
    expect_output("", "");
    expect_output("\n\n   \t\n", "");
    expect_output("// nothing but a comment", "");
}

#[test]
fn statements_run_in_order() {
    expect_output("print(1);\nprint(2);\nprint(3);\n", "1\n2\n3\n");
}

#[test]
fn the_last_semicolon_is_optional_and_empty_statements_are_allowed() {
    expect_output("print(1);;; print(2)", "1\n2\n");
    expect_output(";", "");
}

#[test]
fn the_value_of_a_script_is_not_printed() {
    // In a script the value of the final expression is thrown away. (The
    // REPL prints it; that is milestone 6.)
    expect_output("1 + 2", "");
    expect_output("print(1); 2", "1\n");
}

// ----- print ---------------------------------------------------------------

#[test]
fn print_takes_any_number_of_arguments() {
    expect_output("print();", "\n");
    expect_output("print(1, 2, 3);", "1 2 3\n");
    expect_output(
        r#"print("a", true, false, nil, 7);"#,
        "a true false nil 7\n",
    );
}

#[test]
fn print_allows_a_trailing_comma() {
    expect_output("print(1, 2,);", "1 2\n");
}

#[test]
fn print_returns_nil() {
    expect_output("print(print(1));", "1\nnil\n");
}

// ----- integers ------------------------------------------------------------

#[test]
fn integer_arithmetic() {
    expect_output(
        "print(1 + 2, 7 - 10, 6 * 7, 7 / 2, 7 % 2);",
        "3 -3 42 3 1\n",
    );
}

#[test]
fn division_truncates_toward_zero_and_the_remainder_follows_the_dividend() {
    expect_output("print(-7 / 2, 7 / -2, -7 / -2);", "-3 -3 3\n");
    expect_output("print(-7 % 2, 7 % -2, -7 % -2);", "-1 1 -1\n");
}

#[test]
fn integers_are_64_bit() {
    expect_output("print(9223372036854775807);", "9223372036854775807\n");
    // There is no negative literal: this is `-` applied to a literal, and
    // the smallest integer has to be computed.
    expect_output("print(-9223372036854775807 - 1);", "-9223372036854775808\n");
    expect_output("print(3037000499 * 3037000499);", "9223372030926249001\n");
}

#[test]
fn the_remainder_of_the_smallest_integer_by_minus_one_is_zero() {
    // The division overflows (milestone 2), the remainder does not.
    expect_output("print((-9223372036854775807 - 1) % -1);", "0\n");
}

#[test]
fn leading_zeros_do_not_change_an_integer() {
    expect_output("print(007, 0, 000);", "7 0 0\n");
    // The limit of SPEC 3.4 is on the value, not on the number of digits.
    expect_output("print(000000000000000000000000000042);", "42\n");
}

// ----- precedence and grouping --------------------------------------------

#[test]
fn multiplication_binds_tighter_than_addition() {
    expect_output(
        "print(1 + 2 * 3, 2 * 3 + 1, 10 - 6 / 2, 7 + 9 % 4);",
        "7 7 7 8\n",
    );
}

#[test]
fn parentheses_group() {
    expect_output("print((1 + 2) * 3, 2 * (3 + 1), ((((5)))));", "9 8 5\n");
}

#[test]
fn binary_operators_group_to_the_left() {
    expect_output(
        "print(10 - 4 - 3, 100 / 10 / 5, 2 * 7 % 4, 1 - 2 + 3, 20 % 7 % 4);",
        "3 2 2 2 2\n",
    );
}

#[test]
fn unary_minus() {
    expect_output("print(-3, - -3, -(1 + 2), -0);", "-3 3 -3 0\n");
    expect_output("print(2 - -1, 2--1, -3 * -2, 6 / -3);", "3 3 6 -2\n");
}

#[test]
fn unary_minus_binds_tighter_than_multiplication() {
    // (-a) * b and -(a * b) are the same number, so the grouping shows only
    // at the edge of the integer range: (-2^62) * 2 fits, 2^62 * 2 does not.
    expect_output("print(-4611686018427387904 * 2);", "-9223372036854775808\n");
}

#[test]
fn an_expression_may_span_lines() {
    expect_output("print(1 +\n      2\n      * 3\n);\n", "7\n");
}

// ----- comparison and equality --------------------------------------------

#[test]
fn integers_compare() {
    expect_output(
        "print(1 < 2, 2 < 1, 2 <= 2, 3 > 4, 4 >= 5, 5 >= 5);",
        "true false true false false true\n",
    );
}

#[test]
fn comparison_binds_looser_than_arithmetic() {
    expect_output(
        "print(1 + 1 == 2, 2 * 3 > 5, 1 < 2 - 5, 10 - 1 >= 3 * 3);",
        "true true false true\n",
    );
}

#[test]
fn equality_works_on_every_type() {
    expect_output(
        r#"print(1 == 1, 1 != 1, "a" == "a", "a" == "b", true == true, nil == nil, nil != nil);"#,
        "true false true false true true false\n",
    );
}

#[test]
fn values_of_different_types_are_never_equal() {
    expect_output(
        r#"print(1 == "1", 0 == false, nil == false, "" == nil, 1 == true, 1 != "1");"#,
        "false false false false false true\n",
    );
}

#[test]
fn a_comparison_in_parentheses_can_be_compared() {
    expect_output(
        "print((1 < 2) == true, (1 < 2) == (2 < 1), (1 == 1) != (2 == 2));",
        "true false false\n",
    );
}

// ----- logic ---------------------------------------------------------------

#[test]
fn logical_operators() {
    expect_output(
        "print(true and true, true and false, false and true, false and false);",
        "true false false false\n",
    );
    expect_output(
        "print(true or true, true or false, false or true, false or false);",
        "true true true false\n",
    );
    expect_output(
        "print(not true, not false, not not true);",
        "false true true\n",
    );
}

#[test]
fn and_binds_tighter_than_or() {
    expect_output(
        "print(true or true and false, false and false or true, (true or true) and false);",
        "true true false\n",
    );
}

#[test]
fn not_binds_looser_than_comparison_and_tighter_than_and() {
    // `not 1 == 2` is `not (1 == 2)`.
    expect_output(
        "print(not 1 == 2, not 1 < 2, not 2 + 2 == 5);",
        "true false true\n",
    );
    // `not true or true` is `(not true) or true`.
    expect_output(
        "print(not true or true, not (true or true), not false and false);",
        "true false false\n",
    );
    expect_output(
        "print(true and not false, false or not false, not true == false);",
        "true true true\n",
    );
}

// ----- strings -------------------------------------------------------------

#[test]
fn strings_print_without_quotes() {
    expect_output(r#"print("hello, world");"#, "hello, world\n");
    expect_output(r#"print("");"#, "\n");
    expect_output(r#"print("a", "", "b");"#, "a  b\n");
}

#[test]
fn plus_joins_strings() {
    expect_output(
        r#"print("tan" + "sy", "a" + "b" + "c", "" + "x" + "");"#,
        "tansy abc x\n",
    );
}

#[test]
fn escape_sequences() {
    expect_output(r#"print("one\ttwo\nthree");"#, "one\ttwo\nthree\n");
    expect_output(
        r#"print("quote: \" backslash: \\ done");"#,
        "quote: \" backslash: \\ done\n",
    );
    expect_output(r#"print("a\rb\0c");"#, "a\rb\0c\n");
}

#[test]
fn strings_may_contain_any_unicode_text() {
    expect_output("print(\"żółw 🐢 naïve\");", "żółw 🐢 naïve\n");
    expect_output("print(\"日本\" + \"語\");", "日本語\n");
}

#[test]
fn strings_compare_by_code_point() {
    expect_output(
        r#"print("a" < "b", "abc" < "abd", "ab" < "abc", "" < "a", "b" > "abc");"#,
        "true true true true true\n",
    );
    // Upper-case letters come before lower-case letters in Unicode.
    expect_output(
        r#"print("Z" < "a", "apple" <= "apple", "apple" >= "apples");"#,
        "true true false\n",
    );
    // U+00E9 (é) comes before U+65E5 (日), and both after every ASCII letter.
    expect_output("print(\"z\" < \"é\", \"é\" < \"日\");", "true true\n");
}

// ----- comments and whitespace --------------------------------------------

#[test]
fn comments_run_to_the_end_of_the_line() {
    expect_output(
        "// a comment\nprint(1); // another\n// print(2);\nprint(3);",
        "1\n3\n",
    );
    // Anything may appear in a comment.
    expect_output("print(1); // \" @ \\q { ( [ 99999999999999999999", "1\n");
}

#[test]
fn two_slashes_inside_a_string_are_not_a_comment() {
    expect_output(r#"print("see // here");"#, "see // here\n");
}

#[test]
fn spaces_tabs_and_line_ends_separate_tokens() {
    expect_output("print (\t1\t+\t2 )\t;", "3\n");
    expect_output("print\n(\n1\n)\n;\n", "1\n");
    expect_output("print(1+2*3-4);", "3\n");
}

#[test]
fn windows_line_endings_are_accepted() {
    expect_output("print(1); // comment\r\nprint(\r\n  2\r\n);\r\n", "1\n2\n");
}
