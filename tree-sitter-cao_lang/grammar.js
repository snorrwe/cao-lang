/**
 * @file CaoLang grammar for tree-sitter
 * @author Daniel Kiss <littlesnorrboy@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

function commaSep(rule) {
  return optional(seq(rule, repeat(seq(",", rule))));
}

export default grammar({
  name: "cao_lang",

  rules: {
    source_file: ($) => repeat($._definition),
    _definition: ($) =>
      choice(
        $.function_definition,
        // TODO: imports, submodules
      ),
    function_definition: ($) =>
      seq("fn", $.identifier, $.parameter_list, $.block),
    parameter_list: ($) => seq("(", commaSep($.identifier), ")"),

    block: ($) => seq("{", repeat($._statement), "}"),
    // TODO: local `NAME = expr;` (SetVar), while/repeat/foreach, arrays, closures
    _statement: ($) =>
      choice(
        $.return_statement,
        $.global_var_statement,
        // TODO: other kinds of statements
      ),
    return_statement: ($) => seq("return", $.expression, ";"),
    global_var_statement: ($) =>
      seq(
        "global",
        field("name", $.identifier),
        "=",
        field("value", $.expression),
        ";",
      ),

    expression: ($) =>
      choice(
        $.identifier,
        $.integer,
        $.call_expression,
        $.binary_expression,
        $.if_expression,
        // TODO: string/float literals, arrays, dotted property access, closures
      ),

    call_expression: ($) =>
      seq(field("function", $.identifier), "(", commaSep($.expression), ")"),

    binary_expression: ($) => {
      const table = [
        [1, choice("<=", "<", "==", "!=")],
        [2, choice("+", "-")],
        [3, choice("*", "/")],
      ];

      return choice(
        ...table.map(([precedence, operator]) =>
          prec.left(
            precedence,
            seq(
              field("left", $.expression),
              field("operator", operator),
              field("right", $.expression),
            ),
          ),
        ),
      );
    },

    if_expression: ($) =>
      seq(
        "if",
        field("condition", $.expression),
        field("consequence", $.expression_block),
        "else",
        field("alternative", $.expression_block),
      ),

    expression_block: ($) => seq("{", $.expression, "}"),

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    integer: ($) => /\d+/,
  },
});
