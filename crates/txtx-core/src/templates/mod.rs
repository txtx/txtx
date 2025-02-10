use crate::manifest::WorkspaceManifest;

pub const TXTX_MANIFEST_TEMPLATE: &str = include_str!("../templates/txtx.yml.mst");
pub const TXTX_README_TEMPLATE: &str = include_str!("../templates/readme.md.mst");
pub const TXTX_RUNBOOK_TEMPLATE: &str = include_str!("../templates/runbook.tx.mst");

pub fn build_manifest_data(manifest: &WorkspaceManifest) -> mustache::Data {
    let doc_builder = mustache::MapBuilder::new()
        .insert("double_open", &"{{")
        .expect("failed to encode open braces")
        .insert("double_close", &"}}")
        .expect("failed to encode close braces")
        .insert("workspace_name", &manifest.name)
        .expect("Failed to encode name")
        .insert("workspace_id", &manifest.id)
        .expect("Failed to encode id")
        .insert_vec("runbooks", |functions_builder| {
            let mut runbooks = functions_builder;
            for runbook_spec in manifest.runbooks.iter() {
                runbooks = runbooks.push_map(|function| {
                    function
                        .insert_str("name", &runbook_spec.name)
                        .insert_str("id", &runbook_spec.name)
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
        })
        .insert_vec("environments", |environment_builder| {
            let mut environments = environment_builder;
            for (name, environment_spec) in manifest.environments.iter() {
                environments = environments.push_map(|entry_builder| {
                    entry_builder.insert_str("name", name).insert_vec("values", |inputs_builder| {
                        let mut inputs = inputs_builder;
                        for (key, value) in environment_spec.iter() {
                            inputs = inputs.push_str(format!("{}: {}", key, value));
                        }
                        inputs
                    })
                });
            }
            environments
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
