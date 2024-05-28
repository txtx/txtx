#[macro_export]
macro_rules! diagnosed_error {
    ($($arg:tt)*) => {{
        use txtx_addon_kit::types::diagnostics::{DiagnosticLevel, Diagnostic};

        let res = format_args!($($arg)*).to_string();
        Diagnostic {
            span: None,
            location: None,
            message: res,
            level: DiagnosticLevel::Error,
            documentation: None,
            example: None,
            parent_diagnostic: None,
        }
    }};
}

#[macro_export]
macro_rules! define_function {
    ($func_key:ident => {
        name: $fn_name:expr,
        documentation: $doc:expr,
        example: $example:expr,
        inputs: [$($input_name:ident: { documentation: $input_doc:expr, typing: $input_ts:expr }),*],
        output: { documentation: $output_doc:expr, typing: $output_ts:expr },
    }) => {
        txtx_addon_kit::types::functions::FunctionSpecification {
            name: String::from($fn_name),
            documentation: String::from($doc),
            inputs: vec![$(txtx_addon_kit::types::functions::FunctionInput {
                name: String::from(stringify!($input_name)),
                documentation: String::from($input_doc),
                typing: $input_ts,
            }),*],
            output: txtx_addon_kit::types::functions::FunctionOutput {
                documentation: String::from($output_doc),
                typing: $output_ts,
            },
            example: String::from($example),
            snippet: String::from(""),
            runner: $func_key::run,
            checker: $func_key::check_instantiability,
        };
    };
}

#[macro_export]
macro_rules! define_command {
    ($func_key:ident => {
        name: $fn_name:expr,
        matcher: $matcher:expr,
        documentation: $doc:expr,
        // todo: add key field and use the input_name as the key, so the user can also provide a web-ui facing name
        inputs: [$($input_name:ident: { documentation: $input_doc:expr, typing: $input_ts:expr, optional: $optional:expr, interpolable: $interpolable:expr }),*],
        outputs: [$($output_name:ident: { documentation: $output_doc:expr, typing: $output_ts:expr }),*],
        example: $example:expr,
    }) => {
        {
        use txtx_addon_kit::types::commands::{PreCommandSpecification, CommandSpecification, CommandInput, CommandOutput, CommandRunner};
        PreCommandSpecification::Atomic(
          CommandSpecification {
            name: String::from($fn_name),
            matcher: String::from($matcher),
            documentation: String::from($doc),
            accepts_arbitrary_inputs: false,
            create_output_for_each_input: false,
            update_addon_defaults: false,
            inputs: vec![$(CommandInput {
                name: String::from(stringify!($input_name)),
                documentation: String::from($input_doc),
                typing: $input_ts,
                optional: $optional,
                interpolable: $interpolable,
                check_required: false,
                check_performed: false
            }),*],
            default_inputs: CommandSpecification::default_inputs(),
            outputs: vec![$(CommandOutput {
                name: String::from(stringify!($output_name)),
                documentation: String::from($output_doc),
                typing: $output_ts,
            }),*],
            check_instantiability: $func_key::check_instantiability,
            check_executability: $func_key::check_executability,
            execute: Box::new($func_key::execute),
            example: String::from($example),
        }
      )
    }
    };
}

#[macro_export]
macro_rules! define_multistep_command {
  ($func_key:ident => {
      name: $fn_name:expr,
      matcher: $matcher:expr,
      documentation: $doc:expr,
      parts: [$($part:expr),*],
      example: $example:expr,
  }) => {
      {
          use txtx_addon_kit::types::commands::{PreCommandSpecification, CompositeCommandSpecification, CommandInput, CommandOutput, CommandRunner};

          let mut parts = Vec::new();
          $(parts.push($part);)*

          PreCommandSpecification::Composite( CompositeCommandSpecification {
              name: String::from($fn_name),
              matcher: String::from($matcher),
              documentation: String::from($doc),
              parts: parts,
              default_inputs: CommandSpecification::default_inputs(),
              router: $func_key::router,
              example: String::from($example),
          })
      }
  };
}

#[macro_export]
macro_rules! define_object_type {
    [
        $($input_name:ident: {
            documentation: $input_doc:expr,
            typing: $input_ts:expr,
            optional: $optional:expr,
            interpolable: $interpolable:expr
        }),*
    ] => {
        Type::object(vec![$(txtx_addon_kit::types::types::ObjectProperty {
            name: String::from(stringify!($input_name)),
            documentation: String::from($input_doc),
            typing: $input_ts,
            optional: $optional,
            interpolable: $interpolable,
        }),*])
    };
}

#[macro_export]
macro_rules! define_addon_type {
    ($func_key:ident => {
        name: $fn_name:expr,
        documentation: $doc:expr,
    }) => {
        txtx_addon_kit::types::types::TypeSpecification {
            id: String::from($fn_name),
            documentation: String::from($doc),
            checker: $func_key::check,
        };
    };
}

#[macro_export]
macro_rules! define_wallet {
    ($func_key:ident => {
        name: $fn_name:expr,
        matcher: $matcher:expr,
        documentation: $doc:expr,
        inputs: [$($input_name:ident: { documentation: $input_doc:expr, typing: $input_ts:expr, optional: $optional:expr, interpolable: $interpolable:expr }),*],
        outputs: [$($output_name:ident: { documentation: $output_doc:expr, typing: $output_ts:expr }),*],
        example: $example:expr,
    }) => {
        {
          use txtx_addon_kit::types::wallets::{WalletSpecification, WalletRunner};
          use txtx_addon_kit::types::commands::{CommandInput, CommandOutput};
          WalletSpecification {
            name: String::from($fn_name),
            matcher: String::from($matcher),
            documentation: String::from($doc),
            accepts_arbitrary_inputs: false,
            create_output_for_each_input: false,
            update_addon_defaults: false,
            inputs: vec![$(CommandInput {
                name: String::from(stringify!($input_name)),
                documentation: String::from($input_doc),
                typing: $input_ts,
                optional: $optional,
                interpolable: $interpolable,
                check_required: false,
                check_performed: false
            }),*],
            default_inputs: CommandSpecification::default_inputs(),
            outputs: vec![$(CommandOutput {
                name: String::from(stringify!($output_name)),
                documentation: String::from($output_doc),
                typing: $output_ts,
            }),*],
            check_instantiability: $func_key::check_instantiability,
            check_executability: $func_key::check_executability,
            execute: Box::new($func_key::execute),
            sign: Box::new($func_key::sign),
            check_sign_executability: $func_key::check_sign_executability,
            example: String::from($example),
        }

    }
    };
}
