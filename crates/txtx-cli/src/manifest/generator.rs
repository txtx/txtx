pub fn generate_manifest(protocol_name: &str) -> String {
    let conf = format!(
        r#"
---
name: "{protocol_name}"
"#,
        protocol_name = protocol_name,
    );
    conf
}
