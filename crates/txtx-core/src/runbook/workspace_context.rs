use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::runbook::embedded_runbook::EmbeddedRunbookInstanceBuilder;
use crate::runbook::RawHclContent;
use crate::std::commands;
use crate::types::PreConstructData;
use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};
use txtx_addon_kit::hcl::structure::BlockLabel;
use txtx_addon_kit::hcl::template::Element;
use txtx_addon_kit::hcl::Span;
use txtx_addon_kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use txtx_addon_kit::helpers::hcl::{
    visit_optional_untyped_attribute, visit_required_string_literal_attribute,
};
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::{CommandId, CommandInstance, CommandInstanceType};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::embedded_runbooks::{
    EmbeddedRunbookInputSpecification, EmbeddedRunbookInstance,
};
use txtx_addon_kit::types::package::Package;
use txtx_addon_kit::types::signers::SignerInstance;
use txtx_addon_kit::types::stores::AddonDefaults;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::namespace::Namespace;
use txtx_addon_kit::types::AddonInstance;
use txtx_addon_kit::types::{ConstructDid, ConstructId, Did, PackageDid, PackageId, RunbookId};

use super::{
    get_source_context_for_diagnostic, RunbookExecutionContext, RunbookGraphContext,
    RunbookSources, RuntimeContext,
};

pub enum ConstructInstanceType {
    Executable(CommandInstance),
    Signing(SignerInstance),
    Import,
    EmbeddedRunbook(EmbeddedRunbookInstance),
    Addon(AddonInstance),
}

#[derive(better_debug::BetterDebug, Clone)]
pub struct RunbookWorkspaceContext {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    pub packages: HashMap<PackageId, Package>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, signer, import, ...)
    pub constructs: HashMap<ConstructDid, ConstructId>,
    /// Lookup: Retrieve a construct did, given an environment variable name ('name' in env.name)
    pub top_level_inputs_did_lookup: BTreeMap<String, ConstructDid>,
    /// Lookup: Retrieve a value given an environment variable construct did
    pub top_level_inputs_values: BTreeMap<ConstructDid, Value>,
    /// Lookup: Retrieve an addon's defaults given a package and addon id
    pub addons_defaults: HashMap<(PackageDid, Namespace), AddonDefaults>,

    std_defaults: AddonDefaults,
}

impl RunbookWorkspaceContext {
    pub fn new(runbook_id: RunbookId) -> Self {
        Self {
            runbook_id,
            packages: HashMap::new(),
            constructs: HashMap::new(),
            top_level_inputs_did_lookup: BTreeMap::new(),
            top_level_inputs_values: BTreeMap::new(),
            addons_defaults: HashMap::new(),
            std_defaults: AddonDefaults::new("std"),
        }
    }

    pub fn get_addon_defaults(&self, key: &(PackageDid, Namespace)) -> &AddonDefaults {
        self.addons_defaults.get(key).unwrap_or(&self.std_defaults)
    }

    pub fn sorted_addons_defaults_fingerprints(
        &self,
    ) -> IndexMap<PackageDid, IndexMap<String, IndexMap<String, Did>>> {
        let mut addon_defaults: IndexMap<PackageDid, IndexMap<String, IndexMap<String, Did>>> =
            self.addons_defaults
                .clone()
                .into_iter()
                .map(|((package_did, addon_id), defaults)| {
                    let mut addon_defaults_values = IndexMap::from([(
                        addon_id.to_string(),
                        defaults
                            .store
                            .store
                            .into_iter()
                            .map(|(k, v)| (k, v.compute_fingerprint()))
                            .collect(),
                    )]);
                    addon_defaults_values.sort_keys();
                    (package_did, addon_defaults_values)
                })
                .collect();
        addon_defaults.sort_by(|a, _, b, _| a.0.cmp(&b.0));
        addon_defaults
    }

    pub async fn build_from_sources(
        &mut self,
        runbook_sources: &RunbookSources,
        runtime_context: &mut RuntimeContext,
        graph_context: &mut RunbookGraphContext,
        execution_context: &mut RunbookExecutionContext,
        environment_selector: &Option<String>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let mut sources = VecDeque::new();
        // todo(lgalabru): basing files_visited on path is fragile, we should hash file contents instead
        let mut files_visited = HashSet::new();
        for (location, (module_name, raw_content)) in runbook_sources.tree.iter() {
            files_visited.insert(location);
            sources.push_back((location.clone(), module_name.clone(), raw_content.clone()));
        }

        while let Some((location, package_name, raw_content)) = sources.pop_front() {
            let package_id = PackageId::from_file(&location, &self.runbook_id, &package_name)
                .map_err(|e| vec![e])?;

            let mut blocks =
                raw_content.into_blocks().map_err(|diag| vec![diag.location(&location)])?;

            while let Some(block) = blocks.pop_front() {
                match block.ident.value().as_str() {
                    "import" => {
                        // imports are the only constructs that we need to process in this step
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(
                                Diagnostic::error_from_string("import name missing".into())
                                    .location(&location),
                            );
                            continue;
                        };

                        let path = visit_required_string_literal_attribute("path", &block).unwrap(); // todo(lgalabru)
                        println!("Loading {} at path ({path})", name.to_string());

                        // todo(lgalabru): revisit this approach, filesystem access needs to be abstracted.
                        let mut imported_package_location =
                            location.get_parent_location().map_err(|e| {
                                vec![diagnosed_error!("{}", e.to_string()).location(&location)]
                            })?;

                        imported_package_location.append_path(&path).unwrap();

                        match std::fs::read_dir(imported_package_location.to_string()) {
                            Ok(_) => {
                                let files = get_txtx_files_paths(
                                    &imported_package_location.to_string(),
                                    environment_selector,
                                )
                                .map_err(|e| {
                                    vec![diagnosed_error!("{}", e.to_string())
                                        .location(&imported_package_location)]
                                })?;
                                for file_path in files.into_iter() {
                                    let file_location = FileLocation::from_path(file_path);
                                    if !files_visited.contains(&file_location) {
                                        let raw_content =
                                            RawHclContent::from_file_location(&file_location)
                                                .map_err(|diag| vec![diag])?;
                                        let module_name = name.to_string();
                                        sources.push_back((
                                            file_location,
                                            module_name,
                                            raw_content,
                                        ));
                                    }
                                }
                            }
                            Err(_) => {
                                if !files_visited.contains(&imported_package_location) {
                                    let raw_content = RawHclContent::from_file_location(&location)
                                        .map_err(|diag| vec![diag])?;
                                    let module_name = name.to_string();
                                    sources.push_back((
                                        imported_package_location.clone(),
                                        module_name,
                                        raw_content,
                                    ));
                                }
                            }
                        }

                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Import(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "variable" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(
                                Diagnostic::error_from_string("variable name missing".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Variable(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "module" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(
                                Diagnostic::error_from_string("module name missing".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Module(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "output" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(
                                Diagnostic::error_from_string("output name missing".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Output(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "action" => {
                        let (Some(command_name), Some(namespaced_action)) =
                            (block.labels.get(0), block.labels.get(1))
                        else {
                            diagnostics.push(
                                Diagnostic::error_from_string("invalid action syntax: expected `action \"action_name\" \"namespace::action\"".into())
                                    .location(&location),
                            );
                            continue;
                        };

                        let Some((namespace, command_id)) = namespaced_action.split_once("::")
                        else {
                            todo!("return diagnostic")
                        };

                        match runtime_context.addons_context.create_action_instance(
                            namespace,
                            command_id,
                            command_name.as_str(),
                            &package_id,
                            &block,
                            &location,
                        ) {
                            Ok(command_instance) => {
                                let _ = self.index_construct(
                                    command_name.to_string(),
                                    location.clone(),
                                    PreConstructData::Action(command_instance),
                                    &package_id,
                                    graph_context,
                                    execution_context,
                                );
                            }
                            Err(diagnostic) => {
                                let span =
                                    get_source_context_for_diagnostic(&diagnostic, runbook_sources);
                                diagnostics.push(
                                    diagnostic
                                        .location(&location)
                                        .set_span_range(block.span())
                                        .set_diagnostic_span(span),
                                );
                                continue;
                            }
                        };
                    }
                    "signer" => {
                        let (Some(signer_name), Some(namespaced_signer_cmd)) =
                            (block.labels.get(0), block.labels.get(1))
                        else {
                            diagnostics.push(
                                Diagnostic::error_from_string("signer syntax invalid".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        match runtime_context.addons_context.create_signer_instance(
                            &namespaced_signer_cmd.as_str(),
                            signer_name.as_str(),
                            &package_id,
                            &block,
                            &location,
                        ) {
                            Ok(signer_instance) => {
                                let _ = self.index_construct(
                                    signer_name.to_string(),
                                    location.clone(),
                                    PreConstructData::Signer(signer_instance),
                                    &package_id,
                                    graph_context,
                                    execution_context,
                                );
                            }
                            Err(diagnostic) => {
                                diagnostics.push(diagnostic);
                                continue;
                            }
                        }
                    }
                    "runbook" => {
                        let Some(runbook_name) = block.labels.get(0) else {
                            diagnostics.push(
                                Diagnostic::error_from_string("'runbook' syntax invalid".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        let runbook_name = runbook_name.to_string();
                        let embedded_runbook_location =
                            visit_required_string_literal_attribute("location", &block).unwrap();
                        println!("Loading {runbook_name} at path ({embedded_runbook_location})");

                        let imported_package_location =
                            location.get_parent_location().map_err(|e| {
                                vec![diagnosed_error!(
                                    "invalid runbook location: {}",
                                    e.to_string()
                                )
                                .location(&location)]
                            })?;

                        match FileLocation::try_parse(
                            &embedded_runbook_location,
                            Some(&imported_package_location),
                        ) {
                            None => {
                                diagnostics.push(diagnosed_error!(
                                    "failed to index embedded runbook ({}): could not find runbook at location {}",
                                    runbook_name,
                                    embedded_runbook_location
                                ));
                                continue;
                            }
                            Some(loc) => {
                                let embedded_runbook =
                                    EmbeddedRunbookInstanceBuilder::from_location(
                                        loc,
                                        &runbook_name.to_string(),
                                        &package_id,
                                        &block,
                                        &mut runtime_context.addons_context,
                                    )
                                    .await
                                    .map_err(|e| {
                                        vec![diagnosed_error!(
                                            "failed to index embedded runbook ({}): {}",
                                            runbook_name,
                                            e
                                        )]
                                    })?;

                                let _ = self.index_construct(
                                    runbook_name.to_string(),
                                    location.clone(),
                                    PreConstructData::EmbeddedRunbook(embedded_runbook),
                                    &package_id,
                                    graph_context,
                                    execution_context,
                                );
                            }
                        }
                    }
                    "addon" => {
                        let Some(BlockLabel::String(addon_id)) = block.labels.first() else {
                            diagnostics.push(
                                Diagnostic::error_from_string("addon name missing".into())
                                    .location(&location),
                            );
                            continue;
                        };
                        let _ = self.index_construct(
                            addon_id.to_string(),
                            location.clone(),
                            PreConstructData::Addon(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "flow" => {}
                    unknown => {
                        diagnostics.push(
                            Diagnostic::error_from_string(format!("unknown construct {}", unknown))
                                .location(&location),
                        );
                    }
                }
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    /// Creates a [ConstructDid] from the provided `key`. Indexes the `value` in the `top_level_inputs_values` map by the [ConstructDid].
    /// Indexes the [ConstructDid] in the `top_level_inputs_did_lookup` by the `key`.
    /// Returns the new [ConstructDid]
    pub fn index_top_level_input(&mut self, key: &str, value: &Value) -> ConstructDid {
        let construct_did =
            ConstructDid(Did::from_components(vec!["runbook_input".as_bytes(), key.as_bytes()]));
        self.top_level_inputs_values.insert(construct_did.clone(), value.clone());
        self.top_level_inputs_did_lookup.insert(key.to_string(), construct_did.clone());
        construct_did
    }

    pub fn index_package(&mut self, package_id: &PackageId) {
        loop {
            if let Some(_) = self.packages.get(&package_id) {
                break;
            }
            let package = Package::new(package_id);
            self.packages.insert(package_id.clone(), package);
            continue;
        }
    }

    pub fn index_flow_input(
        &mut self,
        input_name: &str,
        package_id: &PackageId,
        graph_context: &mut RunbookGraphContext,
    ) -> ConstructId {
        let package =
            self.packages.get_mut(&package_id).expect("internal error: unable to retrieve package");
        let construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_type: "flow_input".into(),
            construct_location: package_id.package_location.clone(),
            construct_name: input_name.to_string(),
        };
        let construct_did = construct_id.did();
        package.flow_inputs_dids.insert(construct_did.clone());
        package.flow_inputs_did_lookup.insert(input_name.to_string(), construct_did.clone());
        graph_context.index_construct(&construct_did);
        construct_id
    }

    fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_id: &PackageId,
        graph_context: &mut RunbookGraphContext,
        execution_context: &mut RunbookExecutionContext,
    ) -> ConstructId {
        let package =
            self.packages.get_mut(&package_id).expect("internal error: unable to retrieve package");
        let construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_type: construct_data.construct_type().into(),
            construct_location,
            construct_name: construct_name.clone(),
        };
        let construct_did = construct_id.did();
        self.constructs.insert(construct_did.clone(), construct_id.clone());
        let construct_instance_type = match construct_data {
            PreConstructData::Module(block) => {
                // if construct_name.eq("runbook") && self.runbook_metadata_construct_did.is_none() {
                //     self.runbook_metadata_construct_did = Some(construct_did.clone());
                // }
                package.modules_dids.insert(construct_did.clone());
                package.modules_did_lookup.insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_module_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: Namespace::from(&construct_name),
                    typing: CommandInstanceType::Module,
                })
            }
            PreConstructData::Variable(block) => {
                package.variables_dids.insert(construct_did.clone());
                package.variables_did_lookup.insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_variable_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: Namespace::from(&construct_name),
                    typing: CommandInstanceType::Variable,
                })
            }
            PreConstructData::Addon(block) => {
                package.addons_dids.insert(construct_did.clone());
                package.addons_did_lookup.insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Addon(AddonInstance {
                    block: block.clone(),
                    package_id: package_id.clone(),
                    addon_id: construct_name.clone(),
                })
            }
            PreConstructData::Output(block) => {
                package.outputs_dids.insert(construct_did.clone());
                package.outputs_did_lookup.insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_output_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: Namespace::from(&construct_name),
                    typing: CommandInstanceType::Output,
                })
            }
            PreConstructData::Import(_) => {
                package.imports_dids.insert(construct_did.clone());
                package.imports_did_lookup.insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Import
            }
            PreConstructData::Action(command_instance) => {
                package.commands_dids.insert(construct_did.clone());
                package
                    .commands_did_lookup
                    .insert(CommandId::Action(construct_name).to_string(), construct_did.clone());
                ConstructInstanceType::Executable(command_instance)
            }
            PreConstructData::Signer(signer_instance) => {
                package.signers_dids.insert(construct_did.clone());
                package.signers_did_lookup.insert(construct_name, construct_did.clone());
                ConstructInstanceType::Signing(signer_instance)
            }
            PreConstructData::EmbeddedRunbook(embedded_runbook) => {
                package.embedded_runbooks_dids.insert(construct_did.clone());
                package.embedded_runbooks_did_lookup.insert(construct_name, construct_did.clone());
                ConstructInstanceType::EmbeddedRunbook(embedded_runbook)
            }
            PreConstructData::Root => unreachable!(),
        };

        graph_context.index_construct(&construct_did);
        match construct_instance_type {
            ConstructInstanceType::Executable(instance) => {
                execution_context.commands_instances.insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Signing(instance) => {
                execution_context.signers_instances.insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Import => {}
            ConstructInstanceType::EmbeddedRunbook(runbook) => {
                execution_context.embedded_runbooks.insert(construct_did.clone(), runbook);
            }
            ConstructInstanceType::Addon(instance) => {
                execution_context.addon_instances.insert(construct_did.clone(), instance);
            }
        }

        construct_id
    }

    /// Iterates over the attributes of `command_instance` to see if any of the attributes reference a top level input.
    /// If so, it retrieves the value of the top level input and creates an [EmbeddedRunbookInputSpecification] with it.
    /// For example, the following command instance:
    /// ```hcl
    /// action "deploy" "evm::deploy_contract" {
    ///     ...
    ///     create2 {
    ///         salt = input.salt
    ///     }
    /// }
    /// ```
    /// would generate and embedded runbook input with the name `salt` and type [Type::String].
    pub fn get_embedded_runbook_input_from_command_instance_input_referencing_top_level_input(
        &self,
        command_instance: &CommandInstance,
    ) -> Vec<EmbeddedRunbookInputSpecification> {
        let mut embedded_runbook_inputs = vec![];
        for input in command_instance.specification.inputs.iter() {
            let res = visit_optional_untyped_attribute(&input.name, &command_instance.block);
            if let Some(expr) = res {
                if let Some(input_names) =
                    self.get_top_level_input_name_from_expression_reference(&expr)
                {
                    for input_name in input_names {
                        embedded_runbook_inputs.push(EmbeddedRunbookInputSpecification::new_value(
                            &input_name,
                            &input.typing,
                            &input.documentation,
                        ));
                    }
                }
            }
        }
        embedded_runbook_inputs
    }

    /// Iterates over the attributes of `addon_instance` to see if any of the attributes reference a top level input.
    /// If so, it retrieves the value of the top level input and creates an [EmbeddedRunbookInputSpecification] with it.
    /// For example, the following addon instance:
    /// ```hcl
    /// addon "evm" {
    ///     chain_id = input.chain_id
    /// }
    /// ```
    ///
    /// would generate and embedded runbook input with the name `chain_id` and type [Type::Integer].
    pub fn get_embedded_runbook_input_from_addon_instance_input_referencing_top_level_input(
        &self,
        addon_instance: &AddonInstance,
    ) -> Vec<EmbeddedRunbookInputSpecification> {
        let mut embedded_runbook_inputs = vec![];
        for attribute in addon_instance.block.body.attributes() {
            let expr = &attribute.value;
            if let Some(input_names) =
                self.get_top_level_input_name_from_expression_reference(&expr)
            {
                let addon_defaults = self.get_addon_defaults(&(
                    addon_instance.package_id.did(),
                    Namespace::from(addon_instance.addon_id.clone()),
                ));
                for input_name in input_names {
                    if let Some(value) = addon_defaults.store.get_value(&input_name) {
                        embedded_runbook_inputs.push(EmbeddedRunbookInputSpecification::new_value(
                            &input_name,
                            &value.get_type(),
                            &"".into(),
                        ));
                    }
                }
            }
        }
        embedded_runbook_inputs
    }

    /// Iterates over the attributes of `embedded_runbook_instance` to see if any of the attributes reference a top level input.
    /// If so, it retrieves the value of the top level input and creates an [EmbeddedRunbookInputSpecification] with it.
    /// For example, the following addon instance:
    /// ```hcl
    /// runbook "some_book" {
    ///     chain_id = input.chain_id
    /// }
    /// ```
    ///
    /// would generate and embedded runbook input with the name `chain_id` and type [Type::Integer].
    pub fn get_embedded_runbook_input_from_embedded_runbook_instance_input_referencing_top_level_input(
        &self,
        embedded_runbook_instance: &EmbeddedRunbookInstance,
    ) -> Vec<EmbeddedRunbookInputSpecification> {
        let mut embedded_runbook_inputs = vec![];
        for input in embedded_runbook_instance.specification.inputs.iter() {
            let EmbeddedRunbookInputSpecification::Value(input) = input else {
                continue;
            };
            let res =
                visit_optional_untyped_attribute(&input.name, &embedded_runbook_instance.block);
            if let Some(expr) = res {
                if let Some(input_names) =
                    self.get_top_level_input_name_from_expression_reference(&expr)
                {
                    for input_name in input_names {
                        embedded_runbook_inputs.push(EmbeddedRunbookInputSpecification::new_value(
                            &input_name,
                            &input.typing,
                            &input.documentation,
                        ));
                    }
                }
            }
        }
        embedded_runbook_inputs
    }

    fn get_top_level_input_name_from_expression_reference(
        &self,
        expression: &Expression,
    ) -> Option<Vec<String>> {
        if let Some(traversal) = expression.as_traversal() {
            let Some(root) = traversal.expr.as_variable() else {
                return None;
            };
            if root.eq_ignore_ascii_case("input") {
                let Some(TraversalOperator::GetAttr(value)) =
                    traversal.operators.first().map(|op| op.value())
                else {
                    return None;
                };
                let top_level_input_name = value.to_string();
                if let Some(_) = self.top_level_inputs_did_lookup.get(&top_level_input_name) {
                    return Some(vec![top_level_input_name]);
                };
            }
        } else if let Some(arr) = expression.as_array() {
            let mut res = vec![];
            for expr in arr.iter() {
                if let Some(mut input_name) =
                    self.get_top_level_input_name_from_expression_reference(expr)
                {
                    res.append(&mut input_name);
                }
            }
            return Some(res);
        } else if let Some(obj) = expression.as_object() {
            let mut res = vec![];
            for (_, object_value) in obj.iter() {
                if let Some(mut input_name) =
                    self.get_top_level_input_name_from_expression_reference(object_value.expr())
                {
                    res.append(&mut input_name);
                }
            }
            return Some(res);
        } else if let Some(string_template) = expression.as_string_template() {
            let mut res = vec![];
            for element in string_template.into_iter() {
                match element {
                    Element::Literal(_) => {}
                    Element::Interpolation(interpolation) => {
                        if let Some(mut input_name) = self
                            .get_top_level_input_name_from_expression_reference(&interpolation.expr)
                        {
                            res.append(&mut input_name);
                        }
                    }
                    Element::Directive(_) => {}
                }
            }
            return Some(res);
        }
        None
    }

    /// Expects `expression` to be a traversal and `package_did_source` to be indexed in the runbook's `packages`.
    /// Iterates over the operators of `expression` to see if any of the blocks it references are cached as a
    /// `module`, `output`, `input`, `action`, or `prompt` in the package.
    ///
    pub fn try_resolve_construct_reference_in_expression(
        &self,
        source_package_id: &PackageId,
        expression: &Expression,
    ) -> Result<Option<(ConstructDid, VecDeque<String>, VecDeque<Value>)>, String> {
        let Some(traversal) = expression.as_traversal() else {
            return Ok(None);
        };
        let Some(root) = traversal.expr.as_variable() else {
            if traversal.expr.is_func_call() {
                return Err("properties of function results cannot be referenced in-line; the function result must be stored in a command and referenced".into());
            }
            return Ok(None);
        };

        let mut subpath = VecDeque::new();

        let mut components = VecDeque::new();
        components.push_front(root.to_string());

        for op in traversal.operators.iter() {
            if let TraversalOperator::GetAttr(value) = op.value() {
                components.push_back(value.to_string());
            }
            if let TraversalOperator::Index(expr) = op.value() {
                match expr {
                    Expression::Number(value) => {
                        subpath.push_back(Value::integer(value.as_i64().unwrap().into()));
                    }
                    Expression::String(value) => {
                        subpath.push_back(Value::string(value.to_string()));
                    }
                    Expression::Bool(value) => {
                        subpath.push_back(Value::bool(**value));
                    }
                    _ => unimplemented!(),
                }
            }
        }

        let mut is_root = true;
        while let Some(component) = components.pop_front() {
            if is_root {
                if component.eq_ignore_ascii_case("input") {
                    let Some(env_variable_name) = components.pop_front() else {
                        continue;
                    };

                    if let Some(construct_did) =
                        self.top_level_inputs_did_lookup.get(&env_variable_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                let Some(current_package) = self.packages.get(source_package_id) else {
                    return Ok(None);
                };

                if component.eq_ignore_ascii_case("module") {
                    is_root = false;
                    let Some(module_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.modules_did_lookup.get(&module_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for outputs
                if component.eq_ignore_ascii_case("output") {
                    is_root = false;
                    let Some(output_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.outputs_did_lookup.get(&output_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for variables
                if component.eq_ignore_ascii_case("variable") {
                    is_root = false;
                    let Some(input_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.variables_did_lookup.get(&input_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for actions
                if component.eq_ignore_ascii_case("action") {
                    is_root = false;
                    let Some(action_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package
                        .commands_did_lookup
                        .get(&CommandId::Action(action_name).to_string())
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for signers
                if component.eq_ignore_ascii_case("signer") {
                    is_root = false;
                    let Some(signer_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.signers_did_lookup.get(&signer_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for flows
                if component.eq_ignore_ascii_case("flow") {
                    is_root = false;
                    let Some(flow_input_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.flow_inputs_did_lookup.get(&flow_input_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for embedded runbooks
                if component.eq_ignore_ascii_case("runbook") {
                    is_root = false;
                    let Some(embedded_runbook_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.embedded_runbooks_did_lookup.get(&embedded_runbook_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }
            }
            let Some(mut current_package) = self.packages.get(source_package_id) else {
                return Ok(None);
            };

            let imported_package = current_package
                .imports_did_lookup
                .get(&component.to_string())
                .and_then(|c| self.constructs.get(c))
                .and_then(|c| Some(&c.package_id))
                .and_then(|p| self.packages.get(&p));

            if let Some(imported_package) = imported_package {
                current_package = imported_package;
                continue;
            }
        }
        Ok(None)
    }

    pub fn expect_construct_id(&self, construct_did: &ConstructDid) -> ConstructId {
        match self.constructs.get(construct_did) {
            Some(id) => id.clone(),
            None => unreachable!(),
        }
    }
}
