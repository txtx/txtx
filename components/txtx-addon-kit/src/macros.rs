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
            checker: $func_key::check,
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
            inputs: vec![$(CommandInput {
                name: String::from(stringify!($input_name)),
                documentation: String::from($input_doc),
                typing: $input_ts,
                optional: $optional,
                interpolable: $interpolable,
            }),*],
            default_inputs: CommandSpecification::default_inputs(),
            outputs: vec![$(CommandOutput {
                name: String::from(stringify!($output_name)),
                documentation: String::from($output_doc),
                typing: $output_ts,
            }),*],
            runner: CommandRunner::Sync($func_key::run),
            checker: $func_key::check,
            user_input_parser: $func_key::update_input_evaluation_results_from_user_input
        }
      )
    }
    };
}

#[macro_export]
macro_rules! define_async_command {
    ($func_key:ident => {
        name: $fn_name:expr,
        matcher: $matcher:expr,
        documentation: $doc:expr,
        // todo: add key field and use the input_name as the key, so the user can also provide a web-ui facing name
        inputs: [$($input_name:ident: { documentation: $input_doc:expr, typing: $input_ts:expr, optional: $optional:expr, interpolable: $interpolable:expr }),*],
        outputs: [$($output_name:ident: { documentation: $output_doc:expr, typing: $output_ts:expr }),*],
    }) => {
        {
            use txtx_addon_kit::types::commands::{PreCommandSpecification, CommandSpecification, CommandInput, CommandOutput, CommandRunner};
            PreCommandSpecification::Atomic(CommandSpecification {
                name: String::from($fn_name),
                matcher: String::from($matcher),
                documentation: String::from($doc),
                accepts_arbitrary_inputs: false,
                create_output_for_each_input: false,
                default_inputs: CommandSpecification::default_inputs(),
                inputs: vec![$(CommandInput {
                    name: String::from(stringify!($input_name)),
                    documentation: String::from($input_doc),
                    typing: $input_ts,
                    optional: $optional,
                    interpolable: $interpolable,
                }),*],
                outputs: vec![$(CommandOutput {
                    name: String::from(stringify!($output_name)),
                    documentation: String::from($output_doc),
                    typing: $output_ts,
                }),*],
                // runner:  txtx_addon_kit::types::commands::CommandRunner::Async(Box::new($func_key::run)),
                runner: CommandRunner::Async(Box::new($func_key::run)),
                checker: $func_key::check,
                user_input_parser: $func_key::update_input_evaluation_results_from_user_input,
            })
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
