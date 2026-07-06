use cao_lang::compiler::{Card, CardBody, Function};
use tree_sitter::Node;

use super::error::LowerError;
use super::{FnCtx, stmt, text, unsupported};

/// `node` is the `expression` wrapper node (single named child = the real node).
pub(crate) fn convert_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let inner = node
        .named_child(0)
        .expect("`expression` wraps exactly one node");
    convert_expression_inner(inner, ctx)
}

fn convert_expression_inner(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    match node.kind() {
        "identifier" => Ok(Card::read_var(text(node, ctx.source))),
        "integer" => {
            let t = text(node, ctx.source);
            let v: i64 = t
                .parse()
                .map_err(|_| LowerError::BadInteger { text: t.to_string() })?;
            Ok(Card::scalar_int(v))
        }
        "float" => {
            let t = text(node, ctx.source);
            let v: f64 = t
                .parse()
                .map_err(|_| LowerError::BadFloat { text: t.to_string() })?;
            Ok(CardBody::ScalarFloat(v).into())
        }
        "string" => Ok(Card::string_card(decode_string(node, ctx.source))),
        "call_expression" => convert_call_expression(node, ctx),
        "postfix_expression" => convert_postfix_expression(node, ctx),
        "binary_expression" => convert_binary_expression(node, ctx),
        "if_expression" => convert_if_expression(node, ctx),
        "array_expression" => convert_array_expression(node, ctx),
        "table_expression" => convert_table_expression(node, ctx),
        "closure_expression" => convert_closure_expression(node, ctx),
        _ => Err(unsupported(node)),
    }
}

fn decode_string(node: Node, source: &str) -> String {
    let mut out = String::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "string_content" => out.push_str(text(child, source)),
            "escape_sequence" => out.push_str(&decode_escape(text(child, source))),
            other => unreachable!("string only contains string_content/escape_sequence, got {other}"),
        }
    }
    out
}

fn decode_escape(raw: &str) -> String {
    let bytes = raw.as_bytes();
    match bytes[1] {
        b'"' => "\"".to_string(),
        b'\\' => "\\".to_string(),
        b'n' => "\n".to_string(),
        b'r' => "\r".to_string(),
        b't' => "\t".to_string(),
        b'0' => "\0".to_string(),
        b'u' => {
            let hex = &raw[3..raw.len() - 1]; // strip leading `\u{` and trailing `}`
            let code = u32::from_str_radix(hex, 16).expect("grammar guarantees valid hex digits");
            char::from_u32(code)
                .expect("grammar guarantees a valid unicode scalar value")
                .to_string()
        }
        other => unreachable!("grammar guarantees a known escape sequence, got \\{}", other as char),
    }
}

fn convert_binary_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let left = convert_expression(node.child_by_field_name("left").unwrap(), ctx)?;
    let op_node = node
        .child_by_field_name("operator")
        .expect("binary_expression has an operator field");
    let right = convert_expression(node.child_by_field_name("right").unwrap(), ctx)?;
    let pair = Box::new([left, right]);
    Ok(match text(op_node, ctx.source) {
        "+" => CardBody::Add(pair).into(),
        "-" => CardBody::Sub(pair).into(),
        "*" => CardBody::Mul(pair).into(),
        "/" => CardBody::Div(pair).into(),
        "<" => CardBody::Less(pair).into(),
        "<=" => CardBody::LessOrEq(pair).into(),
        "==" => CardBody::Equals(pair).into(),
        "!=" => CardBody::NotEquals(pair).into(),
        other => unreachable!("grammar guarantees a known binary operator, got {other}"),
    })
}

fn convert_if_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let cond = convert_expression(node.child_by_field_name("condition").unwrap(), ctx)?;
    let then = convert_expression_block(node.child_by_field_name("consequence").unwrap(), ctx)?;
    match node.child_by_field_name("alternative") {
        Some(alt_node) => {
            let els = convert_expression_block(alt_node, ctx)?;
            Ok(CardBody::IfElse(Box::new([cond, then, els])).into())
        }
        None => Ok(CardBody::IfTrue(Box::new([cond, then])).into()),
    }
}

fn convert_expression_block(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let inner = node
        .named_child(0)
        .expect("expression_block wraps exactly one expression");
    convert_expression(inner, ctx)
}

fn convert_array_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let mut cursor = node.walk();
    let items: Vec<Node> = node.named_children(&mut cursor).collect();
    let cards = items
        .into_iter()
        .map(|n| convert_expression(n, ctx))
        .collect::<Result<_, _>>()?;
    Ok(CardBody::Array(cards).into())
}

fn convert_table_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let tmp = ctx.fresh_tmp();
    let mut cursor = node.walk();
    let fields: Vec<Node> = node.named_children(&mut cursor).collect();

    let mut cards = vec![Card::set_var(tmp.as_str(), CardBody::CreateTable)];
    for field in fields {
        let key_node = field
            .child_by_field_name("key")
            .expect("table_field has a key field");
        let key = match key_node.kind() {
            "identifier" => Card::string_card(text(key_node, ctx.source)),
            "string" => Card::string_card(decode_string(key_node, ctx.source)),
            other => unreachable!("table_field key must be identifier or string, got {other}"),
        };
        let value_node = field
            .child_by_field_name("value")
            .expect("table_field has a value field");
        let value = convert_expression(value_node, ctx)?;
        cards.push(Card::set_property(value, Card::read_var(tmp.as_str()), key));
    }
    cards.push(Card::read_var(tmp.as_str()));
    Ok(Card::composite_card("table_literal", cards))
}

fn convert_closure_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let params_node = node
        .child_by_field_name("parameters")
        .expect("closure_expression has a parameters field");
    let body_node = node
        .child_by_field_name("body")
        .expect("closure_expression has a body field");

    let mut function = Function::default();
    let mut pcursor = params_node.walk();
    for param in params_node.named_children(&mut pcursor) {
        function = function.with_arg(text(param, ctx.source));
    }

    let cards = match body_node.kind() {
        "block" => stmt::convert_block(body_node, ctx)?,
        // Arrow-body closures have no `return` keyword in this grammar, but every function
        // (closures included) implicitly returns Nil when it falls off the end of its cards
        // (see `compile_stage_2` unconditionally appending `ScalarNil; Return`) -- so the
        // arrow's value MUST be wrapped in an explicit `Return` card or it would be discarded.
        "expression" => vec![Card::return_card(convert_expression(body_node, ctx)?)],
        _ => return Err(unsupported(body_node)),
    };
    function = function.with_cards(cards);

    Ok(CardBody::Closure(Box::new(function)).into())
}

/// Splits a `postfix_expression`'s named children into its base node and the ordered list of
/// segment nodes. A segment node's kind alone tells us whether it's a `.property` (kind
/// `identifier`) or `[index]` (kind `expression`) access -- the two never collide.
fn postfix_children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn convert_postfix_base(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    match node.kind() {
        "identifier" => Ok(Card::read_var(text(node, ctx.source))),
        "call_expression" => convert_call_expression(node, ctx),
        _ => Err(unsupported(node)),
    }
}

pub(crate) fn convert_postfix_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let children = postfix_children(node);
    let mut acc = convert_postfix_base(children[0], ctx)?;
    for seg in &children[1..] {
        acc = match seg.kind() {
            "identifier" => Card::get_property(acc, Card::string_card(text(*seg, ctx.source))),
            "expression" => CardBody::Get(Box::new([acc, convert_expression(*seg, ctx)?])).into(),
            other => unreachable!("postfix segment must be identifier or expression, got {other}"),
        };
    }
    Ok(acc)
}

/// For `set_property_statement` targets: everything but the last postfix segment is the "table"
/// expression to read, and the last segment is the key/index being written.
pub(crate) fn convert_postfix_assignment_target(
    node: Node,
    ctx: &mut FnCtx,
) -> Result<(Card, Card), LowerError> {
    let children = postfix_children(node);
    debug_assert!(
        children.len() >= 2,
        "postfix_expression always has a base plus at least one segment"
    );
    let last_index = children.len() - 1;

    let mut acc = convert_postfix_base(children[0], ctx)?;
    for seg in &children[1..last_index] {
        acc = match seg.kind() {
            "identifier" => Card::get_property(acc, Card::string_card(text(*seg, ctx.source))),
            "expression" => CardBody::Get(Box::new([acc, convert_expression(*seg, ctx)?])).into(),
            other => unreachable!("postfix segment must be identifier or expression, got {other}"),
        };
    }

    let last = children[last_index];
    let key = match last.kind() {
        "identifier" => Card::string_card(text(last, ctx.source)),
        "expression" => convert_expression(last, ctx)?,
        other => unreachable!("postfix segment must be identifier or expression, got {other}"),
    };
    Ok((acc, key))
}

/// Callee text for a `call_expression`'s `function` field, and whether it's a dynamic (i.e.
/// currently unsupported) target: a dot-only chain of identifiers is resolved to a plain/dotted
/// name string; anything involving `[index]` or a call-as-base has no first-class-function-value
/// syntax in this grammar yet, so it's flagged as dynamic rather than silently mis-compiled.
fn callee_name(node: Node, ctx: &FnCtx) -> (Option<String>, bool) {
    match node.kind() {
        "identifier" => (Some(text(node, ctx.source).to_string()), false),
        "postfix_expression" => {
            let children = postfix_children(node);
            let base = children[0];
            if base.kind() != "identifier" {
                return (None, true);
            }
            let mut parts = vec![text(base, ctx.source).to_string()];
            for seg in &children[1..] {
                if seg.kind() != "identifier" {
                    return (None, true);
                }
                parts.push(text(*seg, ctx.source).to_string());
            }
            (Some(parts.join(".")), false)
        }
        other => unreachable!("call_expression's function field must be identifier or postfix_expression, got {other}"),
    }
}

pub(crate) fn convert_call_expression(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let callee = node
        .child_by_field_name("function")
        .expect("call_expression has a function field");

    let (name, dynamic) = callee_name(callee, ctx);
    if dynamic {
        return Err(LowerError::DynamicCallUnsupported {
            start: callee.start_byte(),
            end: callee.end_byte(),
        });
    }
    let name = name.expect("callee_name returns Some when not dynamic");

    let mut cursor = node.walk();
    let args: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "expression")
        .collect();
    let args: Vec<Card> = args
        .into_iter()
        .map(|n| convert_expression(n, ctx))
        .collect::<Result<_, _>>()?;

    if ctx.known_functions.contains(&name) {
        Ok(Card::call_function(name, args))
    } else {
        Ok(Card::call_native(name, args))
    }
}
