use std::{io::Read, str::FromStr};

use alloy::primitives::Address;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{ObjectProperty, PrimitiveValue, Type, Value},
};

use crate::{codec::foundry::FoundryConfig, typing::ETH_ADDRESS};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        EncodeEVMAddress => {
            name: "address",
            documentation: "`evm::address` creates a valid Ethereum address from the input string.",
            example: indoc! {r#"
                    output "address" { 
                    value = evm::address("0x627306090abaB3A6e1400e9345bC60c78a8BEf57")
                    }
                    // > todo
                    "#},
            inputs: [
                address_string: {
                    documentation: "An Ethereum address string.",
                    typing: vec![Type::uint()]
                }
            ],
            output: {
                documentation: "The input string as an Ethereum address.",
                typing: Type::uint()
            },
        }
    }];
}

#[derive(Clone)]
pub struct EncodeEVMAddress;
impl FunctionImplementation for EncodeEVMAddress {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let entry = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::String(val))) => val.clone(),
            other => {
                return Err(diagnosed_error!(
                    "'evm::address' function: expected string, got {:?}",
                    other
                ))
            }
        };
        let address = Address::from_str(&entry)
            .map_err(|e| diagnosed_error!("'evm::address' function: invalid address: {}", e))?;
        let bytes = address.0 .0;
        Ok(Value::buffer(bytes.to_vec(), ETH_ADDRESS.clone()))
    }
}
