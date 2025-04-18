use std::collections::VecDeque;

use hcl_edit::{expr::Object, structure::Body};

use crate::{
    hcl::{
        expr::{Expression, ObjectKey},
        structure::{Block, BlockLabel},
        template::{Element, StringTemplate},
    },
    types::EvaluatableInput,
};

use crate::{helpers::fs::FileLocation, types::diagnostics::Diagnostic};

#[derive(Debug, Clone)]
pub enum StringExpression {
    Literal(String),
    Template(StringTemplate),
}

#[derive(Debug)]
pub enum VisitorError {
    MissingField(String),
    MissingAttribute(String),
    TypeMismatch(String, String),
    TypeExpected(String),
}

pub fn visit_label(index: usize, name: &str, block: &Block) -> Result<String, VisitorError> {
    let label = block.labels.get(index).ok_or(VisitorError::MissingField(name.to_string()))?;
    match label {
        BlockLabel::String(literal) => Ok(literal.to_string()),
        BlockLabel::Ident(_e) => Err(VisitorError::TypeMismatch("string".into(), name.to_string())),
    }
}

pub fn visit_optional_string_attribute(
    field_name: &str,
    block: &Block,
) -> Result<Option<StringExpression>, VisitorError> {
    let Some(attribute) = block.body.get_attribute(field_name) else {
        return Ok(None);
    };

    match attribute.value.clone() {
        Expression::String(value) => Ok(Some(StringExpression::Literal(value.to_string()))),
        Expression::StringTemplate(template) => Ok(Some(StringExpression::Template(template))),
        _ => Err(VisitorError::TypeExpected("string".into())),
    }
}

pub fn visit_required_string_literal_attribute(
    field_name: &str,
    block: &Block,
) -> Result<String, VisitorError> {
    let Some(attribute) = block.body.get_attribute(field_name) else {
        return Err(VisitorError::MissingAttribute(field_name.to_string()));
    };

    match attribute.value.clone() {
        Expression::String(value) => Ok(value.to_string()),
        _ => Err(VisitorError::TypeExpected("string".into())),
    }
}

pub fn visit_optional_untyped_attribute(field_name: &str, block: &Block) -> Option<Expression> {
    let Some(attribute) = block.body.get_attribute(field_name) else {
        return None;
    };
    Some(attribute.value.clone())
}

pub fn get_object_expression_key(obj: &Object, key: &str) -> Option<hcl_edit::expr::ObjectValue> {
    obj.into_iter()
        .find(|(k, _)| k.as_ident().and_then(|i| Some(i.as_str().eq(key))).unwrap_or(false))
        .map(|(_, v)| v)
        .cloned()
}

pub fn build_diagnostics_for_unused_fields(
    fields_names: Vec<&str>,
    block: &Block,
    location: &FileLocation,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    for attr in block.body.attributes().into_iter() {
        if fields_names.contains(&attr.key.as_str()) {
            continue;
        }
        diagnostics.push(
            Diagnostic::error_from_string(format!("'{}' field is unused", attr.key.as_str()))
                .location(&location),
        )
    }
    diagnostics
}

/// Takes an HCL block and traverses all inner expressions and blocks,
/// recursively collecting all the references to constructs (variables and traversals).
pub fn collect_constructs_references_from_block<'a, T: EvaluatableInput>(
    block: &Block,
    input: Option<&'a T>,
    dependencies: &mut Vec<(Option<&'a T>, Expression)>,
) {
    for attribute in block.body.attributes() {
        let expr = attribute.value.clone();
        let mut references = vec![];
        collect_constructs_references_from_expression(&expr, input, &mut references);
        dependencies.append(&mut references);
    }
    for block in block.body.blocks() {
        collect_constructs_references_from_block(block, input, dependencies);
    }
}

/// Takes an HCL expression and boils it down to a Variable or Traversal expression,
/// pushing those low level expressions to the dependencies vector. For example:
/// ```hcl
/// val = [variable.a, variable.b]
/// ```
/// will push `variable.a` and `variable.b` to the dependencies vector.
pub fn collect_constructs_references_from_expression<'a, T: EvaluatableInput>(
    expr: &Expression,
    input: Option<&'a T>,
    dependencies: &mut Vec<(Option<&'a T>, Expression)>,
) {
    match expr {
        Expression::Variable(_) => {
            dependencies.push((input, expr.clone()));
        }
        Expression::Array(elements) => {
            for element in elements.iter() {
                collect_constructs_references_from_expression(element, input, dependencies);
            }
        }
        Expression::BinaryOp(op) => {
            collect_constructs_references_from_expression(&op.lhs_expr, input, dependencies);
            collect_constructs_references_from_expression(&op.rhs_expr, input, dependencies);
        }
        Expression::Bool(_)
        | Expression::Null(_)
        | Expression::Number(_)
        | Expression::String(_) => return,
        Expression::Conditional(cond) => {
            collect_constructs_references_from_expression(&cond.cond_expr, input, dependencies);
            collect_constructs_references_from_expression(&cond.false_expr, input, dependencies);
            collect_constructs_references_from_expression(&cond.true_expr, input, dependencies);
        }
        Expression::ForExpr(for_expr) => {
            collect_constructs_references_from_expression(
                &for_expr.value_expr,
                input,
                dependencies,
            );
            if let Some(ref key_expr) = for_expr.key_expr {
                collect_constructs_references_from_expression(&key_expr, input, dependencies);
            }
            if let Some(ref cond) = for_expr.cond {
                collect_constructs_references_from_expression(&cond.expr, input, dependencies);
            }
        }
        Expression::FuncCall(expr) => {
            for arg in expr.args.iter() {
                collect_constructs_references_from_expression(arg, input, dependencies);
            }
        }
        Expression::HeredocTemplate(expr) => {
            for element in expr.template.iter() {
                match element {
                    Element::Directive(_) | Element::Literal(_) => {}
                    Element::Interpolation(interpolation) => {
                        collect_constructs_references_from_expression(
                            &interpolation.expr,
                            input,
                            dependencies,
                        );
                    }
                }
            }
        }
        Expression::Object(obj) => {
            for (k, v) in obj.iter() {
                match k {
                    ObjectKey::Expression(expr) => {
                        collect_constructs_references_from_expression(&expr, input, dependencies);
                    }
                    ObjectKey::Ident(_) => {}
                }
                collect_constructs_references_from_expression(&v.expr(), input, dependencies);
            }
        }
        Expression::Parenthesis(expr) => {
            collect_constructs_references_from_expression(&expr.inner(), input, dependencies);
        }
        Expression::StringTemplate(template) => {
            for element in template.iter() {
                match element {
                    Element::Directive(_) | Element::Literal(_) => {}
                    Element::Interpolation(interpolation) => {
                        collect_constructs_references_from_expression(
                            &interpolation.expr,
                            input,
                            dependencies,
                        );
                    }
                }
            }
        }
        Expression::Traversal(traversal) => {
            let Expression::Variable(_) = traversal.expr else {
                return;
            };
            dependencies.push((input.clone(), expr.clone()));
        }
        Expression::UnaryOp(op) => {
            collect_constructs_references_from_expression(&op.expr, input, dependencies);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawHclContent(String);
impl RawHclContent {
    pub fn from_string(s: String) -> Self {
        RawHclContent(s)
    }
    pub fn from_file_location(file_location: &FileLocation) -> Result<Self, Diagnostic> {
        file_location
            .read_content_as_utf8()
            .map_err(|e| {
                Diagnostic::error_from_string(format!("{}", e.to_string())).location(&file_location)
            })
            .map(|s| RawHclContent(s))
    }

    pub fn into_blocks(&self) -> Result<VecDeque<Block>, Diagnostic> {
        let content = crate::hcl::parser::parse_body(&self.0).map_err(|e| {
            Diagnostic::error_from_string(format!("parsing error: {}", e.to_string()))
        })?;
        Ok(content.into_blocks().into_iter().collect::<VecDeque<Block>>())
    }

    pub fn into_block_instance(&self) -> Result<Block, Diagnostic> {
        let mut blocks = self.into_blocks()?;
        if blocks.len() != 1 {
            return Err(Diagnostic::error_from_string(
                "expected exactly one block instance".into(),
            ));
        }
        Ok(blocks.pop_front().unwrap())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Diagnostic> {
        let mut bytes = vec![0u8; 2 * self.0.len()];
        crate::hex::encode_to_slice(self.0.clone(), &mut bytes).map_err(|e| {
            Diagnostic::error_from_string(format!("failed to encode raw content: {e}"))
        })?;
        Ok(bytes)
    }
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
    pub fn from_block(block: &Block) -> Self {
        RawHclContent::from_string(
            Body::builder().block(block.clone()).build().to_string().trim().to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::types::commands::CommandInput;

    use super::*;

    #[test]
    fn test_block_to_raw_hcl() {
        let addon_block_str = r#"
            addon "evm" {
                test = "hi"
                chain_id = input.chain_id
                rpc_api_url = input.rpc_api_url
            }
        "#
        .trim();

        let signer_block_str = r#"
        signer "deployer" "evm::web_wallet" {
            expected_address = "0xCe246168E59dd8e28e367BB49b38Dc621768F425"
        }
        "#
        .trim();

        let runbook_block_str = r#"
            runbook "test" {
                location = "./embedded-runbook.json"
                chain_id = input.chain_id
                rpc_api_url = input.rpc_api_url
                deployer = signer.deployer
            }
        "#
        .trim();

        let output_block_str = r#"
            output "contract_address1" {
                value = runbook.test.action.deploy1.contract_address
            }
        "#
        .trim();

        let input = format!(
            r#"
        {addon_block_str}

        {signer_block_str}

        {runbook_block_str}

        {output_block_str}
        "#
        );

        let raw_hcl = RawHclContent::from_string(input.trim().to_string());
        let blocks = raw_hcl.into_blocks().unwrap();
        assert_eq!(blocks.len(), 4);
        let addon_block = RawHclContent::from_block(&blocks[0]).to_string();
        assert_eq!(addon_block, addon_block_str);
        let signer_block = RawHclContent::from_block(&blocks[1]).to_string();
        assert_eq!(signer_block, signer_block_str);
        let runbook_block = RawHclContent::from_block(&blocks[2]).to_string();
        assert_eq!(runbook_block, runbook_block_str);
        let output_block = RawHclContent::from_block(&blocks[3]).to_string();
        assert_eq!(output_block, output_block_str);
    }

    #[test]
    fn test_collect_constructs_references_from_block() {
        let input = r#"
            runbook "test" {
                location = "./embedded-runbook.json"
                chain_id = input.chain_id
                rpc_api_url = input.rpc_api_url
                deployer = signer.deployer
                arr = [variable.a, variable.b]
                my_map {
                    key1 = variable.a
                    my_inner_map {
                        key2 = variable.b
                    }
                }
            }
        "#;

        let raw_hcl = RawHclContent::from_string(input.trim().to_string());
        let block = raw_hcl.into_block_instance().unwrap();
        let mut dependencies = vec![];
        collect_constructs_references_from_block(&block, None::<&CommandInput>, &mut dependencies);

        assert_eq!(dependencies.len(), 7);
    }

    #[test]
    fn test_collect_constructs_references_expression() {
        let input = r#"
            runbook "test" {
                location = "./embedded-runbook.json"
                chain_id = input.chain_id
                rpc_api_url = input.rpc_api_url
                deployer = signer.deployer
                arr = [variable.a, variable.b]
                my_map {
                    key1 = variable.a
                    my_inner_map {
                        key2 = variable.b
                    }
                }
            }
        "#;

        let raw_hcl = RawHclContent::from_string(input.trim().to_string());
        let block = raw_hcl.into_block_instance().unwrap();
        let attribute = block.body.get_attribute("chain_id").unwrap();

        let mut dependencies = vec![];
        collect_constructs_references_from_expression(
            &attribute.value,
            None::<&CommandInput>,
            &mut dependencies,
        );

        assert_eq!(dependencies.len(), 1);
    }
}
