use hcl_edit::expr::Object;

use crate::{
    hcl::{
        expr::{Expression, ObjectKey},
        structure::{Block, BlockLabel},
        template::{Element, StringTemplate},
    },
    types::commands::CommandInput,
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
    let label = block
        .labels
        .get(index)
        .ok_or(VisitorError::MissingField(name.to_string()))?;
    match label {
        BlockLabel::String(literal) => Ok(literal.to_string()),
        BlockLabel::Ident(_e) => Err(VisitorError::TypeMismatch(
            "string".into(),
            name.to_string(),
        )),
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

pub fn visit_optional_untyped_attribute(
    field_name: &str,
    block: &Block,
) -> Result<Option<Expression>, Vec<Diagnostic>> {
    let Some(attribute) = block.body.get_attribute(field_name) else {
        return Ok(None);
    };
    Ok(Some(attribute.value.clone()))
}

pub fn get_object_expression_key(obj: &Object, key: &str) -> Option<hcl_edit::expr::ObjectValue> {
    obj.into_iter()
        .find(|(k, _)| {
            k.as_ident()
                .and_then(|i| Some(i.as_str().eq(key)))
                .unwrap_or(false)
        })
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

pub fn collect_constructs_references_from_expression<'a>(
    expr: &Expression,
    input: Option<&'a CommandInput>,
    dependencies: &mut Vec<(Option<&'a CommandInput>, Expression)>,
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
