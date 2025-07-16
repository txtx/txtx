use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::errors::{
    TxtxError, ErrorAttachments, ErrorLocation, ErrorDocumentation, ActionContext, TypeMismatchInfo
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::Did;

fn main() {
    println!("\n🔍 Error-Stack Demo for txtx\n");
    println!("This demo shows how the new error-stack integration provides rich error context.\n");
    
    // Demo 1: Missing input error
    println!("1️⃣  Missing Input Error:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━");
    demo_missing_input();
    
    // Demo 2: Type mismatch error
    println!("\n2️⃣  Type Mismatch Error:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━");
    demo_type_mismatch();
    
    // Demo 3: Validation error with full context
    println!("\n3️⃣  Validation Error with Full Context:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    demo_validation_error();
    
    // Demo 4: Error propagation chain
    println!("\n4️⃣  Error Propagation Chain:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    demo_error_chain();
}

fn demo_missing_input() {
    let store = ValueStore::new("demo", &Did::zero());
    
    let result = get_required_value(&store, "api_key");
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            // Show how to extract specific attachments
            if let Some(docs) = error.downcast_ref::<ErrorDocumentation>() {
                println!("\n📚 Documentation:");
                println!("   {}", docs.help);
                if let Some(example) = &docs.example {
                    println!("   Example: {}", example);
                }
            }
        }
    }
}

fn demo_type_mismatch() {
    let mut store = ValueStore::new("demo", &Did::zero());
    store.inputs.insert("port", Value::String("not-a-number".to_string()));
    
    let result = get_port_number(&store);
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            // Show type mismatch details
            if let Some(type_info) = error.downcast_ref::<TypeMismatchInfo>() {
                println!("\n❌ Type Error Details:");
                println!("   Field: {}", type_info.field);
                println!("   Expected: {}", type_info.expected);
                println!("   Actual: {}", type_info.actual);
            }
        }
    }
}

fn demo_validation_error() {
    let mut store = ValueStore::new("demo", &Did::zero());
    store.inputs.insert("address", Value::String("invalid-address".to_string()));
    
    let result = deploy_contract(&store, "MyContract");
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            // Show all attached context
            if let Some(location) = error.downcast_ref::<ErrorLocation>() {
                println!("\n📍 Location: {}:{}:{}", location.file, location.line, location.column);
            }
            
            if let Some(action) = error.downcast_ref::<ActionContext>() {
                println!("\n🎯 Action Context:");
                println!("   Action: {}", action.action_name);
                println!("   Namespace: {}", action.namespace);
                println!("   Construct: {}", action.construct_id);
            }
        }
    }
}

fn demo_error_chain() {
    let result = complex_blockchain_operation();
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            // The Debug format shows the full error chain
            println!("{:#?}", error);
        }
    }
}

// Helper functions that demonstrate error creation

fn get_required_value(store: &ValueStore, key: &str) -> Result<Value, Report<TxtxError>> {
    store.inputs.get_value(key)
        .ok_or_else(|| Report::new(TxtxError::MissingInput)
            .attach_printable(format!("Required configuration '{}' not found", key)))
        .map(|v| v.clone())
        .with_documentation(format!("The '{}' field is required for this operation", key))
        .with_example(format!("{} = \"your-value-here\"", key))
}

fn get_port_number(store: &ValueStore) -> Result<i32, Report<TxtxError>> {
    let value = store.inputs.get_value("port")
        .ok_or_else(|| Report::new(TxtxError::MissingInput))?;
    
    match value {
        Value::Integer(n) => Ok(*n as i32),
        _ => Err(Report::new(TxtxError::TypeMismatch)
            .attach(TypeMismatchInfo {
                field: "port".to_string(),
                expected: "integer".to_string(),
                actual: match value {
                    Value::String(_) => "string",
                    Value::Bool(_) => "boolean",
                    _ => "unknown",
                }.to_string(),
            })
            .attach_printable("Port must be a number"))
            .with_documentation("Port numbers must be integers between 1 and 65535")
            .with_example("port = 8080")
    }
}

fn validate_ethereum_address(address: &str) -> Result<(), Report<TxtxError>> {
    if !address.starts_with("0x") || address.len() != 42 {
        return Err(Report::new(TxtxError::Validation)
            .attach_printable(format!("Invalid Ethereum address: {}", address)))
            .with_location("deploy.tx", 15, 10)
            .with_documentation("Ethereum addresses must start with '0x' and be 42 characters total")
            .with_example("address = \"0x742d35Cc6634C0532925a3b844Bc9e7595f89590\"")
            .with_link("https://docs.txtx.io/ethereum/addresses");
    }
    Ok(())
}

fn deploy_contract(store: &ValueStore, contract_name: &str) -> Result<String, Report<TxtxError>> {
    let address = get_required_value(store, "address")?
        .as_string()
        .ok_or_else(|| Report::new(TxtxError::TypeMismatch))?
        .to_string();
    
    validate_ethereum_address(&address)
        .change_context(TxtxError::Execution)
        .attach_printable(format!("Failed to deploy contract '{}'", contract_name))
        .with_action_context("deploy_contract", "evm", "construct_12345")?;
    
    Ok("0xCONTRACT_ADDRESS".to_string())
}

fn complex_blockchain_operation() -> Result<(), Report<TxtxError>> {
    // Simulate a multi-step operation where step 2 fails
    step_1_parse_config()
        .attach_printable("Step 1: Parse configuration")?;
    
    step_2_connect_network()
        .attach_printable("Step 2: Connect to network")?;
    
    step_3_execute_transaction()
        .attach_printable("Step 3: Execute transaction")?;
    
    Ok(())
}

fn step_1_parse_config() -> Result<(), Report<TxtxError>> {
    Ok(()) // Success
}

fn step_2_connect_network() -> Result<(), Report<TxtxError>> {
    // Simulate network failure
    Err(Report::new(TxtxError::Network)
        .attach_printable("Connection refused: mainnet.infura.io:8545")
        .attach_printable("Timeout after 30 seconds")
        .attach(ErrorLocation {
            file: "network_config.tx".to_string(),
            line: 5,
            column: 12,
        })
        .attach(ErrorDocumentation {
            help: "Check your network configuration and ensure the RPC endpoint is accessible".to_string(),
            example: Some("rpc_url = \"https://mainnet.infura.io/v3/YOUR_PROJECT_ID\"".to_string()),
            link: Some("https://docs.txtx.io/troubleshooting/network".to_string()),
        }))
}

fn step_3_execute_transaction() -> Result<(), Report<TxtxError>> {
    Ok(()) // Would succeed if we got here
}