use cao_lang::compiler::{Card, CardBody, ForEach};
use tree_sitter::Node;

use super::error::LowerError;
use super::expr::{convert_expression, convert_postfix_assignment_target};
use super::{FnCtx, text, unsupported};

pub(crate) fn convert_block(node: Node, ctx: &mut FnCtx) -> Result<Vec<Card>, LowerError> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();
    children
        .into_iter()
        .map(|n| convert_statement(n, ctx))
        .collect()
}

/// Loop bodies (`while`/`repeat`/`foreach`) hold a single `Card`, not a `Vec<Card>` like a
/// function's top-level cards -- collapse a single-statement block directly, otherwise group
/// via `CompositeCard` (sub-cards process in plain sequence with no cleanup between them, so a
/// multi-statement body behaves correctly as one opaque unit).
pub(crate) fn convert_block_as_single_card(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    let mut cards = convert_block(node, ctx)?;
    if cards.len() == 1 {
        Ok(cards.pop().unwrap())
    } else {
        Ok(Card::composite_card("block", cards))
    }
}

fn convert_statement(node: Node, ctx: &mut FnCtx) -> Result<Card, LowerError> {
    match node.kind() {
        "return_statement" => {
            let expr = node
                .named_child(0)
                .expect("return_statement has an expression child");
            Ok(Card::return_card(convert_expression(expr, ctx)?))
        }
        "global_var_statement" => {
            let name = text(node.child_by_field_name("name").unwrap(), ctx.source);
            let value = convert_expression(node.child_by_field_name("value").unwrap(), ctx)?;
            Ok(Card::set_global_var(name, value))
        }
        "local_var_statement" => {
            let name = text(node.child_by_field_name("name").unwrap(), ctx.source);
            let value = convert_expression(node.child_by_field_name("value").unwrap(), ctx)?;
            Ok(Card::set_var(name, value))
        }
        "set_property_statement" => {
            let target = node
                .child_by_field_name("target")
                .expect("set_property_statement has a target field");
            let value = convert_expression(node.child_by_field_name("value").unwrap(), ctx)?;
            let (table, key) = convert_postfix_assignment_target(target, ctx)?;
            Ok(Card::set_property(value, table, key))
        }
        "expression_statement" => {
            let expr = node
                .named_child(0)
                .expect("expression_statement has an expression child");
            // Leftover stack residue from a discarded expression result is fine -- the VM
            // clears the whole stack frame down to the call's start offset on function return,
            // so no explicit "discard" card is needed here.
            convert_expression(expr, ctx)
        }
        "while_statement" => {
            let cond = convert_expression(node.child_by_field_name("condition").unwrap(), ctx)?;
            let body =
                convert_block_as_single_card(node.child_by_field_name("body").unwrap(), ctx)?;
            Ok(CardBody::While(Box::new([cond, body])).into())
        }
        "repeat_statement" => {
            let count = convert_expression(node.child_by_field_name("count").unwrap(), ctx)?;
            let var = node
                .child_by_field_name("var")
                .map(|n| text(n, ctx.source).to_string());
            let body =
                convert_block_as_single_card(node.child_by_field_name("body").unwrap(), ctx)?;
            Ok(Card::repeat(count, var, body))
        }
        "foreach_statement" => {
            let bindings = node
                .child_by_field_name("bindings")
                .expect("foreach_statement has a bindings field");
            let i = bindings
                .child_by_field_name("i")
                .map(|n| text(n, ctx.source).to_string());
            let k = bindings
                .child_by_field_name("k")
                .map(|n| text(n, ctx.source).to_string());
            let v = text(
                bindings
                    .child_by_field_name("v")
                    .expect("foreach_bindings always has a v field"),
                ctx.source,
            )
            .to_string();
            let iterable =
                convert_expression(node.child_by_field_name("iterable").unwrap(), ctx)?;
            let body =
                convert_block_as_single_card(node.child_by_field_name("body").unwrap(), ctx)?;
            Ok(ForEach {
                i,
                k,
                v: Some(v),
                iterable: Box::new(iterable),
                body: Box::new(body),
            }
            .into())
        }
        _ => Err(unsupported(node)),
    }
}
