use super::types::{PrimitiveValue, Value};
use crate::types::diagnostics::Diagnostic;
use std::collections::HashMap;
use test_case::test_case;

#[test_case(Value::string("test".to_string()))]
#[test_case(Value::uint(1))]
#[test_case(Value::int(1))]
#[test_case(Value::bool(true))]
#[test_case(Value::bool(false))]
#[test_case(Value::Primitive(PrimitiveValue::Null))]
#[test_case(Value::array(vec![Value::string("test".to_string()), Value::uint(1)]))]
#[test_case(Value::int(-1); "negative")]
#[test_case(Value::object(HashMap::from([
  ("key1".to_string(), Ok(Value::string("test".to_string()))),
  ("key2".to_string(), Ok(Value::uint(1))),
  ("error".to_string(), Err(Diagnostic::error_from_string("test".to_string()))),
  ])))]
fn it_values(value: Value) {
    let ser = serde_json::to_string(&value).unwrap();
    let de: Value = serde_json::from_str(&ser).unwrap();
    assert_eq!(de, value);
    println!("serialized: {}", ser);
    println!("deserialized:  {:?}", de);
}
