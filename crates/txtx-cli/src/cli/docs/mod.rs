use std::fs::File;
use std::path::PathBuf;

use super::{Context, GetDocumentation};
use itertools::Itertools;
use serde_json::json;
use txtx_addon_network_evm::EvmNetworkAddon;
#[cfg(feature = "ovm")]
use txtx_addon_network_ovm::OvmNetworkAddon;
#[cfg(feature = "stacks")]
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_addon_network_svm::SvmNetworkAddon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::kit::helpers::fs::FileLocation;
use txtx_core::kit::indexmap::IndexMap;
use txtx_core::kit::types::commands::{CommandOutput, PreCommandSpecification};
use txtx_core::kit::types::functions::FunctionSpecification;
use txtx_core::kit::{
    Addon, DEFAULT_ADDON_ACTIONS_TEMPLATE, DEFAULT_ADDON_FUNCTIONS_TEMPLATE,
    DEFAULT_ADDON_OVERVIEW_TEMPLATE, DEFAULT_ADDON_WALLETS_TEMPLATE,
};
use txtx_core::mustache;
use txtx_core::std::commands::actions::http;
use txtx_core::std::functions::{assertions, base64, crypto, hash, hex, json, list, operators};
use txtx_core::std::StdAddon;
use txtx_gql::kit::types::commands::{PostConditionEvaluatableInput, PreConditionEvaluatableInput};
use txtx_gql::kit::types::types::Type;
use txtx_gql::kit::types::EvaluatableInput;

pub async fn handle_docs_command(_cmd: &GetDocumentation, _ctx: &Context) -> Result<(), String> {
    let std: Box<dyn Addon> = Box::new(StdAddon::new());
    let evm: Box<dyn Addon> = Box::new(EvmNetworkAddon::new());
    let svm: Box<dyn Addon> = Box::new(SvmNetworkAddon::new());
    let telegram: Box<dyn Addon> = Box::new(TelegramAddon::new());

    let addons = vec![&std, &evm, &svm, &telegram];
    #[cfg(feature = "ovm")]
    let ovm: Box<dyn Addon> = Box::new(OvmNetworkAddon::new());
    #[cfg(feature = "ovm")]
    addons.push(&ovm);
    #[cfg(feature = "stacks")]
    let stacks: Box<dyn Addon> = Box::new(OvmNetworkAddon::new());
    #[cfg(feature = "stacks")]
    addons.push(&stacks);

    display_documentation(&addons);
    generate_mdx(&addons);
    generate_json(&addons).map_err(|e| format!("Failed to generate JSON documentation: {}", e))?;
    Ok(())
}

pub fn generate_json(addons: &Vec<&Box<dyn Addon>>) -> Result<(), String> {
    let mut path = PathBuf::new();
    path.push("doc");
    path.push("addons");
    std::fs::create_dir_all(&path).map_err(|e| format!("Failed to create directories: {}", e))?;
    path.push("actions.json");

    let mut docs = IndexMap::new();
    for addon in addons.into_iter() {
        let mut actions = vec![];
        for action in addon.get_actions().iter() {
            let command = match action {
                PreCommandSpecification::Atomic(spec) => json!(spec),
                PreCommandSpecification::Composite(spec) => {
                    return Err(format!(
                        "Composite action '{}' is not supported in JSON output",
                        spec.name
                    ))
                }
            };
            actions.push(command);
        }
        let addon_ns = addon.get_namespace();
        docs.insert(addon_ns.to_string(), actions);
    }
    let file = FileLocation::from_path(path);
    let content = json!(docs);
    let formatted_content =
        serde_json::to_string_pretty(&content).expect("unable to pretty print docs");
    let _ = file.write_content(formatted_content.as_bytes());
    return Ok(());
}

pub fn generate_mdx(addons: &Vec<&Box<dyn Addon>>) {
    let mut path = PathBuf::new();
    path.push("doc");
    path.push("addons");
    for addon in addons.iter() {
        let mut addon_path = path.clone();
        let addon_ns = addon.get_namespace();
        addon_path.push(addon_ns);

        if addon_ns == "std" {
            generate_std_mdx(addon, addon_path);
        } else {
            generate_addon_mdx(addon, addon_path);
        }
    }
}

pub fn generate_std_mdx(addon: &Box<dyn Addon>, addon_path: PathBuf) {
    // functions
    {
        let map = vec![
            ("json", "JSON", json::JSON_FUNCTIONS.clone()),
            ("hex", "Hex", hex::FUNCTIONS.clone()),
            ("operators", "Operator", operators::OPERATORS_FUNCTIONS.clone()),
            ("crypto", "Crypto", crypto::FUNCTIONS.clone()),
            ("list", "List", list::LIST_FUNCTIONS.clone()),
            ("base64", "Base64", base64::FUNCTIONS.clone()),
            ("hash", "Hash", hash::FUNCTIONS.clone()),
            ("assertions", "Assertions", assertions::FUNCTIONS.clone()),
        ];
        for (path, title, fns) in map.into_iter() {
            let mut page_path = addon_path.clone();
            page_path.push("functions");
            page_path.push(path);
            page_path.push("page.mdx");
            let mut doc_file = File::create(&page_path)
                .expect(format!("creation failed for {}", page_path.display()).as_str());
            let doc_data = build_addon_function_group_doc_data(&addon, title, fns);
            let template = mustache::compile_str(&DEFAULT_ADDON_FUNCTIONS_TEMPLATE)
                .expect("Failed to compile template");
            template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
        }
    }
    // actions
    {
        let map = vec![("http", "HTTP", vec![http::SEND_HTTP_REQUEST.clone()])];
        for (path, title, actions) in map.into_iter() {
            let mut page_path = addon_path.clone();
            page_path.push("actions");
            page_path.push(path);
            page_path.push("page.mdx");
            let mut doc_file = File::create(page_path).expect("creation failed");
            let doc_data = build_addon_action_group_doc_data(&addon, title, actions);
            let template = mustache::compile_str(&DEFAULT_ADDON_ACTIONS_TEMPLATE)
                .expect("Failed to compile template");
            template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
        }
    }
    // an overview page for each category (functions/actions)
    {
        let mut page_path = addon_path.clone();
        page_path.push("functions/overview/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_addon_overview_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_OVERVIEW_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
    {
        let mut page_path = addon_path.clone();
        page_path.push("actions/overview/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_addon_overview_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_OVERVIEW_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
}

pub fn generate_addon_mdx(addon: &Box<dyn Addon>, addon_path: PathBuf) {
    // functions
    {
        let mut page_path = addon_path.clone();
        page_path.push("functions/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_addon_function_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_FUNCTIONS_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
    // actions
    {
        let mut page_path = addon_path.clone();
        page_path.push("actions/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_addon_action_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_ACTIONS_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
    // signers
    {
        let mut page_path = addon_path.clone();
        page_path.push("signers/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_signers_action_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_WALLETS_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
    // overview page
    {
        let mut page_path = addon_path.clone();
        page_path.push("overview/page.mdx");
        let mut doc_file = File::create(page_path).expect("creation failed");
        let doc_data = build_addon_overview_doc_data(&addon);
        let template = mustache::compile_str(&DEFAULT_ADDON_OVERVIEW_TEMPLATE)
            .expect("Failed to compile template");
        template.render_data(&mut doc_file, &doc_data).expect("Failed to render template");
    }
}

pub fn display_documentation(addons: &Vec<&Box<dyn Addon>>) {
    for addon in addons.iter() {
        println!(
            "{}",
            purple!(format!(
                "############################\n# {} Addon\n############################\n",
                addon.get_name()
            )),
        );
        println!("{}", blue!(format!("{}", "Functions\n")),);
        for function in addon.get_functions() {
            let args =
                function.inputs.iter().map(|a| a.name.to_string()).collect::<Vec<_>>().join(", ");
            println!("{}", yellow!(format!("{}({})", function.name, args)));
            println!("{}", function.documentation);
            println!("\nInputs:");
            for input in function.inputs.iter() {
                println!("- {} ({:?}): {}", input.name, input.typing, input.documentation);
            }
            println!("\nOutput ({:?}): {}", function.output.typing, function.output.documentation);
            println!("\nExample\n{}\n\n", function.example);
        }

        println!("{}", blue!(format!("{}", "Actions\n")),);
        for action in addon.get_actions() {
            match action {
                PreCommandSpecification::Atomic(spec) => {
                    println!("{}", yellow!(format!("{}", spec.name)));
                    println!("{}", spec.documentation);
                    println!("\nInputs (* required):");
                    for input in spec.inputs.iter() {
                        let required = if input.optional { "" } else { "*" };
                        println!(
                            "- {}{} ({:?}): {}",
                            input.name, required, input.typing, input.documentation
                        );
                    }

                    println!("\nOutputs:");
                    for output in spec.outputs.iter() {
                        println!(
                            "- {} ({:?}): {}",
                            output.name, output.typing, output.documentation
                        );
                    }
                    println!("{}", spec.example);
                }
                PreCommandSpecification::Composite(spec) => {
                    println!("{}", yellow!(format!("{}", spec.name)));
                    println!("{}", spec.documentation);
                    println!("\nInputs (* required):");
                    for input in
                        spec.parts.first().unwrap().expect_atomic_specification().inputs.iter()
                    {
                        let required = if input.optional { "" } else { "*" };
                        println!(
                            "- {}{} ({:?}): {}",
                            input.name, required, input.typing, input.documentation
                        );
                    }

                    println!("\nOutputs:");
                    for output in
                        spec.parts.last().unwrap().expect_atomic_specification().outputs.iter()
                    {
                        println!(
                            "- {} ({:?}): {}",
                            output.name, output.typing, output.documentation
                        );
                    }
                }
            }
            println!("\n");
        }
    }
}

fn insert_inputs_from_evaluatable_input(
    evaluatable_input: &dyn EvaluatableInput,
    input_builder: mustache::MapBuilder,
) -> mustache::MapBuilder {
    let input_docs = match &evaluatable_input.typing() {
        Type::Object(object_definition) => {
            format!(
                "{} This is an object type containing the keys:\n{}",
                evaluatable_input.documentation(),
                object_definition.join_documentation(0)
            )
        }
        Type::Map(object_definition) => {
            format!(
                "{} This is a map type containing the keys:\n{}",
                evaluatable_input.documentation(),
                object_definition.join_documentation(0)
            )
        }
        _ => evaluatable_input.documentation().clone(),
    };

    input_builder
        .insert_str("name", &evaluatable_input.name())
        .insert_str(
            "requirementStatus",
            match evaluatable_input.optional() {
                true => "optional",
                false => "required",
            },
        )
        .insert_str("documentation", &input_docs)
        .insert_str("type", &evaluatable_input.typing().to_string())
}

fn insert_outputs_from_spec(
    output_spec: &CommandOutput,
    output_builder: mustache::MapBuilder,
) -> mustache::MapBuilder {
    output_builder
        .insert_str("name", &output_spec.name)
        .insert_str("documentation", &output_spec.documentation)
        .insert_str("type", &output_spec.typing.to_string())
}

fn insert_data_from_spec(
    command_spec: &PreCommandSpecification,
    map_builder: mustache::MapBuilder,
) -> mustache::MapBuilder {
    match command_spec {
        PreCommandSpecification::Atomic(spec) => {
            map_builder
                .insert_str("name", &spec.name)
                .insert_str("matcher", &spec.matcher)
                .insert_str("example", &spec.example)
                .insert_str("documentation", &spec.documentation)
                // .insert_str("example", spec.example)
                // .insert_str("snippet", spec.snippet)
                .insert_vec("inputs", |mut inputs_builder| {
                    for input_spec in spec.inputs.iter() {
                        inputs_builder = inputs_builder.push_map(|input_builder| {
                            insert_inputs_from_evaluatable_input(input_spec, input_builder)
                        });
                    }

                    inputs_builder = inputs_builder.push_map(|input_builder| {
                        insert_inputs_from_evaluatable_input(
                            &PreConditionEvaluatableInput::new(),
                            input_builder,
                        )
                    });
                    inputs_builder = inputs_builder.push_map(|input_builder| {
                        insert_inputs_from_evaluatable_input(
                            &PostConditionEvaluatableInput::new(),
                            input_builder,
                        )
                    });

                    inputs_builder
                })
                .insert_vec("outputs", |mut outputs_builder| {
                    for output_spec in spec.outputs.iter() {
                        outputs_builder = outputs_builder.push_map(|output_builder| {
                            insert_outputs_from_spec(&output_spec, output_builder)
                        });
                    }
                    outputs_builder
                })
        }
        PreCommandSpecification::Composite(spec) => {
            map_builder
                .insert_str("name", &spec.name)
                .insert_str("matcher", &spec.matcher)
                .insert_str("documentation", &spec.documentation)
                .insert_str("example", &spec.example)
                // .insert_str("example", spec.example)
                // .insert_str("snippet", spec.snippet)
                .insert_vec("inputs", |mut inputs_builder| {
                    let inputs_spec = spec.parts.first().unwrap().expect_atomic_specification();
                    for input_spec in inputs_spec.inputs.iter() {
                        inputs_builder = inputs_builder.push_map(|input_builder| {
                            insert_inputs_from_evaluatable_input(input_spec, input_builder)
                        });
                    }
                    inputs_builder = inputs_builder.push_map(|input_builder| {
                        insert_inputs_from_evaluatable_input(
                            &PreConditionEvaluatableInput::new(),
                            input_builder,
                        )
                    });
                    inputs_builder = inputs_builder.push_map(|input_builder| {
                        insert_inputs_from_evaluatable_input(
                            &PostConditionEvaluatableInput::new(),
                            input_builder,
                        )
                    });
                    inputs_builder
                })
                .insert_vec("outputs", |mut outputs_builder| {
                    let outputs_spec = spec.parts.last().unwrap().expect_atomic_specification();
                    for input_spec in outputs_spec.inputs.iter() {
                        outputs_builder = outputs_builder.push_map(|input_builder| {
                            insert_inputs_from_evaluatable_input(input_spec, input_builder)
                        });
                    }
                    outputs_builder
                })
        }
    }
}

fn build_addon_function_group_doc_data(
    addon: &Box<dyn Addon>,
    title: &str,
    function_specs: Vec<FunctionSpecification>,
) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &format!("{} {}", addon.get_name(), title))
        .expect("Failed to encode name")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description")
        .insert_vec("functions", |functions_builder| {
            let mut functions = functions_builder;
            for function_spec in function_specs.iter() {
                functions = functions.push_map(|function| {
                    function
                        .insert_str("name", &function_spec.name)
                        .insert_str("documentation", &function_spec.documentation)
                        .insert_str("example", &function_spec.example)
                        .insert_str("snippet", &function_spec.snippet)
                        .insert_vec("inputs", |inputs_builder| {
                            let mut inputs = inputs_builder;
                            for input_spec in function_spec.inputs.iter() {
                                inputs = inputs.push_map(|input| {
                                    input
                                        .insert_str("name", &input_spec.name)
                                        .insert_str("documentation", &input_spec.documentation)
                                        .insert_str(
                                            "requirementStatus",
                                            match input_spec.optional {
                                                true => "optional",
                                                false => "required",
                                            },
                                        )
                                        .insert_str(
                                            "type",
                                            &input_spec
                                                .typing
                                                .iter()
                                                .map(|t| t.to_string())
                                                .join(" | "),
                                        )
                                });
                            }
                            inputs
                        })
                        .insert_str("output_documentation", &function_spec.output.documentation)
                        .insert_str("type", &function_spec.output.typing.to_string())
                });
            }
            functions
        });
    let data = doc_builder.build();
    data
}

fn build_addon_action_group_doc_data(
    addon: &Box<dyn Addon>,
    title: &str,
    actions_specs: Vec<PreCommandSpecification>,
) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &format!("{} {}", addon.get_name(), title))
        .expect("Failed to encode name")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description")
        .insert_vec("actions", |mut actions| {
            for action_spec in actions_specs.clone().into_iter() {
                actions = actions.push_map(|action| insert_data_from_spec(&action_spec, action));
            }
            actions
        });
    let data = doc_builder.build();
    data
}

fn build_addon_overview_doc_data(addon: &Box<dyn Addon>) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &addon.get_name())
        .expect("Failed to encode name")
        .insert("addon_description", &addon.get_description())
        .expect("Failed to encode description")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description");
    let data = doc_builder.build();
    data
}

fn build_addon_function_doc_data(addon: &Box<dyn Addon>) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &addon.get_name())
        .expect("Failed to encode name")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description")
        .insert_vec("functions", |functions_builder| {
            let mut functions = functions_builder;
            for function_spec in addon.get_functions().iter() {
                functions = functions.push_map(|function| {
                    function
                        .insert_str("name", &function_spec.name)
                        .insert_str("documentation", &function_spec.documentation)
                        .insert_str("example", &function_spec.example)
                        .insert_str("snippet", &function_spec.snippet)
                        .insert_vec("inputs", |inputs_builder| {
                            let mut inputs = inputs_builder;
                            for input_spec in function_spec.inputs.iter() {
                                inputs = inputs.push_map(|input| {
                                    input
                                        .insert_str("name", &input_spec.name)
                                        .insert_str("documentation", &input_spec.documentation)
                                        .insert_str(
                                            "requirementStatus",
                                            match input_spec.optional {
                                                true => "optional",
                                                false => "required",
                                            },
                                        )
                                        .insert_str(
                                            "type",
                                            &input_spec
                                                .typing
                                                .iter()
                                                .map(|t| t.to_string())
                                                .join(" | "),
                                        )
                                });
                            }
                            inputs
                        })
                        .insert_str("output_documentation", &function_spec.output.documentation)
                        .insert_str("type", &function_spec.output.typing.to_string())
                });
            }
            functions
        });
    let data = doc_builder.build();
    data
}

fn build_addon_action_doc_data(addon: &Box<dyn Addon>) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &addon.get_name())
        .expect("Failed to encode name")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description")
        .insert_vec("actions", |mut actions| {
            for action_spec in addon.get_actions() {
                actions = actions.push_map(|action| insert_data_from_spec(&action_spec, action));
            }
            actions
        });
    let data = doc_builder.build();
    data
}

fn build_signers_action_doc_data(addon: &Box<dyn Addon>) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("addon_name", &addon.get_name())
        .expect("Failed to encode name")
        .insert("addon_namespace", &addon.get_namespace())
        .expect("Failed to encode description")
        .insert_vec("signers", |signers_builder| {
            let mut signers = signers_builder;
            for signer_spec in addon.get_signers().iter() {
                signers = signers.push_map(|function| {
                    function
                        .insert_str("name", &signer_spec.name)
                        .insert_str("matcher", &signer_spec.matcher)
                        .insert_str("documentation", &signer_spec.documentation)
                        .insert_str("example", &signer_spec.example)
                        .insert_vec("inputs", |inputs_builder| {
                            let mut inputs = inputs_builder;
                            for input_spec in signer_spec.inputs.iter() {
                                inputs = inputs.push_map(|input| {
                                    input
                                        .insert_str("name", &input_spec.name)
                                        .insert_str("documentation", &input_spec.documentation)
                                        .insert_str(
                                            "requirementStatus",
                                            match input_spec.optional {
                                                true => "optional",
                                                false => "required",
                                            },
                                        )
                                        .insert_str("type", input_spec.typing.to_string())
                                });
                            }
                            inputs
                        })
                        .insert_vec("outputs", |mut outputs_builder| {
                            for output_spec in signer_spec.outputs.iter() {
                                outputs_builder = outputs_builder.push_map(|output_builder| {
                                    insert_outputs_from_spec(&output_spec, output_builder)
                                });
                            }
                            outputs_builder
                        })
                });
            }
            signers
        });
    let data = doc_builder.build();
    data
}
