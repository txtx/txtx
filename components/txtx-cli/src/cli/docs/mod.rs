use std::path::PathBuf;

use super::{Context, GetDocumentation};
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::kit::types::commands::{CommandInput, CommandOutput, PreCommandSpecification};
use txtx_core::kit::{Addon, DEFAULT_ADDON_DOCUMENTATION_TEMPLATE};
use txtx_core::std::StdAddon;

pub async fn handle_docs_command(_cmd: &GetDocumentation, _ctx: &Context) -> Result<(), String> {
    let std: Box<dyn Addon> = Box::new(StdAddon::new());
    let stacks: Box<dyn Addon> = Box::new(StacksNetworkAddon::new());
    let addons = vec![&std, &stacks];
    display_documentation(&addons);
    generate_mdx(&addons);
    Ok(())
}

pub fn generate_mdx(addons: &Vec<&Box<dyn Addon>>) {
    use std::fs::File;

    let mut path = PathBuf::new();
    path.push("doc");
    path.push("addons");
    for addon in addons.iter() {
        let mut addon_path = path.clone();
        addon_path.push(format!("{}.mdx", addon.get_namespace()));
        let mut doc_file = File::create(addon_path).expect("creation failed");

        let doc_data = build_addon_doc_data(&addon);

        let template = mustache::compile_str(&DEFAULT_ADDON_DOCUMENTATION_TEMPLATE)
            .expect("Failed to compile template");

        template
            .render_data(&mut doc_file, &doc_data)
            .expect("Failed to render template");

        // let mut inputs: HashMap<String, &str> = HashMap::new();
        // inputs.insert("addon_name".into(), addon.get_name());
        // inputs.insert("addon_description".into(), addon.get_description());

        // mustache::Template::

        // let content = strfmt(include_str!("templates/addon_header.mdx"), &inputs)
        //     .expect("unable to interpolate template");
        // // Write metadata

        // println!("{:?}", content);
        // data_file.write(content.as_bytes()).expect("write failed");
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
            let args = function
                .inputs
                .iter()
                .map(|a| a.name.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("{}", yellow!(format!("{}({})", function.name, args)));
            println!("{}", function.documentation);
            println!("\nInputs:");
            for input in function.inputs.iter() {
                println!(
                    "- {} ({:?}): {}",
                    input.name, input.typing, input.documentation
                );
            }
            println!(
                "\nOutput ({:?}): {}",
                function.output.typing, function.output.documentation
            );
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
                    for input in spec
                        .parts
                        .first()
                        .unwrap()
                        .expect_atomic_specification()
                        .inputs
                        .iter()
                    {
                        let required = if input.optional { "" } else { "*" };
                        println!(
                            "- {}{} ({:?}): {}",
                            input.name, required, input.typing, input.documentation
                        );
                    }

                    println!("\nOutputs:");
                    for output in spec
                        .parts
                        .last()
                        .unwrap()
                        .expect_atomic_specification()
                        .outputs
                        .iter()
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

fn insert_inputs_from_spec(
    input_spec: &CommandInput,
    input_builder: mustache::MapBuilder,
) -> mustache::MapBuilder {
    input_builder
        .insert_str("name", &input_spec.name)
        .insert_str(
            "requirementStatus",
            match input_spec.optional {
                true => "optional",
                false => "required",
            },
        )
        .insert_str("documentation", &input_spec.documentation)
        .insert_str("type", format!("{:?}", input_spec.typing))
}

fn insert_outputs_from_spec(
    output_spec: &CommandOutput,
    output_builder: mustache::MapBuilder,
) -> mustache::MapBuilder {
    output_builder
        .insert_str("name", &output_spec.name)
        .insert_str("documentation", &output_spec.documentation)
        .insert_str("type", format!("{:?}", output_spec.typing))
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
                            insert_inputs_from_spec(&input_spec, input_builder)
                        });
                    }
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
                            insert_inputs_from_spec(&input_spec, input_builder)
                        });
                    }
                    inputs_builder
                })
                .insert_vec("outputs", |mut outputs_builder| {
                    let outputs_spec = spec.parts.last().unwrap().expect_atomic_specification();
                    for input_spec in outputs_spec.inputs.iter() {
                        outputs_builder = outputs_builder.push_map(|input_builder| {
                            insert_inputs_from_spec(&input_spec, input_builder)
                        });
                    }
                    outputs_builder
                })
        }
    }
}

fn build_addon_doc_data(addon: &Box<dyn Addon>) -> mustache::Data {
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
                                        .insert_str("type", format!("{:?}", input_spec.typing))
                                });
                            }
                            inputs
                        })
                        .insert_str("output_documentation", &function_spec.output.documentation)
                        .insert_str("output_type", format!("{:?}", function_spec.output.typing))
                });
            }
            functions
        })
        .insert_vec("actions", |mut actions| {
            for action_spec in addon.get_actions() {
                actions = actions.push_map(|action| insert_data_from_spec(&action_spec, action));
            }
            actions
        })
        .insert_vec("wallets", |wallets_builder| {
            let mut wallets = wallets_builder;
            for wallet_spec in addon.get_wallets().iter() {
                wallets = wallets.push_map(|function| {
                    function
                        .insert_str("name", &wallet_spec.name)
                        .insert_str("documentation", &wallet_spec.documentation)
                        .insert_str("example", &wallet_spec.example)
                        .insert_vec("inputs", |inputs_builder| {
                            let mut inputs = inputs_builder;
                            for input_spec in wallet_spec.inputs.iter() {
                                inputs = inputs.push_map(|input| {
                                    input
                                        .insert_str("name", &input_spec.name)
                                        .insert_str("documentation", &input_spec.documentation)
                                        .insert_str("type", format!("{:?}", input_spec.typing))
                                });
                            }
                            inputs
                        })
                        .insert_vec("outputs", |mut outputs_builder| {
                            for output_spec in wallet_spec.outputs.iter() {
                                outputs_builder = outputs_builder.push_map(|output_builder| {
                                    insert_outputs_from_spec(&output_spec, output_builder)
                                });
                            }
                            outputs_builder
                        })
                });
            }
            wallets
        });
    let data = doc_builder.build();
    data
}
