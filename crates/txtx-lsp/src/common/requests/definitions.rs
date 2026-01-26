use std::collections::HashMap;

use super::helpers::span_to_range;

use clarity_repl::analysis::ast_visitor::{traverse, ASTVisitor, TypedVar};
use clarity_repl::clarity::functions::define::DefineFunctions;
use clarity_repl::clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity_repl::clarity::{ClarityName, SymbolicExpression};
use lsp_types::Range;

#[cfg(feature = "wasm")]
#[allow(unused_imports)]
use crate::utils::log;

#[derive(Clone, Debug, PartialEq)]
pub enum DefinitionLocation {
    Internal(Range),
    External(QualifiedContractIdentifier, ClarityName),
}

// `global` holds all of the top-level user-defined keywords that are available in the global scope
// `local` holds the locally user-defined keywords: function parameters, let and match bindings
// when a user-defined keyword is used in the code, its position and definition location are stored in `tokens`
#[derive(Clone, Debug, Default)]
pub struct Definitions {
    pub tokens: HashMap<(u32, u32), DefinitionLocation>,
    global: HashMap<ClarityName, Range>,
    local: HashMap<u64, HashMap<ClarityName, Range>>,
    deployer: Option<StandardPrincipalData>,
}

impl<'a> Definitions {
    pub fn new(deployer: Option<StandardPrincipalData>) -> Self {
        Self { deployer, ..Default::default() }
    }

    pub fn run(&mut self, expressions: &'a [SymbolicExpression]) {
        traverse(self, expressions);
    }

    fn set_function_parameters_scope(&mut self, expr: &SymbolicExpression) -> Option<()> {
        let mut local_scope = HashMap::new();
        let (_, binding_exprs) = expr.match_list()?.get(1)?.match_list()?.split_first()?;
        for binding in binding_exprs {
            if let Some(name) = binding
                .match_list()
                .and_then(|l| l.split_first())
                .and_then(|(name, _)| name.match_atom())
            {
                local_scope.insert(name.to_owned(), span_to_range(&binding.span));
            }
        }
        self.local.insert(expr.id, local_scope);
        Some(())
    }

    // helper method to retrieve definitions of global keyword used in methods such as
    // (var-get <global-keyword>) (map-insert <global-keyword> ...) (nft-burn <global-keyword> ...)
    fn set_definition_for_arg_at_index(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        index: usize,
    ) -> Option<()> {
        let range = self.global.get(token)?;
        let keyword = expr.match_list()?.get(index)?;
        self.tokens.insert(
            (keyword.span.start_line, keyword.span.start_column),
            DefinitionLocation::Internal(*range),
        );
        Some(())
    }
}

impl<'a> ASTVisitor<'a> for Definitions {
    fn traverse_expr(&mut self, expr: &'a SymbolicExpression) -> bool {
        use clarity_repl::clarity::vm::representations::SymbolicExpressionType::*;
        match &expr.expr {
            AtomValue(value) => self.visit_atom_value(expr, value),
            Atom(name) => self.visit_atom(expr, name),
            List(exprs) => {
                let result = self.traverse_list(expr, exprs);
                // clear local scope after traversing it
                self.local.remove(&expr.id);
                result
            }
            LiteralValue(value) => self.visit_literal_value(expr, value),
            Field(field) => self.visit_field(expr, field),
            TraitReference(name, trait_def) => self.visit_trait_reference(expr, name, trait_def),
        }
    }

    fn visit_atom(&mut self, expr: &'a SymbolicExpression, atom: &'a ClarityName) -> bool {
        // iterate on local scopes to find if the variable is declared in one of them
        // the order does not matter because variable shadowing is not allowed
        for scope in self.local.values() {
            if let Some(range) = scope.get(atom) {
                self.tokens.insert(
                    (expr.span.start_line, expr.span.start_column),
                    DefinitionLocation::Internal(*range),
                );
                return true;
            }
        }

        if let Some(range) = self.global.get(atom) {
            self.tokens.insert(
                (expr.span.start_line, expr.span.start_column),
                DefinitionLocation::Internal(*range),
            );
        }
        true
    }

    fn visit_var_set(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _value: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_var_get(&mut self, expr: &'a SymbolicExpression, name: &'a ClarityName) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_map_insert(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        _value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_map_get(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_map_set(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        _value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_map_delete(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, name, 1);
        true
    }

    fn visit_call_user_defined(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> bool {
        if let Some(range) = self.global.get(name) {
            self.tokens.insert(
                (expr.span.start_line, expr.span.start_column + 1),
                DefinitionLocation::Internal(*range),
            );
        }
        true
    }

    fn visit_ft_mint(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_ft_burn(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_ft_get_balance(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _owner: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_ft_get_supply(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_ft_transfer(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_nft_burn(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_nft_get_owner(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_nft_mint(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_nft_transfer(
        &mut self,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, token, 1);
        true
    }

    fn visit_map(
        &mut self,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        _sequences: &'a [SymbolicExpression],
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, func, 1);
        true
    }

    fn visit_filter(
        &mut self,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        _sequence: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, func, 1);
        true
    }

    fn visit_fold(
        &mut self,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        _sequence: &'a SymbolicExpression,
        _initial: &'a SymbolicExpression,
    ) -> bool {
        self.set_definition_for_arg_at_index(expr, func, 1);
        true
    }

    fn visit_static_contract_call(
        &mut self,
        expr: &'a SymbolicExpression,
        identifier: &'a QualifiedContractIdentifier,
        function_name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> bool {
        if let Some(list) = expr.match_list() {
            if let Some(SymbolicExpression { span, .. }) = list.get(2) {
                let identifier = if identifier.issuer == StandardPrincipalData::transient() {
                    match &self.deployer {
                        Some(deployer) => QualifiedContractIdentifier::parse(&format!(
                            "{}.{}",
                            deployer, identifier.name
                        ))
                        .expect("failed to set contract name"),
                        None => identifier.to_owned(),
                    }
                } else {
                    identifier.to_owned()
                };

                self.tokens.insert(
                    (span.start_line, span.start_column),
                    DefinitionLocation::External(identifier, function_name.to_owned()),
                );
            };
        };

        true
    }

    fn traverse_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.set_function_parameters_scope(expr);
        self.traverse_expr(body) && self.visit_define_private(expr, name, parameters, body)
    }

    fn visit_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn traverse_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.set_function_parameters_scope(expr);
        self.traverse_expr(body) && self.visit_define_read_only(expr, name, parameters, body)
    }

    fn visit_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn traverse_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.set_function_parameters_scope(expr);
        self.traverse_expr(body) && self.visit_define_public(expr, name, parameters, body)
    }

    fn visit_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn visit_define_constant(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _value: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn visit_define_data_var(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _data_type: &'a SymbolicExpression,
        _initial: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn visit_define_map(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key_type: &'a SymbolicExpression,
        _value_type: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn visit_define_ft(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _supply: Option<&'a SymbolicExpression>,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn visit_define_nft(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _nft_type: &'a SymbolicExpression,
    ) -> bool {
        self.global.insert(name.clone(), span_to_range(&expr.span));
        true
    }

    fn traverse_let(
        &mut self,
        expr: &'a SymbolicExpression,
        bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        body: &'a [SymbolicExpression],
    ) -> bool {
        let local_scope = || -> Option<HashMap<ClarityName, Range>> {
            let mut result = HashMap::new();

            let binding_exprs = expr.match_list()?.get(1)?.match_list()?;
            for binding in binding_exprs {
                if let Some(name) = binding
                    .match_list()
                    .and_then(|l| l.split_first())
                    .and_then(|(name, _)| name.match_atom())
                {
                    result.insert(name.to_owned(), span_to_range(&binding.span));
                }
            }
            Some(result)
        };
        if let Some(local_scope) = local_scope() {
            self.local.insert(expr.id, local_scope);
        }

        for binding in bindings.values() {
            if !self.traverse_expr(binding) {
                return false;
            }
        }

        for expr in body {
            if !self.traverse_expr(expr) {
                return false;
            }
        }
        self.visit_let(expr, bindings, body)
    }

    fn traverse_match_option(
        &mut self,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        some_name: &'a ClarityName,
        some_branch: &'a SymbolicExpression,
        none_branch: &'a SymbolicExpression,
    ) -> bool {
        self.local
            .insert(expr.id, HashMap::from([(some_name.clone(), span_to_range(&input.span))]));
        self.traverse_expr(input)
            && self.traverse_expr(some_branch)
            && self.traverse_expr(none_branch)
            && self.visit_match_option(expr, input, some_name, some_branch, none_branch)
    }

    fn traverse_match_response(
        &mut self,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        ok_name: &'a ClarityName,
        ok_branch: &'a SymbolicExpression,
        err_name: &'a ClarityName,
        err_branch: &'a SymbolicExpression,
    ) -> bool {
        self.local.insert(
            expr.id,
            HashMap::from([
                (ok_name.clone(), span_to_range(&input.span)),
                (err_name.clone(), span_to_range(&input.span)),
            ]),
        );
        self.traverse_expr(input)
            && self.traverse_expr(ok_branch)
            && self.traverse_expr(err_branch)
            && self.visit_match_response(expr, input, ok_name, ok_branch, err_name, err_branch)
    }
}

pub fn get_definitions(
    expressions: &[SymbolicExpression],
    issuer: Option<StandardPrincipalData>,
) -> HashMap<(u32, u32), DefinitionLocation> {
    let mut definitions_visitor = Definitions::new(issuer);
    definitions_visitor.run(expressions);
    definitions_visitor.tokens
}

pub fn get_public_function_definitions(
    expressions: &Vec<SymbolicExpression>,
) -> HashMap<ClarityName, Range> {
    let mut definitions = HashMap::new();

    for expression in expressions {
        if let Some((function_name, args)) = expression
            .match_list()
            .and_then(|l| l.split_first())
            .and_then(|(function_name, args)| Some((function_name.match_atom()?, args)))
        {
            if let Some(DefineFunctions::PublicFunction | DefineFunctions::ReadOnlyFunction) =
                DefineFunctions::lookup_by_name(function_name)
            {
                if let Some(function_name) = args
                    .split_first()
                    .and_then(|(args_list, _)| args_list.match_list()?.split_first())
                    .and_then(|(function_name, _)| function_name.match_atom())
                {
                    definitions.insert(function_name.to_owned(), span_to_range(&expression.span));
                }
            }
        }
    }

    definitions
}
