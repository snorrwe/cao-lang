/**
 * @file CaoLang grammar for tree-sitter
 * @author Daniel Kiss <littlesnorrboy@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "cao_lang",

  rules: {
    // TODO: add the actual grammar rules
    source_file: ($) => repeat($._definition),
    _definition: ($) =>
      choice(
        $.function_definition,
        // TODO: imports, submodules
      ),
    function_definition: ($) =>
      seq("fn", $.identifier, $.parameter_list, $.block),
    parameter_list: ($) =>
      seq(
        "(",
        // TODO: parameters
        ")",
      ),

    block: ($) => seq("{", repeat($._statement), "}"),
    _statement: ($) =>
      choice(
        $.return_statement,
        // TODO: other kinds of statements
      ),
    return_statement: ($) => seq("return", $.expression, ";"),
    expression: ($) =>
      choice(
        $.identifier,
        $.integer,
        // TODO: other kinds of expressions
      ),

    identifier: ($) => /[a-z]+/,

    integer: ($) => /\d+/,
  },
});
