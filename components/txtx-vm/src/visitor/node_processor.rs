use crate::{
    types::{
        ConstructData, ImportConstruct, Manual, ModuleConstruct, OutputConstruct, VariableConstruct,
    },
    ExtensionManager,
};

pub fn run_node_processor(
    extension_manager: &mut ExtensionManager,
    manual: &mut Manual,
) -> Result<(), String> {
    // Iterate over explicit modules, add root constructs
    // Constructs paths, ensure uniqueness
    let mut new_constructs = vec![];
    for (_, package) in manual.packages.iter() {
        for construct_uuid in package.variables_uuids.iter() {
            let pre_construct = manual.pre_constructs.get(construct_uuid).unwrap();
            let (_, location) = manual.constructs_locations.get(construct_uuid).unwrap();

            let construct = VariableConstruct::from_block(
                &pre_construct.data.as_variable().unwrap(),
                &location,
            )
            .unwrap();
            new_constructs.push((construct_uuid.clone(), ConstructData::Variable(construct)));
        }

        for construct_uuid in package.outputs_uuids.iter() {
            let pre_construct = manual.pre_constructs.get(construct_uuid).unwrap();
            let (_, location) = manual.constructs_locations.get(construct_uuid).unwrap();

            let construct =
                OutputConstruct::from_block(&pre_construct.data.as_output().unwrap(), &location)
                    .unwrap();
            new_constructs.push((construct_uuid.clone(), ConstructData::Output(construct)));
        }

        for construct_uuid in package.modules_uuids.iter() {
            let pre_construct = manual.pre_constructs.get(construct_uuid).unwrap();
            let (_, location) = manual.constructs_locations.get(construct_uuid).unwrap();

            let construct =
                ModuleConstruct::from_block(&pre_construct.data.as_module().unwrap(), &location)
                    .unwrap();
            new_constructs.push((construct_uuid.clone(), ConstructData::Module(construct)));
        }

        for construct_uuid in package.imports_uuids.iter() {
            let pre_construct = manual.pre_constructs.get(construct_uuid).unwrap();
            let (_, location) = manual.constructs_locations.get(construct_uuid).unwrap();

            let construct =
                ImportConstruct::from_block(&pre_construct.data.as_import().unwrap(), &location)
                    .unwrap();
            new_constructs.push((construct_uuid.clone(), ConstructData::Import(construct)));
        }

        for construct_uuid in package.exts_uuids.iter() {
            let pre_construct = manual.pre_constructs.get(construct_uuid).unwrap();
            let (_, location) = manual.constructs_locations.get(construct_uuid).unwrap();
            let data = pre_construct.data.as_ext().unwrap();
            let Some(construct) = extension_manager
                .from_block(
                    data.extension_name.clone(),
                    data.construct_name.clone(),
                    &data.block,
                    &location,
                )
                .map_err(|e| format!("{:?}", e))?
            else {
                return Err(format!(
                    "Could not get construct {} for extension {}",
                    data.construct_name, data.extension_name
                ));
            };

            new_constructs.push((construct_uuid.clone(), ConstructData::Ext(construct)));
        }
    }

    for (construct_uuid, data) in new_constructs.into_iter() {
        manual.add_construct(&construct_uuid, data);
    }

    Ok(())
}
