use jaq_interpret::{Ctx, FilterT, ParseCtx, RcIter, Val};
use kit::types::AuthorizationContext;
use serde_json::Value as JsonValue;
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

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
                    typing: vec![Type::string()]
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
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Value::String(input_str) = args.get(0).unwrap() else {
            return Err(diagnosed_error!("json query input must be a string"));
        };
        let Value::String(filter) = args.get(1).unwrap() else {
            return Err(diagnosed_error!("json query filter must be a string"));
        };
        let input: JsonValue = serde_json::from_str(&input_str).unwrap();

        let mut defs = ParseCtx::new(Vec::new());

        // parse the filter
        let (f, errs) = jaq_parse::parse(filter, jaq_parse::main());
        assert_eq!(errs, Vec::new());

        // compile the filter in the context of the given definitions
        let f = defs.compile(f.unwrap());
        assert!(defs.errs.is_empty());

        let inputs = RcIter::new(core::iter::empty());

        // iterator over the output values
        let result = f
            .run((Ctx::new([], &inputs), Val::from(input)))
            .into_iter()
            // todo: we need to allow other types other than string
            .map(|o| Value::from_jaq_value(o.unwrap()))
            .collect::<Vec<Value>>();
        Ok(Value::array(result))
    }
}
