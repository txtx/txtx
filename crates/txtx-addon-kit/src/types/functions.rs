use super::{
    diagnostics::Diagnostic,
    function_errors::FunctionErrorRef,
    namespace::Namespace,
    types::{Type, Value},
    AuthorizationContext,
};

#[derive(Clone, Debug)]
pub struct FunctionInput {
    pub name: String,
    pub documentation: String,
    pub typing: Vec<Type>,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub struct FunctionOutput {
    pub documentation: String,
    pub typing: Type,
}

#[derive(Clone, Debug)]
pub struct FunctionSpecification {
    pub name: String,
    pub documentation: String,
    pub inputs: Vec<FunctionInput>,
    pub output: FunctionOutput,
    pub example: String,
    pub snippet: String,
    pub runner: FunctionRunner,
    pub checker: FunctionChecker,
}

type FunctionRunner =
    fn(&FunctionSpecification, &AuthorizationContext, &Vec<Value>) -> Result<Value, Diagnostic>;
type FunctionChecker =
    fn(&FunctionSpecification, &AuthorizationContext, &Vec<Type>) -> Result<Type, Diagnostic>;

pub trait FunctionImplementation {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Value>,
    ) -> Result<Value, Diagnostic>;
}

pub fn fn_diag_with_ctx(
    namespace: Namespace,
) -> impl Fn(&FunctionSpecification, String) -> Diagnostic {
    let fn_diag_with_ctx = move |fn_spec: &FunctionSpecification, e: String| -> Diagnostic {
        FunctionErrorRef::ExecutionError {
            namespace: namespace.as_str(),
            function: &fn_spec.name,
            message: &e,
        }
        .into()
    };
    return fn_diag_with_ctx;
}

pub fn arg_checker_with_ctx(
    namespace: Namespace,
) -> impl Fn(&FunctionSpecification, &Vec<Value>) -> Result<(), Diagnostic> {
    let fn_checker =
        move |fn_spec: &FunctionSpecification, args: &Vec<Value>| -> Result<(), Diagnostic> {
            for (i, input) in fn_spec.inputs.iter().enumerate() {
                if !input.optional {
                    if let Some(arg) = args.get(i) {
                        let mut has_type_match = false;
                        for typing in input.typing.iter() {
                            let arg_type = arg.get_type();
                            // special case if both are addons: we don't want to be so strict that
                            // we check the addon id here
                            if let Type::Addon(_) = arg_type {
                                if let Type::Addon(_) = typing {
                                    has_type_match = true;
                                    break;
                                }
                            }
                            // special case for empty arrays
                            if let Type::Array(_) = arg_type {
                                if arg.expect_array().len() == 0 {
                                    has_type_match = true;
                                    break;
                                }
                            }
                            // we don't have an "any" type, so if the array is of type null, we won't check types
                            if let Type::Array(inner) = typing {
                                if let Type::Null(_) = **inner {
                                    has_type_match = true;
                                    break;
                                }
                            }
                            if arg_type.eq(typing) {
                                has_type_match = true;
                                break;
                            }
                        }
                        if !has_type_match {
                            let arg_type = arg.get_type();
                            return Err(FunctionErrorRef::TypeMismatch {
                                namespace: namespace.as_str(),
                                function: &fn_spec.name,
                                position: i + 1,
                                name: &input.name,
                                expected: &input.typing,
                                found: &arg_type,
                            }
                            .into());
                        }
                    } else {
                        return Err(FunctionErrorRef::MissingArgument {
                            namespace: namespace.as_str(),
                            function: &fn_spec.name,
                            position: i + 1,
                            name: &input.name,
                        }
                        .into());
                    }
                }
            }
            Ok(())
        };
    return fn_checker;
}
