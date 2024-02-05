

lazy_static! {
    pub static ref STACKS_CONSTRUCTS: Vec<ConstructDeclaration> = vec![
        define_construct! {
            StacksCallContract => {
                name: "call_contract",
                documentation: "Encode contract call payload",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool,
                        optional: true,
                        default_value: 
                    }
                ],
                outputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool,
                        optional: true,
                        default_value: 
                    }
                ],
                default_output: clarity_value
            }
        },
        define_function! {
            StacksDeployContract => {
                name: "deploy_contract",
                documentation: "Encode contract deployment payload",
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool,
                        optional: true,
                        default_value: 
                    }
                ],
                outputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Typing::Bool,
                        optional: true,
                        default_value: 
                    }
                ],
                default_output: clarity_value
            }
        },
    ];
}
