use crate::{
    types::{
        ConstructData, ImportConstruct, Manual, ModuleConstruct, OutputConstruct, VariableConstruct,
    },
    AddonsContext,
};

pub fn run_constructs_processor(
    manual: &mut Manual,
    _addons_ctx: &mut AddonsContext,
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

            let construct = ImportConstruct::from_block(
                &pre_construct.data.as_import().unwrap(),
                &location,
                &manual.packages_uuid_lookup,
            )
            .unwrap();
            new_constructs.push((construct_uuid.clone(), ConstructData::Import(construct)));
        }
    }

    for (construct_uuid, data) in new_constructs.into_iter() {
        manual.add_construct(&construct_uuid, data);
    }

    Ok(())
}
