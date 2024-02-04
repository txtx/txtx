#[macro_export]
macro_rules! define_native_function {
    ($func_key:ident => {
        name: $fn_name:expr,
        documentation: $doc:expr,
        example: $example:expr,
        inputs: [$($input_name:ident: { documentation: $input_doc:expr, type_signature: $input_ts:expr }),*],
        output: { documentation: $output_doc:expr, type_signature: $output_ts:expr },
    }) => {
        txtx_addon_kit::types::functions::NativeFunction {
            name: String::from($fn_name),
            documentation: String::from($doc),
            inputs: vec![$(txtx_addon_kit::types::functions::NativeFunctionInput {
                name: String::from(stringify!($input_name)),
                documentation: String::from($input_doc),
                type_signature: $input_ts,
            }),*],
            output: txtx_addon_kit::types::functions::NativeFunctionOutput {
                documentation: String::from($output_doc),
                type_signature: $output_ts,
            },
            example: String::from($example),
            snippet: String::from(""),
            run: $func_key::run,
            check: $func_key::check,
        };
    };
}
