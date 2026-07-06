pub mod error;
mod expr;
mod stmt;

use std::collections::HashSet;

use cao_lang::compiler::{CaoProgram, Function, Module};
use tree_sitter::{Node, Tree};

pub use error::LowerError;

/// Per-function-conversion state threaded through statement/expression conversion.
///
/// `tmp_counter` is reset for each top-level `function_definition` and keeps counting through
/// any closures nested inside it, so synthesized temp variable names (e.g. for desugared table
/// literals) never collide within one function.
pub(crate) struct FnCtx<'a> {
    pub(crate) source: &'a str,
    pub(crate) known_functions: &'a HashSet<String>,
    tmp_counter: u64,
}

impl<'a> FnCtx<'a> {
    fn new(source: &'a str, known_functions: &'a HashSet<String>) -> Self {
        Self {
            source,
            known_functions,
            tmp_counter: 0,
        }
    }

    pub(crate) fn fresh_tmp(&mut self) -> String {
        let name = format!("__caolc_table_tmp_{}", self.tmp_counter);
        self.tmp_counter += 1;
        name
    }
}

pub(crate) fn text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes())
        .expect("grammar guarantees valid utf8 slices")
}

pub(crate) fn unsupported(node: Node) -> LowerError {
    LowerError::UnsupportedNode {
        kind: node.kind(),
        start: node.start_byte(),
        end: node.end_byte(),
    }
}

pub fn tree_to_program(tree: &Tree, source: &str) -> Result<CaoProgram, LowerError> {
    convert_definitions(tree.root_node(), source)
}

/// Shared by `source_file` and `module_definition` bodies -- both are just a sequence of
/// `function_definition | import_statement | module_definition` children (a `module_definition`
/// additionally has a `name: identifier` field child, which is simply ignored here since its
/// node kind never collides with the three definition kinds).
fn convert_definitions(node: Node, source: &str) -> Result<Module, LowerError> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();

    let known_functions: HashSet<String> = children
        .iter()
        .filter(|n| n.kind() == "function_definition")
        .map(|n| function_definition_name(*n, source))
        .collect();

    let mut functions = Vec::new();
    let mut imports = Vec::new();
    let mut submodules = Vec::new();

    for child in children {
        match child.kind() {
            "function_definition" => functions.push(convert_function_definition(
                child,
                source,
                &known_functions,
            )?),
            "import_statement" => imports.push(convert_import(child, source)),
            "module_definition" => submodules.push(convert_module_definition(child, source)?),
            "identifier" => {} // a module_definition's own `name` field, nothing to do here
            _ => return Err(unsupported(child)),
        }
    }

    Ok(Module {
        submodules,
        functions,
        imports,
    })
}

fn function_definition_name(node: Node, source: &str) -> String {
    let name_node = node
        .named_child(0)
        .expect("function_definition's first named child is its identifier");
    text(name_node, source).to_string()
}

fn convert_function_definition(
    node: Node,
    source: &str,
    known_functions: &HashSet<String>,
) -> Result<(String, Function), LowerError> {
    let name = function_definition_name(node, source);
    let params_node = node
        .named_child(1)
        .expect("function_definition's second named child is its parameter_list");
    let block_node = node
        .named_child(2)
        .expect("function_definition's third named child is its block");

    let mut function = Function::default();
    let mut pcursor = params_node.walk();
    for param in params_node.named_children(&mut pcursor) {
        function = function.with_arg(text(param, source));
    }

    let mut ctx = FnCtx::new(source, known_functions);
    let cards = stmt::convert_block(block_node, &mut ctx)?;
    function = function.with_cards(cards);

    Ok((name, function))
}

fn convert_import(node: Node, source: &str) -> String {
    let path_node = node
        .child_by_field_name("path")
        .expect("import_statement has a path field");
    let mut cursor = path_node.walk();
    path_node
        .named_children(&mut cursor)
        .map(|n| text(n, source))
        .collect::<Vec<_>>()
        .join(".")
}

fn convert_module_definition(node: Node, source: &str) -> Result<(String, Module), LowerError> {
    let name_node = node
        .child_by_field_name("name")
        .expect("module_definition has a name field");
    let name = text(name_node, source).to_string();
    let module = convert_definitions(node, source)?;
    Ok((name, module))
}
