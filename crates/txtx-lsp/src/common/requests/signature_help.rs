use clarity_repl::clarity::docs::FunctionAPI;
use lsp_types::{ParameterInformation, ParameterLabel, Position, SignatureInformation};

use crate::state::ActiveContractData;

use super::{api_ref::API_REF, helpers::get_function_at_position};

pub fn get_signatures(
    contract: &ActiveContractData,
    position: &Position,
) -> Option<Vec<SignatureInformation>> {
    let (function_name, mut active_parameter) =
        get_function_at_position(position, contract.expressions.as_ref()?)?;

    if [
        "define-read-only",
        "define-public",
        "define-private",
        "define-trait,",
        "let",
        "begin",
        "tuple",
    ]
    .contains(&function_name.as_str())
    {
        // showing signature help for define-<function>, define-trait, let and bug adds to much noise
        // it doesn't make sense for the tuple {} notation
        return None;
    }

    let (version, _, reference) = API_REF.get(&function_name.to_string())?;
    let FunctionAPI { signature, output_type, .. } = (*reference).as_ref()?;

    if version > &contract.clarity_version {
        return None;
    }

    let signatures = signature
        .split(" |")
        .map(|mut signature| {
            signature = signature.trim();
            let mut signature_without_parenthesis = signature.chars();
            signature_without_parenthesis.next();
            signature_without_parenthesis.next_back();
            let signature_without_parenthesis = signature_without_parenthesis.as_str();
            let parameters = signature_without_parenthesis.split(' ').collect::<Vec<&str>>();
            let (_, parameters) = parameters.split_first().expect("invalid signature format");

            if active_parameter.unwrap_or_default() >= parameters.len().try_into().unwrap() {
                if let Some(variadic_index) = parameters.iter().position(|p| p.contains("...")) {
                    active_parameter = Some(variadic_index.try_into().unwrap());
                }
            }
            let label = if output_type.eq("Not Applicable") {
                String::from(signature)
            } else {
                format!("{} -> {}", &signature, &output_type)
            };

            SignatureInformation {
                active_parameter,
                documentation: None,
                label,
                parameters: Some(
                    parameters
                        .iter()
                        .map(|param| ParameterInformation {
                            documentation: None,
                            label: ParameterLabel::Simple(param.to_string()),
                        })
                        .collect::<Vec<ParameterInformation>>(),
                ),
            }
        })
        .collect::<Vec<SignatureInformation>>();

    Some(signatures)
}
