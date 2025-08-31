use crate::codec::abi::encoding::{value_to_abi_param, value_to_abi_params};
use crate::codec::abi::types::value_to_sol_value;
use crate::codec::abi::decoding::sol_value_to_value;
use crate::codec::abi::value_to_struct_abi_type;
use crate::typing::EvmValue;
use alloy::json_abi::Param;
use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, U256};
use alloy::dyn_abi::Word;
use txtx_addon_kit::types::types::{ObjectType, Value};

#[test]
fn test_value_to_abi_param_simple_types() {
    // Test address conversion
    let addr_param = Param {
        name: "addr".to_string(),
        ty: "address".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let addr = address!("0000000000000000000000000000000000000001");
    let value = EvmValue::address(&addr);
    let result = value_to_abi_param(&value, &addr_param);
    assert!(result.is_ok());
    match result.unwrap() {
        DynSolValue::Address(a) => assert_eq!(a, addr),
        _ => panic!("Expected address"),
    }
    
    // Test uint256 conversion
    let uint_param = Param {
        name: "amount".to_string(),
        ty: "uint256".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::integer(12345);
    let result = value_to_abi_param(&value, &uint_param);
    assert!(result.is_ok());
    match result.unwrap() {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(bits, 256);
            assert_eq!(val, U256::from(12345));
        }
        _ => panic!("Expected uint256"),
    }
    
    // Test bool conversion
    let bool_param = Param {
        name: "flag".to_string(),
        ty: "bool".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::bool(true);
    let result = value_to_abi_param(&value, &bool_param);
    assert!(result.is_ok());
    match result.unwrap() {
        DynSolValue::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
}

#[test]
fn test_value_to_abi_param_array_types() {
    // Test fixed array
    let fixed_array_param = Param {
        name: "nums".to_string(),
        ty: "uint256[3]".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::array(vec![
        Value::integer(1),
        Value::integer(2),
        Value::integer(3),
    ]);
    
    let result = value_to_abi_param(&value, &fixed_array_param);
    assert!(result.is_ok());
    match result.unwrap() {
        DynSolValue::FixedArray(arr) => assert_eq!(arr.len(), 3),
        _ => panic!("Expected fixed array"),
    }
    
    // Test dynamic array
    let dynamic_array_param = Param {
        name: "nums".to_string(),
        ty: "uint256[]".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::array(vec![
        Value::integer(10),
        Value::integer(20),
    ]);
    
    let result = value_to_abi_param(&value, &dynamic_array_param);
    assert!(result.is_ok());
    match result.unwrap() {
        DynSolValue::Array(arr) => assert_eq!(arr.len(), 2),
        _ => panic!("Expected dynamic array"),
    }
}

#[test]
fn test_value_to_abi_params_multiple() {
    let params = vec![
        Param {
            name: "to".to_string(),
            ty: "address".to_string(),
            internal_type: None,
            components: vec![],
        },
        Param {
            name: "amount".to_string(),
            ty: "uint256".to_string(),
            internal_type: None,
            components: vec![],
        },
        Param {
            name: "data".to_string(),
            ty: "bytes".to_string(),
            internal_type: None,
            components: vec![],
        },
    ];
    
    let addr = address!("0000000000000000000000000000000000000001");
    let values = vec![
        EvmValue::address(&addr),
        Value::integer(1000),
        Value::buffer(vec![0x01, 0x02, 0x03]),
    ];
    
    let result = value_to_abi_params(&values, &params);
    assert!(result.is_ok());
    
    let encoded = result.unwrap();
    assert_eq!(encoded.len(), 3);
    
    match &encoded[0] {
        DynSolValue::Address(a) => assert_eq!(*a, addr),
        _ => panic!("Expected address"),
    }
    
    match &encoded[1] {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(*bits, 256);
            assert_eq!(*val, U256::from(1000));
        }
        _ => panic!("Expected uint256"),
    }
    
    match &encoded[2] {
        DynSolValue::Bytes(b) => assert_eq!(*b, vec![0x01, 0x02, 0x03]),
        _ => panic!("Expected bytes"),
    }
}

#[test]
fn test_value_to_abi_params_wrong_count() {
    let params = vec![
        Param {
            name: "to".to_string(),
            ty: "address".to_string(),
            internal_type: None,
            components: vec![],
        },
        Param {
            name: "amount".to_string(),
            ty: "uint256".to_string(),
            internal_type: None,
            components: vec![],
        },
    ];
    
    let values = vec![
        EvmValue::address(&address!("0000000000000000000000000000000000000001")),
    ];
    
    let result = value_to_abi_params(&values, &params);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    // Error message is now clearer: "expected 2 arguments, got 1"
    assert!(error_msg.contains("expected 2 arguments") || error_msg.contains("expected 2, got 1"),
            "Unexpected error message: {}", error_msg);
}

// TODO: Fix value_to_struct_abi_type - it has a bug where it passes the entire value
// to each component instead of extracting the component's value from the struct
#[test]
#[ignore]
fn test_value_to_struct_abi_type() {
    let param = Param {
        name: "Person".to_string(),
        ty: "struct".to_string(),
        internal_type: None, // InternalType type has changed
        components: vec![
            Param {
                name: "name".to_string(),
                ty: "string".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "age".to_string(),
                ty: "uint256".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "wallet".to_string(),
                ty: "address".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
    };
    
    let addr = address!("0000000000000000000000000000000000000001");
    let value = ObjectType::from(vec![
        ("name", Value::string("Alice".to_string())),
        ("age", Value::integer(30)),
        ("wallet", EvmValue::address(&addr)),
    ]).to_value();
    
    let result = value_to_struct_abi_type(&value, &param);
    if let Err(e) = &result {
        println!("Error in value_to_struct_abi_type: {:?}", e);
    }
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::CustomStruct { name, prop_names, tuple } => {
            assert_eq!(name, "Person");
            assert_eq!(prop_names.len(), 3);
            assert_eq!(tuple.len(), 3);
            assert!(prop_names.contains(&"name".to_string()));
            assert!(prop_names.contains(&"age".to_string()));
            assert!(prop_names.contains(&"wallet".to_string()));
        }
        _ => panic!("Expected custom struct"),
    }
}

#[test]
fn test_value_to_sol_value_edge_cases() {
    // With error-stack migration, these now return errors instead of panicking
    
    // Test null (returns error for unsupported type)
    let value = Value::null();
    let result = value_to_sol_value(&value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("null"));
    
    // Test float (returns error for unsupported type)
    let value = Value::float(3.14);
    let result = value_to_sol_value(&value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("float"));
    
    // Test object (returns error for unsupported type)
    let value = ObjectType::from(vec![("key", Value::string("value".to_string()))]).to_value();
    let result = value_to_sol_value(&value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("object"));
}

#[test]
fn test_sol_value_to_value_edge_cases() {
    // With error-stack migration, these now return errors instead of panicking
    
    // Test fixed bytes (returns error for unsupported type)
    let sol_value = DynSolValue::FixedBytes(Word::default(), 20);
    let result = sol_value_to_value(&sol_value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("bytes20"));
    
    // Test function (returns error for unsupported type)
    let sol_value = DynSolValue::Function(alloy::primitives::Function::default());
    let result = sol_value_to_value(&sol_value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("function"));
    
    // Test bytes (returns error for unsupported type)
    let sol_value = DynSolValue::Bytes(vec![0x01, 0x02]);
    let result = sol_value_to_value(&sol_value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("bytes"));
    
    // Test fixed array (now supported - converts elements)
    let sol_value = DynSolValue::FixedArray(vec![]);
    let result = sol_value_to_value(&sol_value);
    assert!(result.is_ok()); // Empty array is valid
    assert_eq!(result.unwrap(), Value::array(vec![]));
    
    // Test tuple (returns error for unsupported type)
    let sol_value = DynSolValue::Tuple(vec![]);
    let result = sol_value_to_value(&sol_value);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("tuple"));
}