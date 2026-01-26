use std::collections::HashMap;

use clarity_repl::{
    analysis::ast_visitor::{traverse, ASTVisitor},
    clarity::{representations::Span, ClarityName, SymbolicExpression, SymbolicExpressionType},
};
use lsp_types::{DocumentSymbol, SymbolKind};
use serde::{Deserialize, Serialize};

use super::helpers::span_to_range;

fn symbolic_expression_to_name(symbolic_expr: &SymbolicExpression) -> String {
    match &symbolic_expr.expr {
        SymbolicExpressionType::Atom(name) => name.to_string(),
        SymbolicExpressionType::List(list) => symbolic_expression_to_name(&(*list).to_vec()[0]),
        _ => "".to_string(),
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct ClaritySymbolKind(i32);
impl ClaritySymbolKind {
    pub const FUNCTION: SymbolKind = SymbolKind::FUNCTION;
    pub const BEGIN: SymbolKind = SymbolKind::NAMESPACE;
    pub const LET: SymbolKind = SymbolKind::NAMESPACE;
    pub const NAMESPACE: SymbolKind = SymbolKind::NAMESPACE;
    pub const LET_BINDING: SymbolKind = SymbolKind::VARIABLE;
    pub const IMPL_TRAIT: SymbolKind = SymbolKind::NAMESPACE;
    pub const TRAIT: SymbolKind = SymbolKind::STRUCT;
    pub const TOKEN: SymbolKind = SymbolKind::NAMESPACE;
    pub const CONSTANT: SymbolKind = SymbolKind::CONSTANT;
    pub const VARIABLE: SymbolKind = SymbolKind::VARIABLE;
    pub const MAP: SymbolKind = SymbolKind::STRUCT;
    pub const KEY: SymbolKind = SymbolKind::KEY;
    pub const VALUE: SymbolKind = SymbolKind::PROPERTY;
    pub const FLOW: SymbolKind = SymbolKind::OBJECT;
    pub const RESPONSE: SymbolKind = SymbolKind::OBJECT;
}

fn build_symbol(
    name: &str,
    detail: Option<String>,
    kind: SymbolKind,
    span: &Span,
    children: Option<Vec<DocumentSymbol>>,
) -> DocumentSymbol {
    let range = span_to_range(span);
    #[allow(deprecated)]
    DocumentSymbol {
        name: name.to_string(),
        kind,
        detail,
        tags: None,
        deprecated: None,
        selection_range: range,
        range,
        children,
    }
}

#[derive(Clone, Debug)]
pub struct ASTSymbols {
    pub symbols: Vec<DocumentSymbol>,
    pub children_map: HashMap<u64, Vec<DocumentSymbol>>,
}

impl<'a> ASTSymbols {
    pub fn new() -> ASTSymbols {
        Self { symbols: Vec::new(), children_map: HashMap::new() }
    }

    pub fn get_symbols(mut self, expressions: &'a [SymbolicExpression]) -> Vec<DocumentSymbol> {
        traverse(&mut self, expressions);
        self.symbols
    }
}

impl<'a> ASTVisitor<'a> for ASTSymbols {
    fn visit_impl_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        trait_identifier: &clarity_repl::clarity::vm::types::TraitIdentifier,
    ) -> bool {
        self.symbols.push(build_symbol(
            "impl-trait",
            Some(trait_identifier.name.to_string()),
            ClaritySymbolKind::IMPL_TRAIT,
            &expr.span,
            None,
        ));
        true
    }

    fn visit_define_data_var(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        data_type: &'a SymbolicExpression,
        initial: &'a SymbolicExpression,
    ) -> bool {
        let symbol_type = symbolic_expression_to_name(data_type);
        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some(symbol_type),
            ClaritySymbolKind::VARIABLE,
            &expr.span,
            self.children_map.remove(&initial.id),
        ));

        true
    }

    fn visit_tuple(
        &mut self,
        expr: &'a SymbolicExpression,
        values: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> bool {
        let mut symbols: Vec<DocumentSymbol> = Vec::new();
        for (name, expr) in values.iter() {
            if let Some(name) = name {
                symbols.push(build_symbol(
                    name.as_str(),
                    None,
                    ClaritySymbolKind::VALUE,
                    &expr.span,
                    self.children_map.remove(&expr.id),
                ));
            }
        }
        self.children_map.insert(expr.id, symbols);
        true
    }

    fn visit_define_constant(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        _value: &'a SymbolicExpression,
    ) -> bool {
        self.symbols.push(build_symbol(
            &name.to_owned(),
            None,
            ClaritySymbolKind::CONSTANT,
            &expr.span,
            None,
        ));
        true
    }

    fn visit_define_map(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        key_type: &'a SymbolicExpression,
        value_type: &'a SymbolicExpression,
    ) -> bool {
        let children = vec![
            build_symbol(
                "key",
                Some(symbolic_expression_to_name(key_type)),
                ClaritySymbolKind::KEY,
                &key_type.span,
                None,
            ),
            build_symbol(
                "value",
                Some(symbolic_expression_to_name(value_type)),
                ClaritySymbolKind::VALUE,
                &value_type.span,
                None,
            ),
        ];

        self.symbols.push(build_symbol(
            &name.to_owned(),
            None,
            ClaritySymbolKind::MAP,
            &expr.span,
            Some(children),
        ));
        true
    }

    fn visit_define_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        functions: &'a [SymbolicExpression],
    ) -> bool {
        let mut children = Vec::new();
        let methods = functions[0].match_list().unwrap();
        for expr in methods {
            let list = expr.match_list().unwrap();
            let name = &list[0].match_atom().unwrap();
            children.push(build_symbol(
                name.to_owned(),
                Some("trait method".to_owned()),
                ClaritySymbolKind::FUNCTION,
                &expr.span,
                None,
            ))
        }

        self.symbols.push(build_symbol(
            &name.to_owned(),
            None,
            ClaritySymbolKind::TRAIT,
            &expr.span,
            Some(children),
        ));
        true
    }

    fn visit_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some("private".to_owned()),
            ClaritySymbolKind::FUNCTION,
            &expr.span,
            self.children_map.remove(&body.id),
        ));
        true
    }

    fn visit_define_public(
        &mut self,
        expr: &'a clarity_repl::clarity::SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        body: &'a clarity_repl::clarity::SymbolicExpression,
    ) -> bool {
        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some("public".to_owned()),
            ClaritySymbolKind::FUNCTION,
            &expr.span,
            self.children_map.remove(&body.id),
        ));
        true
    }

    fn visit_define_read_only(
        &mut self,
        expr: &'a clarity_repl::clarity::SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        body: &'a clarity_repl::clarity::SymbolicExpression,
    ) -> bool {
        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some("read-only".to_owned()),
            ClaritySymbolKind::FUNCTION,
            &expr.span,
            self.children_map.remove(&body.id),
        ));
        true
    }

    fn visit_define_ft(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        _supply: Option<&'a SymbolicExpression>,
    ) -> bool {
        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some("FT".to_owned()),
            ClaritySymbolKind::TOKEN,
            &expr.span,
            None,
        ));
        true
    }

    fn visit_define_nft(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a clarity_repl::clarity::ClarityName,
        nft_type: &'a SymbolicExpression,
    ) -> bool {
        let nft_type = match nft_type.expr.clone() {
            SymbolicExpressionType::Atom(name) => name.to_string(),
            SymbolicExpressionType::List(_) => "tuple".to_string(),
            _ => "".to_string(),
        };

        self.symbols.push(build_symbol(
            &name.to_owned(),
            Some(format!("NFT {}", &nft_type)),
            ClaritySymbolKind::TOKEN,
            &expr.span,
            None,
        ));
        true
    }

    fn visit_begin(
        &mut self,
        expr: &'a SymbolicExpression,
        statements: &'a [SymbolicExpression],
    ) -> bool {
        let mut children = Vec::new();
        for statement in statements.iter() {
            if let Some(mut child) = self.children_map.remove(&statement.id) {
                children.append(&mut child);
            }
        }

        self.children_map.insert(
            expr.id,
            vec![build_symbol("begin", None, ClaritySymbolKind::BEGIN, &expr.span, Some(children))],
        );
        true
    }

    fn visit_let(
        &mut self,
        expr: &'a SymbolicExpression,
        bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        body: &'a [SymbolicExpression],
    ) -> bool {
        let mut children: Vec<DocumentSymbol> = Vec::new();

        let mut bindings_children: Vec<DocumentSymbol> = Vec::new();
        for (name, expr) in bindings.iter() {
            bindings_children.push(build_symbol(
                name.as_str(),
                None,
                ClaritySymbolKind::LET_BINDING,
                &expr.span,
                self.children_map.remove(&expr.id),
            ))
        }
        if !bindings_children.is_empty() {
            let start = bindings_children.first().unwrap().range.start;
            let end = bindings_children.last().unwrap().range.start;
            let bindings_span = Span {
                start_line: start.line + 1,
                start_column: start.character + 1,
                end_line: end.line + 1,
                end_column: end.character + 1,
            };
            children.push(build_symbol(
                "bindings",
                None,
                ClaritySymbolKind::NAMESPACE,
                &bindings_span,
                Some(bindings_children),
            ));
        }

        let mut body_children = Vec::new();
        for statement in body.iter() {
            if let Some(children) = self.children_map.remove(&statement.id) {
                for child in children {
                    body_children.push(child);
                }
            }
        }
        if !body_children.is_empty() {
            let start = body_children.first().unwrap().range.start;
            let end = body_children.last().unwrap().range.start;
            let body_span = Span {
                start_line: start.line + 1,
                start_column: start.character + 1,
                end_line: end.line + 1,
                end_column: end.character + 1,
            };
            children.push(build_symbol(
                "body",
                None,
                ClaritySymbolKind::NAMESPACE,
                &body_span,
                Some(body_children),
            ));
        }

        self.children_map.insert(
            expr.id,
            vec![build_symbol("let", None, ClaritySymbolKind::LET, &expr.span, Some(children))],
        );
        true
    }

    fn visit_asserts(
        &mut self,
        expr: &'a SymbolicExpression,
        cond: &'a SymbolicExpression,
        thrown: &'a SymbolicExpression,
    ) -> bool {
        let mut children = Vec::new();

        if self.children_map.contains_key(&cond.id) {
            children.append(&mut self.children_map.remove(&cond.id).unwrap())
        }
        if self.children_map.contains_key(&thrown.id) {
            children.append(&mut self.children_map.remove(&thrown.id).unwrap())
        }

        self.children_map.insert(
            expr.id,
            vec![build_symbol(
                "asserts!",
                None,
                ClaritySymbolKind::FLOW,
                &expr.span,
                Some(children),
            )],
        );

        true
    }

    fn visit_try(&mut self, expr: &'a SymbolicExpression, input: &'a SymbolicExpression) -> bool {
        let children = self.children_map.remove(&input.id);
        self.children_map.insert(
            expr.id,
            vec![build_symbol("try!", None, ClaritySymbolKind::FLOW, &expr.span, children)],
        );

        true
    }

    fn visit_ok(&mut self, expr: &'a SymbolicExpression, value: &'a SymbolicExpression) -> bool {
        let children = self.children_map.remove(&value.id);
        self.children_map.insert(
            expr.id,
            vec![build_symbol("ok", None, ClaritySymbolKind::RESPONSE, &expr.span, children)],
        );
        true
    }

    fn visit_err(&mut self, expr: &'a SymbolicExpression, value: &'a SymbolicExpression) -> bool {
        let children = self.children_map.remove(&value.id);
        self.children_map.insert(
            expr.id,
            vec![build_symbol("err", None, ClaritySymbolKind::RESPONSE, &expr.span, children)],
        );
        true
    }
}
