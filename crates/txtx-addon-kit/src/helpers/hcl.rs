use std::collections::VecDeque;

use hcl_edit::{
    structure::{Attribute, Body},
    Span,
};

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

fn visit_required_string_literal_attribute(
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
pub fn collect_constructs_referenced_by_construct<'a, T: EvaluatableInput>(
    construct: &RunbookConstruct,
    input: Option<&'a T>,
    dependencies: &mut Vec<(Option<&'a T>, Expression)>,
) {
    unimplemented!()
    // for attribute in construct.get_attributes() {
    //     let expr = attribute.get_value();
    //     let mut references = vec![];
    //     expr.collect_constructs_references_from_expression(input, &mut references);
    //     dependencies.append(&mut references);
    // }
    // for sub_construct in construct.get_sub_constructs() {
    //     collect_constructs_referenced_by_construct(sub_construct, input, dependencies);
    // }
}

/// Takes an HCL expression and boils it down to a Variable or Traversal expression,
/// pushing those low level expressions to the dependencies vector. For example:
/// ```hcl
/// val = [variable.a, variable.b]
/// ```
/// will push `variable.a` and `variable.b` to the dependencies vector.
pub fn collect_constructs_references_from_hcl_expression<'a, T: EvaluatableInput>(
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
                collect_constructs_references_from_hcl_expression(element, input, dependencies);
            }
        }
        Expression::BinaryOp(op) => {
            collect_constructs_references_from_hcl_expression(&op.lhs_expr, input, dependencies);
            collect_constructs_references_from_hcl_expression(&op.rhs_expr, input, dependencies);
        }
        Expression::Bool(_)
        | Expression::Null(_)
        | Expression::Number(_)
        | Expression::String(_) => return,
        Expression::Conditional(cond) => {
            collect_constructs_references_from_hcl_expression(&cond.cond_expr, input, dependencies);
            collect_constructs_references_from_hcl_expression(
                &cond.false_expr,
                input,
                dependencies,
            );
            collect_constructs_references_from_hcl_expression(&cond.true_expr, input, dependencies);
        }
        Expression::ForExpr(for_expr) => {
            collect_constructs_references_from_hcl_expression(
                &for_expr.value_expr,
                input,
                dependencies,
            );
            if let Some(ref key_expr) = for_expr.key_expr {
                collect_constructs_references_from_hcl_expression(&key_expr, input, dependencies);
            }
            if let Some(ref cond) = for_expr.cond {
                collect_constructs_references_from_hcl_expression(&cond.expr, input, dependencies);
            }
        }
        Expression::FuncCall(expr) => {
            for arg in expr.args.iter() {
                collect_constructs_references_from_hcl_expression(arg, input, dependencies);
            }
        }
        Expression::HeredocTemplate(expr) => {
            for element in expr.template.iter() {
                match element {
                    Element::Directive(_) | Element::Literal(_) => {}
                    Element::Interpolation(interpolation) => {
                        collect_constructs_references_from_hcl_expression(
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
                        collect_constructs_references_from_hcl_expression(
                            &expr,
                            input,
                            dependencies,
                        );
                    }
                    ObjectKey::Ident(_) => {}
                }
                collect_constructs_references_from_hcl_expression(&v.expr(), input, dependencies);
            }
        }
        Expression::Parenthesis(expr) => {
            collect_constructs_references_from_hcl_expression(&expr.inner(), input, dependencies);
        }
        Expression::StringTemplate(template) => {
            for element in template.iter() {
                match element {
                    Element::Directive(_) | Element::Literal(_) => {}
                    Element::Interpolation(interpolation) => {
                        collect_constructs_references_from_hcl_expression(
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
            collect_constructs_references_from_hcl_expression(&op.expr, input, dependencies);
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConstructAttributeRef<'a> {
    Hcl(&'a Attribute),
    Json(),
    Yaml(),
}

impl<'a> ConstructAttributeRef<'a> {
    pub fn get_value(&self) -> ConstructExpression {
        match &self {
            Self::Hcl(expr) => ConstructExpression::Hcl(expr.value.clone()),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConstructExpression {
    Hcl(Expression),
    Json(),
    Yaml(),
}

#[derive(Debug, Clone)]
pub enum ConstructExpressionRef<'a> {
    Hcl(&'a Expression),
    Json(),
    Yaml(),
}

impl<'a> From<ConstructExpressionRef<'a>> for ConstructExpression {
    fn from(value: ConstructExpressionRef<'a>) -> Self {
        match value {
            ConstructExpressionRef::Hcl(expr_ref) => ConstructExpression::Hcl(expr_ref.clone()),
            ConstructExpressionRef::Json() => ConstructExpression::Json(),
            ConstructExpressionRef::Yaml() => ConstructExpression::Yaml(),
        }
    }
}

impl<'a> From<&'a ConstructExpression> for ConstructExpressionRef<'a> {
    fn from(value: &'a ConstructExpression) -> Self {
        match value {
            ConstructExpression::Hcl(expr) => ConstructExpressionRef::Hcl(expr),
            ConstructExpression::Json() => ConstructExpressionRef::Json(),
            ConstructExpression::Yaml() => ConstructExpressionRef::Yaml(),
        }
    }
}

impl<'a> ConstructExpression {
    pub fn collect_constructs_references_from_expression<T: EvaluatableInput>(
        &self,
        input: Option<&'a T>,
        dependencies: &mut Vec<(Option<&'a T>, Expression)>,
    ) {
        match &self {
            Self::Hcl(expr) => {
                collect_constructs_references_from_hcl_expression(&expr, input, dependencies)
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_object_expression_key(&self, key: &str) -> Option<ConstructExpression> {
        match &self {
            Self::Hcl(expr) => {
                let object_expr = expr.as_object().unwrap();
                let expr_res = object_expr
                    .into_iter()
                    .find(|(k, _)| {
                        k.as_ident().and_then(|i| Some(i.as_str().eq(key))).unwrap_or(false)
                    })
                    .map(|(_, v)| v)
                    .cloned();
                match expr_res {
                    Some(expression) => Some(ConstructExpression::Hcl(expression.expr().clone())),
                    None => None,
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn expect_hcl_expression(&self) -> &Expression {
        let expr = match &self {
            Self::Hcl(src) => src,
            _ => unimplemented!(),
        };
        expr
    }
}

#[derive(Debug, Clone)]
pub enum RunbookConstruct {
    Hcl(Block),
    Json(),
    Yaml(),
}

#[derive(Debug, Clone)]
pub enum RunbookConstructRef<'a> {
    Hcl(&'a Block),
    Json(),
    Yaml(),
}

impl<'a> From<RunbookConstructRef<'a>> for RunbookConstruct {
    fn from(value: RunbookConstructRef<'a>) -> Self {
        match value {
            RunbookConstructRef::Hcl(block_ref) => RunbookConstruct::Hcl(block_ref.clone()),
            RunbookConstructRef::Json() => RunbookConstruct::Json(),
            RunbookConstructRef::Yaml() => RunbookConstruct::Yaml(),
        }
    }
}

impl<'a> From<&'a RunbookConstruct> for RunbookConstructRef<'a> {
    fn from(value: &'a RunbookConstruct) -> Self {
        match value {
            RunbookConstruct::Hcl(block) => RunbookConstructRef::Hcl(block),
            RunbookConstruct::Json() => RunbookConstructRef::Json(),
            RunbookConstruct::Yaml() => RunbookConstructRef::Yaml(),
        }
    }
}
impl<'a> RunbookConstructRef<'a> {
    pub fn get_sub_constructs(&'a self) -> Objects<'a> {
        match &self {
            Self::Hcl(block) => {
                block.body.blocks().map(|b: &Block| RunbookConstructRef::Hcl(b)).collect::<Vec<_>>()
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_attributes(&'a self) -> Vec<ConstructAttributeRef<'a>> {
        match &self {
            Self::Hcl(block) => {
                block.body.attributes().map(|b| ConstructAttributeRef::Hcl(b)).collect::<Vec<_>>()
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_sub_constructs_type(&'a self, sub_constructs_type: &'a str) -> Objects<'a> {
        match &self {
            Self::Hcl(block) => block
                .body
                .get_blocks(sub_constructs_type)
                .map(|b: &Block| RunbookConstructRef::Hcl(b))
                .collect::<Vec<_>>(),
            _ => unimplemented!(),
        }
    }

    pub fn get_expression_from_attribute(
        &'a self,
        attribute_name: &str,
    ) -> Option<ConstructExpressionRef<'a>> {
        match &self {
            Self::Hcl(block) => {
                let Some(attribute) = block.body.get_attribute(attribute_name) else {
                    return None;
                };
                Some(ConstructExpressionRef::Hcl(&attribute.value))
            }
            _ => unimplemented!(),
        }
    }

    // pub fn collect_constructs_referenced_by_construct<'b, T: EvaluatableInput>(
    //     &self,
    //     input: Option<&'b T>,
    //     dependencies: &'b mut Vec<(Option<&'b T>, Expression)>,
    // ) {
    //     for attribute in self.get_attributes() {
    //         let expr = attribute.get_value();
    //         let mut references = vec![];
    //         expr.collect_constructs_references_from_expression(input, &mut references);
    //         dependencies.append(&mut references);
    //     }
    //     for sub_constructs in self.get_sub_constructs() {
    //         sub_constructs.collect_constructs_referenced_by_construct(input, dependencies);
    //     }
    // }
}

pub type Objects<'a> = Vec<RunbookConstructRef<'a>>;

impl RunbookConstruct {
    pub fn get_construct_type(&self) -> &str {
        match &self {
            Self::Hcl(src) => src.ident.value().as_str(),
            _ => unimplemented!(),
        }
    }

    pub fn get_construct_instance_name(&self) -> Option<&str> {
        match &self {
            Self::Hcl(block) => {
                let Some(BlockLabel::String(name)) = block.labels.get(0) else { return None };
                Some(name.as_str())
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_attributes<'a>(&'a self) -> Vec<ConstructAttributeRef<'a>> {
        match &self {
            Self::Hcl(block) => {
                block.body.attributes().map(|b| ConstructAttributeRef::Hcl(b)).collect::<Vec<_>>()
            }
            _ => unimplemented!(),
        }
    }

    // pub fn get_attribute<'a>(&'a self, attribute_name: &str, expected_type: Option<Type>) -> Option<ConstructAttributeRef<'a>> {
    //     match &self {
    //         Self::Hcl(block) => {
    //             let value = block.body.get_attribute(attribute_name).map(|b| ConstructAttributeRef::Hcl(b))?;
    //             match expected_type {
    //                 None => Some(value),
    //                 Some(Type::String) =>
    //             }
    //         }
    //         _ => unimplemented!(),
    //     }
    // }

    pub fn get_attribute_stringified(&self, attribute_name: &str) -> Option<String> {
        match &self {
            Self::Hcl(block) => {
                block.body.get_attribute(attribute_name).map(|b| b.value.to_string())
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_command_instance_type(&self) -> Option<&str> {
        match &self {
            Self::Hcl(block) => {
                let Some(BlockLabel::String(name)) = block.labels.get(1) else { return None };
                Some(name.as_str())
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_sub_constructs<'a>(&'a self) -> Objects<'a> {
        match &self {
            Self::Hcl(block) => {
                block.body.blocks().map(|b: &Block| RunbookConstructRef::Hcl(b)).collect::<Vec<_>>()
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_sub_constructs_type<'a>(
        &'a self,
        sub_constructs_type: &'a str,
    ) -> Vec<RunbookConstruct> {
        match &self {
            Self::Hcl(block) => block
                .body
                .get_blocks(sub_constructs_type)
                .map(|b: &Block| RunbookConstruct::Hcl(b.clone()))
                .collect::<Vec<_>>(),
            _ => unimplemented!(),
        }
    }

    pub fn get_required_string_literal_from_attribute(
        &self,
        attribute_name: &str,
    ) -> Result<String, VisitorError> {
        match &self {
            Self::Hcl(block) => visit_required_string_literal_attribute(attribute_name, block),
            _ => unimplemented!(),
        }
    }

    pub fn get_expression_from_attribute<'a>(
        &'a self,
        attribute_name: &str,
    ) -> Option<ConstructExpression> {
        match &self {
            Self::Hcl(block) => {
                let Some(attribute) = block.body.get_attribute(attribute_name) else {
                    return None;
                };
                Some(ConstructExpression::Hcl(attribute.value.clone()))
            }
            _ => unimplemented!(),
        }
    }

    // pub fn collect_constructs_referenced_by_construct<T: EvaluatableInput>(
    //     &self,
    //     input: Option<&T>,
    //     dependencies: &mut Vec<(Option<&T>, Expression)>,
    // ) {
    //     for attribute in self.get_attributes() {
    //         let expr = attribute.get_value();
    //         let mut references = vec![];
    //         expr.collect_constructs_references_from_expression(input, &mut references);
    //         dependencies.append(&mut references);
    //     }
    //     for block in self.get_sub_constructs() {
    //         // block.coll
    //         // collect_constructs_referenced_by_construct(block, input, dependencies);
    //     }
    // }

    pub fn get_span(&self) -> Option<std::ops::Range<usize>> {
        match &self {
            Self::Hcl(block) => block.body.span(),
            _ => unimplemented!(),
        }
    }

    pub fn expect_hcl_block(&self) -> &Block {
        let block = match &self {
            Self::Hcl(src) => src,
            _ => unimplemented!(),
        };
        block
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunbookSource {
    Hcl(String),
    Json(String),
    Yaml(String),
}

impl RunbookSource {
    pub fn from_hcl_string(s: String) -> Self {
        Self::Hcl(s)
    }

    pub fn from_file_location(file_location: &FileLocation) -> Result<Self, Diagnostic> {
        file_location
            .read_content_as_utf8()
            .map_err(|e| {
                Diagnostic::error_from_string(format!("{}", e.to_string())).location(&file_location)
            })
            .map(|s| {
                let res = match file_location.get_file_name() {
                    Some(name) if name.ends_with(".tx") => Self::Hcl(s),
                    Some(name) if name.ends_with(".tx.hcl") => Self::Hcl(s),
                    Some(name) if name.ends_with(".tx.yaml") => Self::Hcl(s),
                    Some(name) if name.ends_with(".tx.json") => Self::Hcl(s),
                    _ => {
                        return Err(Diagnostic::error_from_string(format!(
                            "file format unknown (.tx, .tx.hcl, .tx.yaml, .tx.json)",
                        ))
                        .location(&file_location))
                    }
                };
                Ok(res)
            })?
    }

    pub fn into_constructs(&self) -> Result<VecDeque<RunbookConstruct>, Diagnostic> {
        let constructs = match &self {
            Self::Hcl(source) => {
                let content = crate::hcl::parser::parse_body(&source).map_err(|e| {
                    Diagnostic::error_from_string(format!("parsing error: {}", e.to_string()))
                })?;
                content
                    .into_blocks()
                    .into_iter()
                    .map(RunbookConstruct::Hcl)
                    .collect::<VecDeque<RunbookConstruct>>()
            }
            _ => unimplemented!(),
        };
        Ok(constructs)
    }

    pub fn into_construct(&self) -> Result<RunbookConstruct, Diagnostic> {
        let mut blocks = self.into_constructs()?;
        if blocks.len() != 1 {
            return Err(Diagnostic::error_from_string(
                "expected exactly one block instance".into(),
            ));
        }
        Ok(blocks.pop_front().unwrap())
    }

    // pub fn to_bytes(&self) -> Result<Vec<u8>, Diagnostic> {
    //     let mut bytes = vec![0u8; 2 * self.0.len()];
    //     crate::hex::encode_to_slice(self.0.clone(), &mut bytes).map_err(|e| {
    //         Diagnostic::error_from_string(format!("failed to encode raw content: {e}"))
    //     })?;
    //     Ok(bytes)
    // }
    pub fn as_hcl_str(&self) -> &str {
        let source = match &self {
            Self::Hcl(src) => src,
            _ => unreachable!(),
        };
        source.as_str()
    }

    pub fn from_hcl_construct(block: &Block) -> Self {
        Self::from_hcl_string(
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

        let raw_hcl = RunbookSource::from_hcl_string(input.trim().to_string());
        let constructs = raw_hcl.into_constructs().unwrap();
        assert_eq!(constructs.len(), 4);
        let addon_block = RunbookSource::from_hcl_construct(&constructs[0].expect_hcl_block())
            .as_hcl_str()
            .to_string();
        assert_eq!(addon_block, addon_block_str);
        let signer_block = RunbookSource::from_hcl_construct(&constructs[1].expect_hcl_block())
            .as_hcl_str()
            .to_string();
        assert_eq!(signer_block, signer_block_str);
        let runbook_block = RunbookSource::from_hcl_construct(&constructs[2].expect_hcl_block())
            .as_hcl_str()
            .to_string();
        assert_eq!(runbook_block, runbook_block_str);
        let output_block = RunbookSource::from_hcl_construct(&constructs[3].expect_hcl_block())
            .as_hcl_str()
            .to_string();
        assert_eq!(output_block, output_block_str);
    }

    #[test]
    fn test_collect_constructs_referenced_by_construct() {
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

        let raw_hcl = RunbookSource::from_hcl_string(input.trim().to_string());
        let construct = raw_hcl.into_construct().unwrap();

        let mut dependencies = vec![];
        collect_constructs_referenced_by_construct(
            &construct,
            None::<&CommandInput>,
            &mut dependencies,
        );

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

        let raw_hcl = RunbookSource::from_hcl_string(input.trim().to_string());
        let block = raw_hcl.into_construct().unwrap().expect_hcl_block().clone();
        let attribute = block.body.get_attribute("chain_id").unwrap();

        let mut dependencies = vec![];
        collect_constructs_references_from_hcl_expression(
            &attribute.value,
            None::<&CommandInput>,
            &mut dependencies,
        );

        assert_eq!(dependencies.len(), 1);
    }
}
