use jaq_interpret::{Ctx, FilterT, ParseCtx, RcIter, Val};
use serde_json::Value as JsonValue;
use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

use crate::std::functions::{arg_checker, to_diag};

lazy_static! {
    pub static ref JSON_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        JsonQuery => {
            name: "jq",
            documentation: indoc!{r#"
            The `jq` function allows slicing, filtering, and mapping JSON data. 
            See the [jq](https://jqlang.github.io/jq/manual/) documentation for more details.
            "#},                
            example: indoc!{r#"
              output "message" { 
                  value = jq("{ \"message\": \"Hello world!\" }", ".message")
              }
              > message: Hello world!
            "#},
            inputs: [
                decoded_json: {
                    documentation: "A JSON object.",
                    typing: vec![Type::string(), Type::arbitrary_object()]
                },
                query: {
                    documentation: "A JSON query. See the [jq](https://jqlang.github.io/jq/manual/) documentation.",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "The result of the `jq` query.",
                typing: Type::array(Type::string())
            },
        }
    },];
}

pub struct JsonQuery;
impl FunctionImplementation for JsonQuery {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let input_str = args.get(0).unwrap().encode_to_string();
        let filter = args.get(1).unwrap().as_string().unwrap();

        // try to deserialize the string as is
        let input: JsonValue = match serde_json::from_str(&input_str) {
            Ok(json) => json,
            // if it fails, trim quotes and try again
            Err(e) => serde_json::from_str(&input_str.trim_matches('"'))
                .map_err(|_| to_diag(fn_spec, format!("failed to decode input as json: {e}")))?,
        };

        let mut defs = ParseCtx::new(Vec::new());

        // parse the filter
        let (f, errs) = jaq_parse::parse(&filter, jaq_parse::main());
        if !errs.is_empty() {
            return Err(to_diag(fn_spec, errs.first().unwrap().to_string()));
        }

        // compile the filter in the context of the given definitions
        let f = defs.compile(f.unwrap());
        let errs = defs.errs;
        if !errs.is_empty() {
            return Err(to_diag(fn_spec, errs.first().unwrap().0.to_string()));
        }

        let inputs = RcIter::new(core::iter::empty());
        // iterator over the output values
        let result = f
            .run((Ctx::new([], &inputs), Val::from(input)))
            .into_iter()
            // todo: we need to allow other types other than string
            .map(|o| o.map(|v| Value::from_jaq_value(&v)))
            .collect::<Result<Result<Vec<Value>, _>, _>>()
            .map_err(|e| to_diag(fn_spec, e.to_string()))?
            .map_err(|e| to_diag(fn_spec, e.to_string()))?;
        if result.len() == 1 {
            Ok(result.first().unwrap().clone())
        } else {
            Ok(Value::array(result))
        }
    }
}
