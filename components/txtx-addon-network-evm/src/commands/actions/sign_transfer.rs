use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
use std::str::FromStr;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest,
};
use txtx_addon_kit::types::wallets::{
    return_synchronous_ok, WalletActionsFutureResult, WalletInstance, WalletSignFutureResult,
    WalletsState,
};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::codec::CommonTransactionFields;
use crate::constants::{
    NETWORK_ID, RPC_API_URL, SIGNED_TRANSACTION_BYTES, UNSIGNED_TRANSACTION_BYTES,
};
use crate::rpc::EVMRpc;
use crate::typing::ETH_ADDRESS;

lazy_static! {
    pub static ref SIGN_EVM_TRANSFER: PreCommandSpecification = define_command! {
      SignEVMTransfer => {
          name: "Sign EVM Transfer Transaction",
          matcher: "sign_transfer",
          documentation: "The `evm::sign_transaction` action signs an unsigned transaction with the supplied wallet data.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            from: {
                documentation: "A reference to a wallet construct, which will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            to: {
                documentation: "The address of the recipient of the transfer.",
                typing: Type::addon(ETH_ADDRESS.clone()),
                optional: false,
                interpolable: true
            },
            amount: {
                documentation: "The amount, in WEI, to transfer.",
                typing: Type::uint(),
                optional: false,
                interpolable: true
            },
            type: {
                documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            max_fee_per_gas: {
                documentation: "Sets the max fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            max_priority_fee_per_gas: {
                documentation: "Sets the max priority fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            chain_id: {
                documentation: "The chain id.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            network_id: {
                documentation: "The network id.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_limit: {
                documentation: "Sets the maximum amount of gas that should be used to execute this transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_price: {
                documentation: "Sets the gas price for Legacy transactions.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            rpc_api_url: {
              documentation: "The URL of the EVM API used to fetch and fill transaction data.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            },
            depends_on: {
              documentation: "References another command's outputs, preventing this command from executing until the referenced command is successful.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              network_id: {
                  documentation: "Network id of the signed transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct SignEVMTransfer;
impl CommandImplementation for SignEVMTransfer {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_signed_executability(
        uuid: &ConstructUuid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        mut wallets: WalletsState,
    ) -> WalletActionsFutureResult {
        use alloy::{consensus::Transaction, hex, network::TransactionBuilder};

        use crate::{
            codec::get_typed_transaction_bytes,
            constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE, TRANSACTION_FROM},
            typing::ETH_TRANSACTION,
        };

        let signer = args.get_expected_string(TRANSACTION_FROM).unwrap();
        let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());
        let wallet = wallets_instances.get(&wallet_uuid).unwrap().clone();
        let uuid = uuid.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let args = args.clone();
        let defaults = defaults.clone();
        let execution_context = execution_context.clone();
        let wallets_instances = wallets_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();
            if let Some(_) =
                wallet_state.get_scoped_value(&uuid.value().to_string(), SIGNED_TRANSACTION_BYTES)
            {
                return Ok((wallets, wallet_state, Actions::none()));
            }

            let transaction = build_unsigned_transfer(&wallet_state, &spec, &args, &defaults)
                .await
                .map_err(|diag| (wallets.clone(), wallet_state.clone(), diag))?;

            println!("unsigned evm tx: {:?}", transaction);
            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    wallets.clone(),
                    wallet_state.clone(),
                    diagnosed_error!("command 'evm::sign_transfer': {e}"),
                )
            })?;
            let transaction = transaction.build_unsigned().unwrap();
            println!("tx bytes: 0x{}", hex::encode(&bytes));

            let payload = Value::buffer(bytes, ETH_TRANSACTION.clone());

            wallet_state.insert_scoped_value(
                &uuid.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            wallets.push_wallet_state(wallet_state);

            if execution_context.review_input_values {
                actions.push_panel("Transaction Signing", "");
                actions.push_sub_group(vec![
                    ActionItemRequest::new(
                        &Some(uuid.value()),
                        "".into(),
                        Some(format!("Check account nonce")),
                        ActionItemStatus::Todo,
                        ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(),
                            value: Value::uint(transaction.nonce()),
                        }),
                        ACTION_ITEM_CHECK_NONCE,
                    ),
                    ActionItemRequest::new(
                        &Some(uuid.value()),
                        "µSTX".into(),
                        Some(format!("Check transaction fee")),
                        ActionItemStatus::Todo,
                        ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(),
                            value: Value::uint(transaction.gas_limit().try_into().unwrap()), // todo
                        }),
                        ACTION_ITEM_CHECK_FEE,
                    ),
                ])
            }

            let wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();
            let description = args
                .get_expected_string("description")
                .ok()
                .and_then(|d| Some(d.to_string()));
            let (wallets, wallet_state, mut wallet_actions) =
                (wallet.specification.check_signability)(
                    &uuid,
                    &instance_name,
                    &description,
                    &payload,
                    &wallet.specification,
                    &args,
                    wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &execution_context,
                )?;
            actions.append(&mut wallet_actions);
            Ok((wallets, wallet_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        mut wallets: WalletsState,
    ) -> WalletSignFutureResult {
        let signer = args.get_expected_string("from").unwrap();
        let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());
        let wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_ok(wallets, wallet_state, result);
        }

        let wallet = wallets_instances.get(&wallet_uuid).unwrap();

        let payload = wallet_state
            .get_scoped_value(&uuid.value().to_string(), UNSIGNED_TRANSACTION_BYTES)
            .unwrap()
            .clone();

        let title = args
            .get_expected_string("description")
            .unwrap_or("New Transaction".into());

        let res = (wallet.specification.sign)(
            uuid,
            title,
            &payload,
            &wallet.specification,
            &args,
            wallet_state,
            wallets,
            wallets_instances,
            &defaults,
        );
        res
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_transfer(
    wallet_state: &ValueStore,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<TransactionRequest, Diagnostic> {
    use crate::{
        codec::{build_unsigned_transaction, TransactionType},
        constants::{
            CHAIN_ID, GAS_LIMIT, NONCE, TRANSACTION_AMOUNT, TRANSACTION_TO, TRANSACTION_TYPE,
        },
    };

    let from = wallet_state.get_expected_value("signer_address")?;

    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_string(CHAIN_ID, &defaults)?;

    let to = args.get_expected_value(TRANSACTION_TO)?;
    let amount = args.get_expected_uint(TRANSACTION_AMOUNT)?;
    let gas_limit = args.get_value(GAS_LIMIT).map(|v| v.expect_uint());
    let nonce = args.get_value(NONCE).map(|v| v.expect_uint());
    let tx_type = TransactionType::from_some_value(args.get_string(TRANSACTION_TYPE))?;

    let rpc: EVMRpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::sign_transfer': {}", e))?;

    let common = CommonTransactionFields {
        to: Some(to.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: None,
        deploy_code: None,
    };

    let tx = build_unsigned_transaction(rpc, args, common)
        .await
        .map_err(|e| diagnosed_error!("command: 'evm::sign_transfer': {e}"))?;

    Ok(tx)
}
