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
    source_file: $ => "hello"
  }
});
