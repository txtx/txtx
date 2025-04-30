use serde::Deserialize;
use serde_json::json;
use txtx_addon_kit::{reqwest, uuid::Uuid};

pub async fn get_user_workspaces(
    access_token: &str,
    service_gql_url: &str,
) -> Result<Vec<Workspace>, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(service_gql_url)
        .bearer_auth(access_token)
        .json(&json!({
            "query": r#"
                query GetWorkspaces {
                    svm_workspaces {
                        name
                        id
                    }
                }
            "#.to_string(),
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("received error from server: {}", err));
    }

    let response: GqlResponse<WorkspaceResponse> =
        res.json().await.map_err(|e| format!("failed to parse response: {}", e))?;

    if let Some(workspace_response) = response.data {
        Ok(workspace_response.svm_workspaces.clone())
    } else {
        Err("No data returned from server".to_string())
    }
}

pub async fn get_workspace_id(
    workspace_name: &str,
    access_token: &str,
    service_gql_url: &str,
) -> Result<Workspace, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(service_gql_url)
        .bearer_auth(access_token)
        .json(&json!({
            "query": r#"
                query GetWorkspaceByName($_eq: String = "") {
                    svm_workspaces(where: {name: {_eq: $_eq}}) {
                        name
                        id
                    }
                }
            "#.to_string(),
            "variables": {
                "_eq": workspace_name
            }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("received error from server: {}", err));
    }

    let response: GqlResponse<WorkspaceResponse> =
        res.json().await.map_err(|e| format!("failed to parse response: {}", e))?;

    if let Some(workspace_response) = response.data {
        if workspace_response.svm_workspaces.is_empty() {
            return Err(format!("workspace '{}' not found", workspace_name));
        }
        Ok(workspace_response.svm_workspaces[0].clone())
    } else {
        Err("No data returned from server".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GqlResponse<T> {
    pub data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceResponse {
    pub svm_workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
}
