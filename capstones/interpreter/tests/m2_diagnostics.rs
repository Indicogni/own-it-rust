//! Milestone 2: errors with a position.
//!
//! SPEC section 11 (the format of a diagnostic, spans, the list of
//! messages), 2.2 (exit statuses) and 13 (the nesting limit).
//!
//! The expected text is built with `diagnostic(line, column, message,
//! source_line, width)`: the header line, the source line indented by four
//! spaces, and `width` carets under column `column`.

mod common;

use common::{
    COMPILE_ERROR, diagnostic, expect_compile_error, expect_output, expect_runtime_error,
    run_script,
};

// ----- lexical errors ------------------------------------------------------

#[test]
fn an_unexpected_character() {
    expect_compile_error(
        "print(1);\nprint(2 @ 3);\n",
        &diagnostic(2, 9, "unexpected character '@'", "print(2 @ 3);", 1),
    );
}

#[test]
fn characters_that_are_only_half_an_operator() {
    expect_compile_error(
        "print(1 ! 2);",
        &diagnostic(1, 9, "unexpected character '!'", "print(1 ! 2);", 1),
    );
    expect_compile_error(
        "print(1 . 2);",
        &diagnostic(1, 9, "unexpected character '.'", "print(1 . 2);", 1),
    );
    expect_compile_error(
        "print(1 & 2);",
        &diagnostic(1, 9, "unexpected character '&'", "print(1 & 2);", 1),
    );
}

#[test]
fn a_letter_outside_ascii_is_not_part_of_a_name() {
    expect_compile_error(
        "print(é);",
        &diagnostic(1, 7, "unexpected character 'é'", "print(é);", 1),
    );
}

#[test]
fn an_unterminated_string_is_reported_at_its_opening_quote() {
    expect_compile_error(
        "print(\"abc);\nprint(\"next line\");\n",
        &diagnostic(1, 7, "unterminated string", "print(\"abc);", 1),
    );
    // The same at the end of the file, with and without a final line end.
    expect_compile_error(
        "print(1);\n  \"abc",
        &diagnostic(2, 3, "unterminated string", "  \"abc", 1),
    );
    expect_compile_error("\"", &diagnostic(1, 1, "unterminated string", "\"", 1));
}

#[test]
fn a_backslash_at_the_end_of_a_line_leaves_the_string_unterminated() {
    expect_compile_error(
        "print(\"abc\\\n\");",
        &diagnostic(1, 7, "unterminated string", "print(\"abc\\", 1),
    );
}

#[test]
fn an_invalid_escape_sequence_is_reported_at_the_backslash() {
    expect_compile_error(
        r#"print("a\qb");"#,
        &diagnostic(
            1,
            9,
            r"invalid escape sequence '\q'",
            r#"print("a\qb");"#,
            2,
        ),
    );
    // The first problem in the string wins: here the escape, although the
    // string is also unterminated.
    expect_compile_error(
        r#"print("\x41);"#,
        &diagnostic(1, 8, r"invalid escape sequence '\x'", r#"print("\x41);"#, 2),
    );
}

#[test]
fn an_integer_literal_that_does_not_fit() {
    expect_compile_error(
        "print(9223372036854775808);",
        &diagnostic(
            1,
            7,
            "integer literal is too large",
            "print(9223372036854775808);",
            19,
        ),
    );
    // Forty digits: too large for every integer type Rust has. The whole
    // literal is the span.
    let literal = "1234567890".repeat(4);
    let script = format!("print({literal});");
    expect_compile_error(
        &script,
        &diagnostic(1, 7, "integer literal is too large", &script, 40),
    );
}

// ----- syntax errors -------------------------------------------------------

#[test]
fn a_missing_operand() {
    expect_compile_error(
        "print(1 + );",
        &diagnostic(1, 11, "expected expression, found `)`", "print(1 + );", 1),
    );
    expect_compile_error(
        "print(* 2);",
        &diagnostic(1, 7, "expected expression, found `*`", "print(* 2);", 1),
    );
}

#[test]
fn a_missing_comma_or_parenthesis_in_a_call() {
    expect_compile_error(
        "print(1 2);",
        &diagnostic(1, 9, "expected `,` or `)`, found `2`", "print(1 2);", 1),
    );
    expect_compile_error(
        "print((1 + 2);",
        &diagnostic(1, 14, "expected `,` or `)`, found `;`", "print((1 + 2);", 1),
    );
}

#[test]
fn a_missing_closing_parenthesis_of_a_group() {
    expect_compile_error(
        "print((1 + 2 3));",
        &diagnostic(1, 14, "expected `)`, found `3`", "print((1 + 2 3));", 1),
    );
}

#[test]
fn a_missing_semicolon() {
    expect_compile_error(
        "print(1) print(2);",
        &diagnostic(
            1,
            10,
            "expected `;`, found `print`",
            "print(1) print(2);",
            5,
        ),
    );
    expect_compile_error(
        "1 2",
        &diagnostic(1, 3, "expected `;`, found `2`", "1 2", 1),
    );
}

#[test]
fn the_found_token_is_shown_as_it_is_written() {
    expect_compile_error(
        r#"print(1 "two");"#,
        &diagnostic(
            1,
            9,
            r#"expected `,` or `)`, found `"two"`"#,
            r#"print(1 "two");"#,
            5,
        ),
    );
    expect_compile_error(
        "print(else);",
        &diagnostic(1, 7, "expected expression, found `else`", "print(else);", 4),
    );
    expect_compile_error(
        "print(1 <= >= 2);",
        &diagnostic(
            1,
            12,
            "expected expression, found `>=`",
            "print(1 <= >= 2);",
            2,
        ),
    );
}

#[test]
fn a_closing_delimiter_that_closes_nothing() {
    expect_compile_error(
        ")",
        &diagnostic(1, 1, "expected expression, found `)`", ")", 1),
    );
    expect_compile_error(
        "print(1); }",
        &diagnostic(1, 11, "expected expression, found `}`", "print(1); }", 1),
    );
    expect_compile_error(
        "print(1) }",
        &diagnostic(1, 10, "expected expression, found `}`", "print(1) }", 1),
    );
}

#[test]
fn the_end_of_input_is_a_position_too() {
    // After a final line end, the end of input is column 1 of the next
    // line, and that line is empty.
    expect_compile_error(
        "print(1,\n",
        &diagnostic(2, 1, "expected expression, found end of input", "", 1),
    );
    // Without a final line end it is one column past the last character.
    expect_compile_error(
        "print(1,",
        &diagnostic(
            1,
            9,
            "expected expression, found end of input",
            "print(1,",
            1,
        ),
    );
    expect_compile_error(
        "print(1",
        &diagnostic(
            1,
            8,
            "expected `,` or `)`, found end of input",
            "print(1",
            1,
        ),
    );
    expect_compile_error(
        "print(1 +  // to be continued",
        &diagnostic(
            1,
            30,
            "expected expression, found end of input",
            "print(1 +  // to be continued",
            1,
        ),
    );
}

#[test]
fn comparison_operators_cannot_be_chained() {
    let message = "comparison operators cannot be chained";
    expect_compile_error(
        "print(1 < 2 < 3);",
        &diagnostic(1, 13, message, "print(1 < 2 < 3);", 1),
    );
    expect_compile_error(
        "print(1 == 2 != 3);",
        &diagnostic(1, 14, message, "print(1 == 2 != 3);", 2),
    );
    expect_compile_error(
        "print(1 + 1 < 2 * 2 >= 3);",
        &diagnostic(1, 21, message, "print(1 + 1 < 2 * 2 >= 3);", 2),
    );
}

#[test]
fn not_cannot_be_an_operand_of_arithmetic_or_comparison() {
    let message = "expected expression, found `not`";
    expect_compile_error(
        "print(1 + not true);",
        &diagnostic(1, 11, message, "print(1 + not true);", 3),
    );
    expect_compile_error(
        "print(true == not true);",
        &diagnostic(1, 15, message, "print(true == not true);", 3),
    );
    expect_compile_error(
        "print(-not true);",
        &diagnostic(1, 8, message, "print(-not true);", 3),
    );
    // In parentheses it is an operand like any other.
    expect_output("print(true == (not false));", "true\n");
}

// ----- which error, and what else happens ---------------------------------

#[test]
fn nothing_runs_when_the_script_has_a_syntax_error() {
    // The error is on the last line; the first two lines must not print.
    expect_compile_error(
        "print(\"one\");\nprint(\"two\");\nprint(3 +);\n",
        &diagnostic(3, 10, "expected expression, found `)`", "print(3 +);", 1),
    );
}

#[test]
fn only_the_first_error_is_reported() {
    expect_compile_error(
        "print(1 +);\nprint(2 +);\nprint(@);\n",
        &diagnostic(1, 10, "expected expression, found `)`", "print(1 +);", 1),
    );
}

#[test]
fn the_first_error_in_the_text_wins_whatever_its_kind() {
    // A grammar error before a lexical error ...
    expect_compile_error(
        "print(1 +);\nprint(\"open);\n",
        &diagnostic(1, 10, "expected expression, found `)`", "print(1 +);", 1),
    );
    // ... and a lexical error before a grammar error.
    expect_compile_error(
        "print(\"open);\nprint(1 +);\n",
        &diagnostic(1, 7, "unterminated string", "print(\"open);", 1),
    );
    // A lexical error where any token would be wrong is still the lexical
    // error: there is no token to complain about.
    expect_compile_error(
        "print(1 @);",
        &diagnostic(1, 9, "unexpected character '@'", "print(1 @);", 1),
    );
}

// ----- columns and source lines -------------------------------------------

#[test]
fn a_column_is_a_character_not_a_byte() {
    expect_compile_error(
        "print(\"żółw\" + \"🐢\" + @);",
        &diagnostic(
            1,
            22,
            "unexpected character '@'",
            "print(\"żółw\" + \"🐢\" + @);",
            1,
        ),
    );
    // The carets count characters too.
    expect_compile_error(
        "print(1 \"żółw\");",
        &diagnostic(
            1,
            9,
            "expected `,` or `)`, found `\"żółw\"`",
            "print(1 \"żółw\");",
            6,
        ),
    );
}

#[test]
fn a_tab_is_one_column_and_is_shown_as_one_space() {
    expect_compile_error(
        "\t\tprint(1\t+ );",
        &diagnostic(1, 13, "expected expression, found `)`", "  print(1 + );", 1),
    );
}

#[test]
fn a_carriage_return_is_not_part_of_the_source_line() {
    expect_compile_error(
        "print(1);\r\nprint(2 +);\r\nprint(3);\r\n",
        &diagnostic(2, 10, "expected expression, found `)`", "print(2 +);", 1),
    );
}

#[test]
fn lines_are_counted_through_comments_and_blank_lines() {
    expect_compile_error(
        "// one\n\n// three\n   \nprint(5 $);\n",
        &diagnostic(5, 9, "unexpected character '$'", "print(5 $);", 1),
    );
}

// ----- the nesting limit ---------------------------------------------------

#[test]
fn one_hundred_open_delimiters_are_accepted() {
    // The `(` of the call is the first open delimiter, so 99 more fit.
    let script = format!("print({}7{});", "(".repeat(99), ")".repeat(99));
    expect_output(&script, "7\n");
}

#[test]
fn the_one_hundred_and_first_open_delimiter_is_an_error() {
    let line = format!("print({}7{});", "(".repeat(100), ")".repeat(100));
    // `print(` is columns 1 to 6; the hundredth `(` after it is column 106.
    expect_compile_error(&line, &diagnostic(1, 106, "nesting too deep", &line, 1));
}

#[test]
fn very_deep_nesting_is_an_error_and_not_a_crash() {
    let line = format!("print({}7{});", "(".repeat(200_000), ")".repeat(200_000));
    expect_compile_error(&line, &diagnostic(1, 106, "nesting too deep", &line, 1));
}

#[test]
fn delimiters_that_were_closed_do_not_count() {
    // 200 pairs, but never more than two open at once.
    let script = format!("print(0{});", " + (1)".repeat(200));
    expect_output(&script, "200\n");
}

#[test]
fn runs_of_250_operators_without_delimiters_are_accepted() {
    // SPEC 13.2: nesting that needs no delimiter must work up to 250.
    expect_output(&format!("print({}7);", "- ".repeat(250)), "7\n");
    expect_output(&format!("print({}7);", "-".repeat(249)), "-7\n");
    expect_output(&format!("print({}true);", "not ".repeat(250)), "true\n");
    expect_output(&format!("print(0{});", " + 1".repeat(250)), "250\n");
    expect_output(&format!("print(1{});", " * 1 - 0".repeat(125)), "1\n");
}

// ----- runtime errors: types ----------------------------------------------

#[test]
fn an_operator_applied_to_the_wrong_types() {
    expect_runtime_error(
        r#"print("a" + 1);"#,
        "",
        &diagnostic(
            1,
            7,
            "cannot apply `+` to str and int",
            r#"print("a" + 1);"#,
            7,
        ),
    );
}

#[test]
fn every_arithmetic_operator_checks_its_operands() {
    let cases = [
        (r#"1 - "a""#, "cannot apply `-` to int and str"),
        ("true * 2", "cannot apply `*` to bool and int"),
        ("nil / 1", "cannot apply `/` to nil and int"),
        (r#""a" % 2"#, "cannot apply `%` to str and int"),
        ("1 + nil", "cannot apply `+` to int and nil"),
        ("true + true", "cannot apply `+` to bool and bool"),
        (r#""5" * "5""#, "cannot apply `*` to str and str"),
    ];
    for (expression, message) in cases {
        let script = format!("print({expression});");
        expect_runtime_error(
            &script,
            "",
            &diagnostic(1, 7, message, &script, expression.chars().count()),
        );
    }
}

#[test]
fn ordering_needs_two_integers_or_two_strings() {
    let cases = [
        (r#"1 < "a""#, "cannot apply `<` to int and str"),
        ("true > false", "cannot apply `>` to bool and bool"),
        ("nil <= nil", "cannot apply `<=` to nil and nil"),
        (r#""a" >= 1"#, "cannot apply `>=` to str and int"),
    ];
    for (expression, message) in cases {
        let script = format!("print({expression});");
        expect_runtime_error(
            &script,
            "",
            &diagnostic(1, 7, message, &script, expression.chars().count()),
        );
    }
}

#[test]
fn unary_minus_needs_an_integer() {
    expect_runtime_error(
        r#"print(-"a");"#,
        "",
        &diagnostic(1, 7, "cannot apply `-` to str", r#"print(-"a");"#, 4),
    );
    expect_runtime_error(
        "print(- true);",
        "",
        &diagnostic(1, 7, "cannot apply `-` to bool", "print(- true);", 6),
    );
}

#[test]
fn logical_operators_need_booleans_and_point_at_the_operand() {
    expect_runtime_error(
        "print(not 1);",
        "",
        &diagnostic(1, 11, "expected bool, found int", "print(not 1);", 1),
    );
    expect_runtime_error(
        "print(1 and true);",
        "",
        &diagnostic(1, 7, "expected bool, found int", "print(1 and true);", 1),
    );
    expect_runtime_error(
        r#"print(true and "yes");"#,
        "",
        &diagnostic(
            1,
            16,
            "expected bool, found str",
            r#"print(true and "yes");"#,
            5,
        ),
    );
    expect_runtime_error(
        "print(false or nil);",
        "",
        &diagnostic(1, 16, "expected bool, found nil", "print(false or nil);", 3),
    );
}

#[test]
fn the_right_operand_of_a_decided_logical_operator_is_not_evaluated() {
    // No error: the right side is never looked at.
    expect_output("print(false and 1, true or 1);", "false true\n");
    expect_output(
        "print(false and 1 / 0 == 0, true or 1 / 0 == 0);",
        "false true\n",
    );
}

// ----- runtime errors: arithmetic -----------------------------------------

#[test]
fn integer_overflow_is_an_error() {
    let cases = [
        "9223372036854775807 + 1",
        "-9223372036854775807 - 2",
        "3037000500 * 3037000500",
        "4611686018427387904 * 2",
        "-(-9223372036854775807 - 1)",
        "(-9223372036854775807 - 1) / -1",
    ];
    for expression in cases {
        let script = format!("print({expression});");
        expect_runtime_error(
            &script,
            "",
            &diagnostic(
                1,
                7,
                "integer overflow",
                &script,
                expression.chars().count(),
            ),
        );
    }
}

#[test]
fn division_by_zero_is_an_error() {
    expect_runtime_error(
        "print(1 / 0);",
        "",
        &diagnostic(1, 7, "division by zero", "print(1 / 0);", 5),
    );
    expect_runtime_error(
        "print(1 % 0);",
        "",
        &diagnostic(1, 7, "division by zero", "print(1 % 0);", 5),
    );
    expect_runtime_error(
        "print(0 / (2 - 2));",
        "",
        &diagnostic(1, 7, "division by zero", "print(0 / (2 - 2));", 11),
    );
}

// ----- runtime errors: what else happens ----------------------------------

#[test]
fn output_before_a_runtime_error_is_kept() {
    expect_runtime_error(
        "print(\"one\");\nprint(\"two\", 2 / 0);\nprint(\"three\");\n",
        "one\n",
        &diagnostic(2, 14, "division by zero", "print(\"two\", 2 / 0);", 5),
    );
}

#[test]
fn operands_are_evaluated_from_left_to_right() {
    // Both operands fail; the left one fails first.
    expect_runtime_error(
        r#"print(1 / 0 + ("a" - 1));"#,
        "",
        &diagnostic(1, 7, "division by zero", r#"print(1 / 0 + ("a" - 1));"#, 5),
    );
    expect_runtime_error(
        r#"print(("a" - 1) + 1 / 0);"#,
        "",
        &diagnostic(
            1,
            8,
            "cannot apply `-` to str and int",
            r#"print(("a" - 1) + 1 / 0);"#,
            7,
        ),
    );
}

// ----- spans ---------------------------------------------------------------

#[test]
fn a_span_covers_the_parentheses_of_its_operands() {
    expect_runtime_error(
        r#"print((1 + 2) * "a");"#,
        "",
        &diagnostic(
            1,
            7,
            "cannot apply `*` to int and str",
            r#"print((1 + 2) * "a");"#,
            13,
        ),
    );
    expect_runtime_error(
        r#"print("a" * (1 + 2));"#,
        "",
        &diagnostic(
            1,
            7,
            "cannot apply `*` to str and int",
            r#"print("a" * (1 + 2));"#,
            13,
        ),
    );
}

#[test]
fn a_span_does_not_cover_parentheses_around_the_expression_itself() {
    expect_runtime_error(
        "print(((1 / 0)));",
        "",
        &diagnostic(1, 9, "division by zero", "print(((1 / 0)));", 5),
    );
    expect_runtime_error(
        "print(not (1));",
        "",
        &diagnostic(1, 12, "expected bool, found int", "print(not (1));", 1),
    );
}

#[test]
fn a_span_over_several_lines_is_marked_to_the_end_of_its_first_line() {
    expect_runtime_error(
        "print(1 +\n      \"a\");\n",
        "",
        &diagnostic(1, 7, "cannot apply `+` to int and str", "print(1 +", 3),
    );
}

#[test]
fn a_runtime_error_exits_with_1_and_a_syntax_error_with_2() {
    assert_eq!(
        run_script("print(1 / 0);").code,
        Some(1),
        "a runtime error must exit with status 1"
    );
    assert_eq!(
        run_script("print(1 / );").code,
        Some(COMPILE_ERROR),
        "a syntax error must exit with status 2"
    );
}
