use crate::codec::abi::encoding::*;
use alloy::json_abi::{Function, JsonAbi, Param, StateMutability, Constructor};
use alloy::primitives::{address, U256};
use alloy::dyn_abi::DynSolValue;
use txtx_addon_kit::types::types::Value;
use crate::typing::EvmValue;
use std::collections::VecDeque;
use std::num::NonZeroUsize;

fn create_simple_abi() -> JsonAbi {
    // Create a simple ABI with a transfer function
    let transfer_fn = Function {
        name: "transfer".to_string(),
        inputs: vec![
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
        ],
        outputs: vec![
            Param {
                name: "".to_string(),
                ty: "bool".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        state_mutability: StateMutability::NonPayable,
    };
    
    let mut abi = JsonAbi::default();
    abi.functions.insert("transfer".to_string(), vec![transfer_fn]);
    abi
}

#[test]
fn test_value_to_abi_function_args_valid() {
    let abi = create_simple_abi();
    let to_addr = address!("0000000000000000000000000000000000000001");
    
    // Create function arguments: [address, uint256]
    let args = Value::array(vec![
        EvmValue::address(&to_addr),
        Value::integer(1000000),
    ]);
    
    let result = value_to_abi_function_args("transfer", &args, &abi);
    assert!(result.is_ok());
    
    let encoded = result.unwrap();
    assert_eq!(encoded.len(), 2);
    
    // Check first argument is address
    match &encoded[0] {
        DynSolValue::Address(addr) => assert_eq!(*addr, to_addr),
        _ => panic!("Expected address"),
    }
    
    // Check second argument is uint256
    match &encoded[1] {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(*bits, 256);
            assert_eq!(*val, U256::from(1000000));
        }
        _ => panic!("Expected uint256"),
    }
}

#[test]
fn test_value_to_abi_function_args_wrong_count() {
    let abi = create_simple_abi();
    
    // Create wrong number of arguments
    let args = Value::array(vec![
        EvmValue::address(&address!("0000000000000000000000000000000000000001")),
    ]);
    
    let result = value_to_abi_function_args("transfer", &args, &abi);
    assert!(result.is_err());
    // With improved error messages: "expected 2 arguments, got 1"
    assert!(result.unwrap_err().to_string().contains("expected 2 arguments, got 1"));
}

#[test]
fn test_value_to_abi_function_args_function_not_found() {
    let abi = create_simple_abi();
    let args = Value::array(vec![]);
    
    let result = value_to_abi_function_args("nonexistent", &args, &abi);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_value_to_abi_constructor_args() {
    let constructor = Constructor {
        inputs: vec![
            Param {
                name: "owner".to_string(),
                ty: "address".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "initialSupply".to_string(),
                ty: "uint256".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        state_mutability: StateMutability::NonPayable,
    };
    
    let owner = address!("0000000000000000000000000000000000000001");
    let args = Value::array(vec![
        EvmValue::address(&owner),
        Value::integer(1000000000),
    ]);
    
    let result = value_to_abi_constructor_args(&args, &constructor);
    assert!(result.is_ok());
    
    let encoded = result.unwrap();
    assert_eq!(encoded.len(), 2);
}

#[test]
fn test_value_to_primitive_abi_type_address() {
    let param = Param {
        name: "addr".to_string(),
        ty: "address".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let addr = address!("0000000000000000000000000000000000000001");
    let value = EvmValue::address(&addr);
    
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Address(a) => assert_eq!(a, addr),
        _ => panic!("Expected address"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_uint256() {
    let param = Param {
        name: "amount".to_string(),
        ty: "uint256".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::integer(123456789);
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(bits, 256);
            assert_eq!(val, U256::from(123456789));
        }
        _ => panic!("Expected uint256"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_uint8() {
    let param = Param {
        name: "val".to_string(),
        ty: "uint8".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::integer(255);
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(bits, 8);
            assert_eq!(val, U256::from(255));
        }
        _ => panic!("Expected uint8"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_bytes32() {
    let param = Param {
        name: "hash".to_string(),
        ty: "bytes32".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let bytes = vec![0u8; 32];
    let value = Value::buffer(bytes.clone());
    
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::FixedBytes(word, size) => {
            assert_eq!(size, 32);
            assert_eq!(word.as_slice(), &bytes[..]);
        }
        _ => panic!("Expected bytes32"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_bool() {
    let param = Param {
        name: "flag".to_string(),
        ty: "bool".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::bool(true);
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    let value = Value::bool(false);
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Bool(b) => assert!(!b),
        _ => panic!("Expected bool"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_string() {
    let param = Param {
        name: "name".to_string(),
        ty: "string".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let value = Value::string("Hello, World!".to_string());
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::String(s) => assert_eq!(s, "Hello, World!"),
        _ => panic!("Expected string"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_bytes() {
    let param = Param {
        name: "data".to_string(),
        ty: "bytes".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let bytes = vec![0x01, 0x02, 0x03, 0x04];
    let value = Value::buffer(bytes.clone());
    
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Bytes(b) => assert_eq!(b, bytes),
        _ => panic!("Expected bytes"),
    }
}

#[test]
fn test_value_to_primitive_abi_type_tuple() {
    let param = Param {
        name: "pair".to_string(),
        ty: "tuple".to_string(),
        internal_type: None,
        components: vec![
            Param {
                name: "first".to_string(),
                ty: "uint256".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "second".to_string(),
                ty: "address".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
    };
    
    let addr = address!("0000000000000000000000000000000000000001");
    let value = Value::array(vec![
        Value::integer(123),
        EvmValue::address(&addr),
    ]);
    
    let result = value_to_primitive_abi_type(&value, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Tuple(tuple) => {
            assert_eq!(tuple.len(), 2);
            match &tuple[0] {
                DynSolValue::Uint(val, bits) => {
                    assert_eq!(*bits, 256);
                    assert_eq!(*val, U256::from(123));
                }
                _ => panic!("Expected uint256 in tuple"),
            }
            match &tuple[1] {
                DynSolValue::Address(a) => assert_eq!(*a, addr),
                _ => panic!("Expected address in tuple"),
            }
        }
        _ => panic!("Expected tuple"),
    }
}

#[test]
fn test_value_to_array_abi_type_fixed() {
    use std::num::NonZeroUsize;
    
    let param = Param {
        name: "arr".to_string(),
        ty: "uint256[3]".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let values = vec![
        Value::integer(1),
        Value::integer(2),
        Value::integer(3),
    ];
    
    let mut sizes = VecDeque::from(vec![Some(NonZeroUsize::new(3).unwrap())]);
    let result = value_to_array_abi_type(&values, &mut sizes, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::FixedArray(arr) => {
            assert_eq!(arr.len(), 3);
            for (i, val) in arr.iter().enumerate() {
                match val {
                    DynSolValue::Uint(v, bits) => {
                        assert_eq!(*bits, 256);
                        assert_eq!(*v, U256::from(i + 1));
                    }
                    _ => panic!("Expected uint256 in array"),
                }
            }
        }
        _ => panic!("Expected fixed array"),
    }
}

#[test]
fn test_value_to_array_abi_type_dynamic() {
    let param = Param {
        name: "arr".to_string(),
        ty: "uint256[]".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    let values = vec![
        Value::integer(10),
        Value::integer(20),
        Value::integer(30),
        Value::integer(40),
    ];
    
    let mut sizes = VecDeque::from(vec![None]);
    let result = value_to_array_abi_type(&values, &mut sizes, &param);
    assert!(result.is_ok());
    
    match result.unwrap() {
        DynSolValue::Array(arr) => {
            assert_eq!(arr.len(), 4);
            match &arr[0] {
                DynSolValue::Uint(v, _) => assert_eq!(*v, U256::from(10)),
                _ => panic!("Expected uint256"),
            }
            match &arr[3] {
                DynSolValue::Uint(v, _) => assert_eq!(*v, U256::from(40)),
                _ => panic!("Expected uint256"),
            }
        }
        _ => panic!("Expected dynamic array"),
    }
}

#[test]
fn test_value_to_array_abi_type_wrong_size() {
    use std::num::NonZeroUsize;
    
    let param = Param {
        name: "arr".to_string(),
        ty: "uint256[3]".to_string(),
        internal_type: None,
        components: vec![],
    };
    
    // Wrong number of elements
    let values = vec![
        Value::integer(1),
        Value::integer(2),
    ];
    
    let mut sizes = VecDeque::from(vec![Some(NonZeroUsize::new(3).unwrap())]);
    let result = value_to_array_abi_type(&values, &mut sizes, &param);
    assert!(result.is_err());
    // With improved error messages: "expected array of length 3, got 2"
    assert!(result.unwrap_err().to_string().contains("expected array of length 3, got 2"));
}