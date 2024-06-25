use crate::manifest::ProtocolManifest;

pub fn build_manifest_data(manifest: &ProtocolManifest) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("project_name", &manifest.name)
        .expect("Failed to encode name")
        .insert_vec("runbooks", |functions_builder| {
            let mut runbooks = functions_builder;
            for runbook_spec in manifest.runbooks.iter() {
                runbooks = runbooks.push_map(|function| {
                    function
                        .insert_str("name", &runbook_spec.name)
                        .insert_str(
                            "description",
                            &runbook_spec
                                .description
                                .as_ref()
                                .unwrap_or(&"".to_string())
                                .to_string(),
                        )
                        .insert_str("location", &runbook_spec.location)
                });
            }
            runbooks
        });

    let data = doc_builder.build();
    data
}

pub fn build_runbook_data(runbook_name: &str) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("runbook_name", &runbook_name)
        .expect("Failed to encode name");

    let data = doc_builder.build();
    data
}
