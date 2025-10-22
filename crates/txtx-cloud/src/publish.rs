use dialoguer::{theme::ColorfulTheme, Select};
use hiro_system_kit::green;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_core::{
    kit::types::{embedded_runbooks::EmbeddedRunbookInputSpecification, types::Type},
    manifest::WorkspaceManifest,
    runbook::embedded_runbook::publishable::PublishableEmbeddedRunbookSpecification,
    types::Runbook,
};

use crate::{
    gql::{
        get_orgs_for_user::{OrgsForUser, OrgsForUserHelper},
        insert_runbook::{insert_runbooks_one, InsertRunbookHelper, InsertRunbooksOne},
    },
    PublishRunbook, PublishRunbookReadPermissions, PublishRunbookWritePermissions,
};

use super::{auth::AuthConfig, gql::GqlClient};

pub fn load_workspace_manifest_from_manifest_path(
    manifest_path: &str,
) -> Result<WorkspaceManifest, String> {
    let manifest_location = FileLocation::from_path_string(manifest_path)?;
    WorkspaceManifest::from_location(&manifest_location)
}

pub async fn handle_publish_command(
    cmd: &PublishRunbook,
    mut runbook: Runbook,
    id_service_url: &str,
    txtx_console_url: &str,
    registry_gql_url: &str,
) -> Result<(), String> {
    let auth_config = AuthConfig::read_from_system_config()
        .map_err(|e| format!("failed to authenticate user: {e}"))?
        .ok_or(format!(
            "You must be logged in to publish a runbook. Run `txtx cloud login` to log in."
        ))?;

    {
        let run = runbook.flow_contexts.first_mut().expect("no flow contexts found");
        let frontier = HashSet::new();
        let _res = run
            .execution_context
            .simulate_execution(
                &runbook.runtime_context,
                &run.workspace_context,
                &runbook.supervision_context,
                &frontier,
            )
            .await;
    }

    let publishable = PublishableEmbeddedRunbookSpecification::build_from_runbook(&runbook)
        .map_err(|diag| {
            format!("failed to build publishable version of runbook: {}", diag.message)
        })?;

    publish_gql(cmd, publishable, &auth_config, id_service_url, txtx_console_url, registry_gql_url)
        .await?;

    Ok(())
}

async fn publish_gql(
    cmd: &PublishRunbook,
    runbook: PublishableEmbeddedRunbookSpecification,
    auth_config: &AuthConfig,
    id_service_url: &str,
    txtx_console_url: &str,
    registry_gql_url: &str,
) -> Result<(), String> {
    let user_id = auth_config.user.id.clone();
    let mut gql_client = GqlClient::new(auth_config, id_service_url, registry_gql_url);

    let indexed_runbook = CloudServiceIndexedRunbook::new(&runbook)?;

    let user_orgs = match (&cmd.read_permissions, &cmd.update_permissions, &cmd.delete_permissions)
    {
        (Some(PublishRunbookReadPermissions::Org), _, _)
        | (_, Some(PublishRunbookWritePermissions::Org), _)
        | (_, _, Some(PublishRunbookWritePermissions::Org)) => Some(
            gql_client
                .send_request::<OrgsForUser>(OrgsForUserHelper::get_variable(&user_id))
                .await
                .map_err(|e| {
                    format!("failed to determine user's organization membership: {}", e)
                })?,
        ),
        _ => None,
    };

    let selected_org_id = if let Some(user_orgs) = user_orgs {
        let mut org_names = vec![];
        let mut org_ids = vec![];
        for org in user_orgs.organizations.iter() {
            if let Some(name) = &org.name {
                org_names.push(name.clone());
                org_ids.push(org.id.clone());
            }
        }
        let org_name_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Which organization do you want to publish to?")
            .items(&org_names)
            .interact()
            .map_err(|e| format!("failed to select organization: {}", e))?;
        let selected_org_id = org_ids[org_name_idx].clone();
        Some(selected_org_id)
    } else {
        None
    };

    let read_permissions = match cmd.read_permissions {
        Some(PublishRunbookReadPermissions::Private) | None => {
            InsertRunbookHelper::get_private_permissions(&user_id)
        }
        Some(PublishRunbookReadPermissions::Org) => InsertRunbookHelper::get_org_permissions(
            &selected_org_id.clone().expect("missing required org data"),
        ),
        Some(PublishRunbookReadPermissions::Public) => {
            InsertRunbookHelper::get_public_permissions()
        }
    };

    let update_permissions = match cmd.update_permissions {
        Some(PublishRunbookWritePermissions::Private) | None => {
            InsertRunbookHelper::get_private_permissions(&user_id)
        }
        Some(PublishRunbookWritePermissions::Org) => InsertRunbookHelper::get_org_permissions(
            &selected_org_id.clone().expect("missing required org data"),
        ),
    };

    let delete_permissions = match cmd.delete_permissions {
        Some(PublishRunbookWritePermissions::Private) | None => {
            InsertRunbookHelper::get_private_permissions(&user_id)
        }
        Some(PublishRunbookWritePermissions::Org) => InsertRunbookHelper::get_org_permissions(
            &selected_org_id.clone().expect("missing required org data"),
        ),
    };

    let response: insert_runbooks_one::ResponseData = gql_client
        .send_request::<InsertRunbooksOne>(InsertRunbookHelper::get_variable(
            read_permissions,
            update_permissions,
            delete_permissions,
            indexed_runbook,
        ))
        .await
        .map_err(|e| format!("failed to publish runbook: {}", e))?;

    println!(
        "{} Runbook published to {}/runbook/{}",
        green!("✓"),
        txtx_console_url,
        response.insert_runbooks_one.unwrap().id
    );

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceSignerDocumentation {
    pub name: String,
    pub description: Option<String>, // todo: maybe make required?
    pub namespace: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceInputDocumentation {
    pub name: String,
    pub description: Option<String>, // todo: maybe make required?
    pub optional: bool,
    pub value_type: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceOutputDocumentation {
    pub name: String,
    pub description: Option<String>, // todo: maybe make required?
    pub value_type: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceRunbookDocumentation {
    pub signers: Vec<CloudServiceSignerDocumentation>,
    pub inputs: Vec<CloudServiceInputDocumentation>,
    pub outputs: Vec<CloudServiceOutputDocumentation>,
}

impl CloudServiceRunbookDocumentation {
    pub fn new(runbook: &PublishableEmbeddedRunbookSpecification) -> Self {
        let mut signers = vec![];
        let mut inputs = vec![];
        for input in runbook.inputs.iter() {
            match input {
                EmbeddedRunbookInputSpecification::Value(value) => {
                    inputs.push(CloudServiceInputDocumentation {
                        name: value.name.clone(),
                        description: Some(value.documentation.clone()),
                        optional: false, // todo: need to find out where this comes from
                        value_type: value.typing.clone(),
                    });
                }
                EmbeddedRunbookInputSpecification::Signer(signer) => {
                    signers.push(CloudServiceSignerDocumentation {
                        name: signer.name.clone(),
                        description: Some(signer.documentation.clone()),
                        namespace: signer.namespace.to_string(),
                    });
                }
            }
        }
        Self {
            signers,
            inputs,
            outputs: vec![], // todo
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceIndexedRunbook {
    pub name: String,
    pub description: Option<String>,
    pub workspace_id: Option<String>,
    pub documentation: JsonValue,
    pub raw_runbook: JsonValue, // this is a serialized PublishableEmbeddedRunbookSpecification
}

impl CloudServiceIndexedRunbook {
    pub fn new(runbook: &PublishableEmbeddedRunbookSpecification) -> Result<Self, String> {
        Ok(Self {
            name: runbook.runbook_id.name.to_string(),
            description: runbook.description.clone(),
            workspace_id: runbook.runbook_id.workspace.clone(),
            documentation: serde_json::to_value(&CloudServiceRunbookDocumentation::new(&runbook))
                .map_err(|e| {
                format!("failed to serialize runbook documentation: {}", e)
            })?,
            raw_runbook: serde_json::to_value(&runbook)
                .map_err(|e| format!("failed to serialize runbook specification: {}", e))?,
        })
    }
}
