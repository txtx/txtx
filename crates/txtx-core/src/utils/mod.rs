use kit::helpers::fs::FileLocation;

use crate::runbook::RunbookOutputs;

pub fn try_write_outputs_to_file(
    output_loc: &str,
    runbook_outputs: RunbookOutputs,
    workspace_location: &FileLocation,
    runbook_id: &str,
    environment: &str,
) -> Result<FileLocation, String> {
    let json_outputs = runbook_outputs.to_json();
    let output = serde_json::to_string_pretty(&json_outputs)
        .map_err(|e| format!("failed to serialize outputs: {e}"))?;

    let mut output_location = workspace_location
        .get_parent_location()
        .map_err(|e| format!("failed to write to output file: {e}"))?;
    output_location
        .append_path(&output_loc)
        .map_err(|e| format!("invalid output directory: {e}"))?;
    output_location
        .append_path(environment)
        .map_err(|e| format!("invalid output directory: {e}"))?;

    let now = chrono::Local::now();
    let formatted = now.format("%Y-%m-%d--%H-%M-%S").to_string();
    output_location
        .append_path(&format!("{}_{}.output.json", runbook_id, formatted))
        .map_err(|e| format!("invalid output file path: {e}"))?;

    output_location
        .write_content(output.as_bytes())
        .map_err(|e| format!("failed to write to output file: {e}"))?;

    Ok(output_location)
}
