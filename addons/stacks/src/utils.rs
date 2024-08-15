// use std::collections::HashMap;

// use txtx_addon_kit::{
//     types::{diagnostics::Diagnostic, types::Value},
//     AddonDefaults,
// };

// pub fn retrieve_string_value_from_args_using_defaults(
//     key: &str,
//     construct_name: &str,
//     args: &HashMap<String, Value>,
//     defaults: &AddonDefaults,
// ) -> Result<String, Diagnostic> {
//     let value = args
//         .get(key)
//         .and_then(|a| Some(a.expect_string()))
//         .or(defaults.keys.get(key).map(|x| x.as_str()))
//         .ok_or(Diagnostic::error_from_string(format!(
//             "command '{}': attribute '{}' is missing",
//             construct_name, key
//         )))
//         .map(|v| v.to_string());
//     value
// }
