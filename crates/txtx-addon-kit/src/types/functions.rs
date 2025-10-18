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
    fn(&FunctionSpecification, &AuthorizationContext, &[Value]) -> Result<Value, Diagnostic>;
type FunctionChecker =
    fn(&FunctionSpecification, &AuthorizationContext, &[Type]) -> Result<Type, Diagnostic>;

pub trait FunctionImplementation {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic>;

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Value],
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
) -> impl Fn(&FunctionSpecification, &[Value]) -> Result<(), Diagnostic> {
    move |fn_spec, args| {
        for (i, input) in fn_spec.inputs.iter().enumerate() {
            if input.optional {
                continue;
            }

            let arg = args.get(i).ok_or_else(|| {
                Diagnostic::from(FunctionErrorRef::MissingArgument {
                    namespace: namespace.as_str(),
                    function: &fn_spec.name,
                    position: i + 1,
                    name: &input.name,
                })
            })?;

            let type_matches = input.typing.iter().any(|typing| {
                match (arg.get_type(), typing) {
                    // Both are addons (any addon matches)
                    (Type::Addon(_), Type::Addon(_)) => true,
                    // Empty arrays always match
                    (Type::Array(_), _) if arg.expect_array().is_empty() => true,
                    // Array with null inner type (accepts any array)
                    (_, Type::Array(inner)) if matches!(**inner, Type::Null) => true,
                    // Exact type match
                    (arg_type, typing) => arg_type.eq(typing),
                }
            });

            if !type_matches {
                return Err(FunctionErrorRef::TypeMismatch {
                    namespace: namespace.as_str(),
                    function: &fn_spec.name,
                    position: i + 1,
                    name: &input.name,
                    expected: &input.typing,
                    found: &arg.get_type(),
                }
                .into());
            }
        }
        Ok(())
    }
}
