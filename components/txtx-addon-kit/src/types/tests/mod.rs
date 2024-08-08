use super::types::{PrimitiveValue, TypeSpecification, Value};
use crate::types::types::default_checker;
use test_case::test_case;

lazy_static! {
    static ref BYTES: Vec<u8> = hex::decode("ffffff").unwrap();
    pub static ref TYPING: TypeSpecification = TypeSpecification {
        id: "typing".to_string(),
        documentation: "test".to_string(),
        checker: default_checker()
    };
}

#[test_case(Value::string("test".to_string()))]
#[test_case(Value::uint(1))]
#[test_case(Value::int(1))]
#[test_case(Value::bool(true))]
#[test_case(Value::bool(false))]
#[test_case(Value::Primitive(PrimitiveValue::Null))]
#[test_case(Value::array(vec![Value::string("test".to_string()), Value::uint(1)]))]
#[test_case(Value::int(-1); "negative")]
#[test_case({
    let mut o = indexmap::IndexMap::new();
     o.insert("key1".to_string(), Value::string("test".to_string()));
     o.insert("key2".to_string(), Value::uint(1));
     Value::Object(o)
})]
#[test_case(Value::buffer(BYTES.clone(), TYPING.clone()))]
#[test_case(Value::addon(Value::buffer(BYTES.clone(), TYPING.clone()), TYPING.clone()))]
fn it_serdes_values(value: Value) {
    let ser = serde_json::to_string(&value).unwrap();
    println!("\nserialized: {}", ser);
    let de: Value = serde_json::from_str(&ser).unwrap();
    println!("deserialized:  {:?}\n", de);
    assert_eq!(de, value);
}
