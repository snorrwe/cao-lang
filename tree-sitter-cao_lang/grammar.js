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
        $.import_statement,
        $.module_definition,
      ),
    import_statement: ($) => seq("import", field("path", $.dotted_path), ";"),
    dotted_path: ($) => seq($.identifier, repeat(seq(".", $.identifier))),

    module_definition: ($) =>
      seq("mod", field("name", $.identifier), "{", repeat($._definition), "}"),

    function_definition: ($) =>
      seq("fn", $.identifier, $.parameter_list, $.block),
    parameter_list: ($) => seq("(", commaSep($.identifier), ")"),

    block: ($) => seq("{", repeat($._statement), "}"),
    _statement: ($) =>
      choice(
        $.return_statement,
        $.global_var_statement,
        $.local_var_statement,
        $.set_property_statement,
        $.expression_statement,
        $.while_statement,
        $.repeat_statement,
        $.foreach_statement,
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
    local_var_statement: ($) =>
      seq(
        field("name", $.identifier),
        "=",
        field("value", $.expression),
        ";",
      ),
    set_property_statement: ($) =>
      seq(
        field("target", $.postfix_expression),
        "=",
        field("value", $.expression),
        ";",
      ),
    expression_statement: ($) => seq($.expression, ";"),

    while_statement: ($) =>
      seq("while", field("condition", $.expression), field("body", $.block)),

    repeat_statement: ($) =>
      seq(
        "repeat",
        field("count", $.expression),
        optional(seq("as", field("var", $.identifier))),
        field("body", $.block),
      ),

    foreach_statement: ($) =>
      seq(
        "foreach",
        field("bindings", $.foreach_bindings),
        "in",
        field("iterable", $.expression),
        field("body", $.block),
      ),

    foreach_bindings: ($) =>
      choice(
        field("v", $.identifier),
        seq(field("k", $.identifier), ",", field("v", $.identifier)),
        seq(
          field("i", $.identifier),
          ",",
          field("k", $.identifier),
          ",",
          field("v", $.identifier),
        ),
      ),

    expression: ($) =>
      choice(
        $.identifier,
        $.integer,
        $.float,
        $.string,
        $.call_expression,
        $.postfix_expression,
        $.binary_expression,
        $.if_expression,
        $.array_expression,
        $.table_expression,
        $.closure_expression,
      ),

    call_expression: ($) =>
      seq(field("function", $._callee), "(", commaSep($.expression), ")"),

    _callee: ($) => choice($.identifier, $.postfix_expression),

    postfix_expression: ($) =>
      prec(4, seq(
        choice($.identifier, $.call_expression),
        repeat1(choice(
          seq(".", field("property", $.identifier)),
          seq("[", field("index", $.expression), "]"),
        )),
      )),

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
        optional(seq("else", field("alternative", $.expression_block))),
      ),

    expression_block: ($) => seq("{", $.expression, "}"),

    array_expression: ($) => seq("[", commaSep($.expression), "]"),

    table_expression: ($) => seq("#{", commaSep($.table_field), "}"),
    table_field: ($) =>
      seq(
        field("key", choice($.identifier, $.string)),
        ":",
        field("value", $.expression),
      ),

    closure_expression: ($) =>
      prec.right(seq(
        field("parameters", $.parameter_list),
        "=>",
        field("body", choice($.expression, $.block)),
      )),

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    integer: ($) => /-?\d+/,

    float: ($) => /-?\d+\.\d+/,

    string: ($) =>
      seq('"', repeat(choice($.string_content, $.escape_sequence)), '"'),
    string_content: ($) => token.immediate(prec(1, /[^"\\]+/)),
    escape_sequence: ($) =>
      token.immediate(
        seq("\\", choice(/["\\nrt0]/, seq("u", "{", /[0-9a-fA-F]+/, "}"))),
      ),
  },
});
