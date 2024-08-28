use std::path::Path;

use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{Type, Value},
    AuthorizationContext,
};

use crate::typing::{Sp1Value, SP1_ELF};

const INFURA_API_KEY: &str = "";

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        GetElfFromSp1Project => {
            name: "get_elf_from_sp1_project",
            documentation: "Coming soon",
            example: indoc! {r#"
                        // Coming Soon
                        "#},
            inputs: [
                input: {
                    documentation: "Coming Soon",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "Coming Soon",
                typing: Type::addon(SP1_ELF)
            },
        }
    },];
}

#[derive(Clone)]
pub struct GetElfFromSp1Project;
impl FunctionImplementation for GetElfFromSp1Project {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let prefix = "command 'evm::get_elf_from_sp1_project'";
        let elf_file_loc = match args.get(0) {
            Some(Value::String(elf_path)) => {
                let path = Path::new(elf_path);
                if path.is_absolute() {
                    FileLocation::from_path(path.to_path_buf())
                } else {
                    let mut workspace_loc =
                        auth_ctx.workspace_location.get_parent_location().map_err(|e| {
                            diagnosed_error!(
                                "{}: unable to read workspace location: {}",
                                prefix,
                                e.to_string()
                            )
                        })?;
                    workspace_loc.append_path(&elf_path.to_string()).map_err(|e| {
                        diagnosed_error!("{}: invalid foundry manifest path: {}", prefix, e)
                    })?;
                    workspace_loc
                }
            }
            other => return Err(format_fn_error(&prefix, 1, "string", other)),
        };

        let elf_bytes = elf_file_loc.read_content().map_err(|e| {
            diagnosed_error!("{}: invalid ELF location {}: {}", prefix, elf_file_loc, e)
        })?;

        Ok(Sp1Value::elf(elf_bytes))
    }
}

fn format_fn_error(ctx: &str, position: u64, expected: &str, actual: Option<&Value>) -> Diagnostic {
    return diagnosed_error!(
        "'{}', argument position {:?}: expected {}, got {:?}",
        ctx,
        position,
        expected,
        actual.and_then(|v| Some(v.get_type()))
    );
}
