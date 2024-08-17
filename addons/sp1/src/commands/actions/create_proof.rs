use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref CREATE_PROOF: PreCommandSpecification = define_command! {
        CreateProof => {
            name: "Create ZK Proof",
            matcher: "create_proof",
            documentation: "The `sp1::create_proof` action...",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                program: {
                    documentation: "The compiled program ELF.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                inputs: {
                    documentation: "The programs's inputs.",
                    typing: Type::array(Type::string()),
                    optional: false,
                    interpolable: true
                },
                verify: {
                    documentation: "Verify proof locally.",
                    typing: Type::bool(),
                    optional: false,
                    interpolable: true
                } 
            ],
            outputs: [
                verification_key: {
                    documentation: "Coming soon.",
                    typing: Type::buffer()
                },
                proof: {
                    documentation: "Coming soon.",
                    typing: Type::buffer()
                },
                public_values: {
                    documentation: "Coming soon.",
                    typing: Type::buffer()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                // Coming Soon.
            "#},
        }
    };
}
pub struct CreateProof;
impl CommandImplementation for CreateProof {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    #[cfg(not(feature = "wasm"))]
    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        _outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        use sp1_sdk::{HashableKey, NetworkProver, ProverClient, SP1Stdin};

        use crate::typing::Sp1Value;

        let elf = inputs.get_expected_buffer_bytes("program")?;
        let program_inputs = inputs
            .get_expected_array("inputs")?
            .iter()
            .map(|input| input.expect_string().to_string())
            .collect::<Vec<String>>();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let network_prover = NetworkProver::new_from_key("<SP1_PRIVATE_KEY>");
            let client = ProverClient {
                prover: Box::new(network_prover)
            };

            let mut stdin = SP1Stdin::new();
            for input in program_inputs {
                stdin.write(&input);
            }

            let (pk, vk) = client.setup(&elf);

            let proof = client.prove(&pk, stdin).plonk().run().map_err(|e| {
                diagnosed_error!("command 'sp1::create_proof': failed to generate proof")
            })?;

            client.verify(&proof, &vk).map_err(|e| {
                diagnosed_error!("command 'sp1::create_proof': failed to verify proof")
            })?;
            
            result.outputs.insert("verification_key".into(), Sp1Value::verification_key(vk.bytes32().into_bytes()));

            result.outputs.insert("proof".into(), Sp1Value::proof(proof.bytes()));

            result.outputs.insert("public_values".into(), Sp1Value::public_values(proof.public_values.to_vec()));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub fn sleep_ms(millis: u64) -> () {
    let t = std::time::Duration::from_millis(millis);
    std::thread::sleep(t);
}
