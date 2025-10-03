use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
struct Component {
    name: String,
    container: String,
    description: String,
    technology: String,
    relationships: Vec<(String, String)>, // (target, description)
    uses: Vec<(String, String)>,          // (target, description)
    responsibilities: Vec<String>,
}

fn main() {
    let project_root = std::env::current_dir().expect("Failed to get current directory");
    let crates_dir = project_root.join("crates");
    let output_file = project_root.join("docs/architecture/linter/workspace-generated.dsl");

    eprintln!("🔍 Scanning for C4 annotations in Rust code...");

    let mut components: HashMap<String, Component> = HashMap::new();

    // Walk through all Rust files
    for entry in WalkDir::new(&crates_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path = entry.path();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract annotations
        if let Some(component) = extract_component(&content, path) {
            eprintln!("  Found: {} in {}", component.name, path.display());
            components.insert(component.name.clone(), component);
        }
    }

    if components.is_empty() {
        eprintln!("❌ No C4 annotations found");
        std::process::exit(1);
    }

    eprintln!("📝 Generating Structurizr DSL...");
    eprintln!("  Found {} components", components.len());

    // Generate DSL
    let dsl = generate_dsl(&components);

    fs::write(&output_file, dsl).expect("Failed to write output file");

    eprintln!("✅ Generated: {}", output_file.display());
}

fn extract_component(content: &str, _path: &Path) -> Option<Component> {
    let re_component = Regex::new(r"@c4-component\s+(.+)").unwrap();
    let re_container = Regex::new(r"@c4-container\s+(.+)").unwrap();
    let re_description = Regex::new(r"@c4-description\s+(.+)").unwrap();
    let re_technology = Regex::new(r"@c4-technology\s+(.+)").unwrap();
    let re_relationship = Regex::new(r#"@c4-relationship\s+"([^"]+)"\s+"([^"]+)""#).unwrap();
    let re_uses = Regex::new(r#"@c4-uses\s+(\S+)(?:\s+"([^"]+)")?"#).unwrap();
    let re_responsibility = Regex::new(r"@c4-responsibility\s+(.+)").unwrap();

    // Check if this file has a component annotation
    let component_name = re_component
        .captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())?;

    let mut component = Component {
        name: component_name,
        container: re_container
            .captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default(),
        description: re_description
            .captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default(),
        technology: re_technology
            .captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Rust".to_string()),
        ..Default::default()
    };

    // Extract relationships
    for cap in re_relationship.captures_iter(content) {
        let rel_type = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let target = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        component.relationships.push((rel_type.to_string(), target.to_string()));
    }

    // Extract uses
    for cap in re_uses.captures_iter(content) {
        let target = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let desc = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        component.uses.push((target.to_string(), desc.to_string()));
    }

    // Extract responsibilities
    for cap in re_responsibility.captures_iter(content) {
        let resp = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        component.responsibilities.push(resp.to_string());
    }

    Some(component)
}

fn generate_dsl(components: &HashMap<String, Component>) -> String {
    let mut dsl = String::new();

    dsl.push_str("# Auto-generated from C4 annotations in Rust source code\n");
    dsl.push_str("# DO NOT EDIT - Regenerate with: just arch-c4\n");
    dsl.push_str("# For hand-written architecture including dynamic views, see workspace.dsl\n\n");
    dsl.push_str("workspace \"txtx Linter Architecture (Generated from Code)\" \"Auto-generated from C4 annotations in Rust source\" {\n\n");
    dsl.push_str("    model {\n");
    dsl.push_str("        user = person \"Developer\" \"Writes txtx runbooks and manifests\"\n\n");
    dsl.push_str("        txtxSystem = softwareSystem \"txtx CLI\" \"Command-line tool for runbook execution and validation\" {\n");

    // Group components by container
    let mut containers: HashMap<String, Vec<&Component>> = HashMap::new();
    for component in components.values() {
        if !component.container.is_empty() {
            containers
                .entry(component.container.clone())
                .or_default()
                .push(component);
        }
    }

    // Generate containers and components
    for (container_name, comps) in containers.iter() {
        let container_id = sanitize_id(container_name);
        dsl.push_str(&format!(
            "\n            {} = container \"{}\" \"Container for {} components\" \"Rust\" {{\n",
            container_id, container_name, container_name
        ));

        for comp in comps {
            let comp_id = sanitize_id(&comp.name);
            dsl.push_str(&format!(
                "                {} = component \"{}\" \"{}\" \"{}\"\n",
                comp_id, comp.name, comp.description, comp.technology
            ));

            // Add responsibilities as comments
            for resp in &comp.responsibilities {
                dsl.push_str(&format!("                // Responsibility: {}\n", resp));
            }
        }

        dsl.push_str("            }\n");
    }

    dsl.push_str("        }\n\n");
    dsl.push_str("        // Relationships\n");

    // Add relationships
    for component in components.values() {
        let source_id = sanitize_id(&component.name);

        for (rel_type, target) in &component.relationships {
            let target_id = sanitize_id(target);
            dsl.push_str(&format!(
                "        {} -> {} \"{}\"\n",
                source_id, target_id, rel_type
            ));
        }

        for (target, desc) in &component.uses {
            let target_id = sanitize_id(target);
            dsl.push_str(&format!(
                "        {} -> {} \"{}\"\n",
                source_id, target_id, desc
            ));
        }
    }

    dsl.push_str("    }\n\n");
    dsl.push_str("    views {\n");
    dsl.push_str("        systemContext txtxSystem \"SystemContext\" {\n");
    dsl.push_str("            include *\n");
    dsl.push_str("            autoLayout lr\n");
    dsl.push_str("        }\n\n");

    // Generate component views for each container
    for container_name in containers.keys() {
        let container_id = sanitize_id(container_name);
        dsl.push_str(&format!("        component {} {{\n", container_id));
        dsl.push_str("            include *\n");
        dsl.push_str("            autoLayout tb\n");
        dsl.push_str(&format!("            title \"{}\"\n", container_name));
        dsl.push_str("        }\n\n");
    }

    dsl.push_str("        styles {\n");
    dsl.push_str("            element \"Software System\" {\n");
    dsl.push_str("                background #1168bd\n");
    dsl.push_str("                color #ffffff\n");
    dsl.push_str("            }\n");
    dsl.push_str("            element \"Container\" {\n");
    dsl.push_str("                background #438dd5\n");
    dsl.push_str("                color #ffffff\n");
    dsl.push_str("            }\n");
    dsl.push_str("            element \"Component\" {\n");
    dsl.push_str("                background #85bbf0\n");
    dsl.push_str("                color #000000\n");
    dsl.push_str("            }\n");
    dsl.push_str("            element \"Person\" {\n");
    dsl.push_str("                shape person\n");
    dsl.push_str("                background #08427b\n");
    dsl.push_str("                color #ffffff\n");
    dsl.push_str("            }\n");
    dsl.push_str("        }\n\n");
    dsl.push_str("        theme default\n");
    dsl.push_str("    }\n");
    dsl.push_str("}\n");

    dsl
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
