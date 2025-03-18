use crate::publish::CloudServiceIndexedRunbook;

use graphql_client::GraphQLQuery;

type Timestamptz = String;
type Bigint = u64;
type Uuid = String;
type Jsonb = serde_json::Value;
type Citext = String;
type Bytea = Vec<u8>;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/fixtures/schema.graphql",
    query_path = "src/gql/fixtures/queries.graphql",
    response_derives = "Debug",
    normalization = "rust",
    skip_serializing_none
)]
pub struct InsertRunbooksOne;

pub struct InsertRunbookHelper;
impl InsertRunbookHelper {
    pub fn get_public_permissions() -> insert_runbooks_one::PermissionsObjRelInsertInput {
        insert_runbooks_one::PermissionsObjRelInsertInput {
            data: Box::new(insert_runbooks_one::PermissionsInsertInput {
                type_: Some("public".to_string()),
                permission_users: None,
                permission_organizations: None,
                id: None,
                runbook: Box::new(None),
                runbook_by_id: Box::new(None),
                runbook_by_id1: Box::new(None),
                runbooks: None,
                runbooks_by_read_permissions_id: None,
                runbooks_by_update_permissions_id: None,
            }),
            on_conflict: None,
        }
    }
    pub fn get_private_permissions(
        user_id: &str,
    ) -> insert_runbooks_one::PermissionsObjRelInsertInput {
        insert_runbooks_one::PermissionsObjRelInsertInput {
            data: Box::new(insert_runbooks_one::PermissionsInsertInput {
                type_: Some("private".to_string()),
                permission_users: Some(insert_runbooks_one::PermissionUserArrRelInsertInput {
                    data: vec![insert_runbooks_one::PermissionUserInsertInput {
                        user_id: Some(user_id.to_string()),
                        id: None,
                        permission: Box::new(None),
                        permission_id: None,
                        user: None,
                    }],
                    on_conflict: None,
                }),
                permission_organizations: None,
                id: None,
                runbook: Box::new(None),
                runbook_by_id: Box::new(None),
                runbook_by_id1: Box::new(None),
                runbooks: None,
                runbooks_by_read_permissions_id: None,
                runbooks_by_update_permissions_id: None,
            }),
            on_conflict: None,
        }
    }
    pub fn get_org_permissions(org_id: &str) -> insert_runbooks_one::PermissionsObjRelInsertInput {
        insert_runbooks_one::PermissionsObjRelInsertInput {
            data: Box::new(insert_runbooks_one::PermissionsInsertInput {
                type_: Some("org".to_string()),
                permission_users: None,
                permission_organizations: Some(
                    insert_runbooks_one::PermissionOrganizationArrRelInsertInput {
                        data: vec![insert_runbooks_one::PermissionOrganizationInsertInput {
                            id: None,
                            permission: Box::new(None),
                            permission_id: None,
                            organization_id: Some(org_id.to_string()),
                            organization: None,
                        }],
                        on_conflict: None,
                    },
                ),
                id: None,
                runbook: Box::new(None),
                runbook_by_id: Box::new(None),
                runbook_by_id1: Box::new(None),
                runbooks: None,
                runbooks_by_read_permissions_id: None,
                runbooks_by_update_permissions_id: None,
            }),
            on_conflict: None,
        }
    }
    pub fn get_variable(
        read_permissions: insert_runbooks_one::PermissionsObjRelInsertInput,
        update_permissions: insert_runbooks_one::PermissionsObjRelInsertInput,
        delete_permissions: insert_runbooks_one::PermissionsObjRelInsertInput,
        indexed_runbook: CloudServiceIndexedRunbook,
    ) -> insert_runbooks_one::Variables {
        insert_runbooks_one::Variables {
            name: indexed_runbook.name,
            description: indexed_runbook.description,
            raw_runbook: indexed_runbook.raw_runbook,
            documentation: indexed_runbook.documentation,
            read_permissions,
            update_permissions,
            delete_permissions,
        }
    }
}
