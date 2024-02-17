use std::collections::HashMap;

use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveType, Typing, Value},
};

lazy_static! {
    pub static ref STACKS_COMMANDS: Vec<CommandSpecification> = vec![
        define_command! {
            StacksCallContract => {
                name: "Stacks Contract Call",
                matcher: "call_contract",
                documentation: "Encode contract call payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Typing::string(),
                        optional: true,
                        interpolable: true
                    },
                    contract_id: {
                        documentation: "Contract identifier to invoke",
                        typing: Typing::string(),
                        optional: false,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded contract call",
                        typing: Typing::string()
                    }
                ],
            }
        },
        define_command! {
            StacksDeployContract => {
                name: "Stacks Contract Deployment",
                matcher: "deploy_contract",
                documentation: "Encode contract deployment payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Typing::string(),
                        optional: true,
                        interpolable: true
                    },
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::bool(),
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded contract call",
                        typing: Typing::string()
                    }
                ],
            }
        },
        define_command! {
            StacksTransaction => {
                name: "Stacks Transaction",
                matcher: "transaction",
                documentation: "Encode contract deployment payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Typing::string(),
                        optional: true,
                        interpolable: true
                    },
                    no_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Nonce of the transaction",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            }
                        ],
                        optional: true,
                        interpolable: true
                    },
                    cli_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Nonce of the transaction",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            }
                        ],
                        optional: true,
                        interpolable: true
                    },
                    web_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Nonce of the transaction",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            }
                        ],
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded contract call",
                        typing: Typing::string()
                    }
                ],
            }
        },
    ];
}

pub struct StacksCallContract;
impl CommandImplementation for StacksCallContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("contract_id").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }
}

pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }
}

pub struct StacksTransaction;
impl CommandImplementation for StacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        println!("{:?}", args);
        let value = args.get("no_interact").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }
}
