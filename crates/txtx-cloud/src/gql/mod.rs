pub mod get_orgs_for_user;
pub mod insert_runbook;

use graphql_client::GraphQLQuery;
use graphql_client::Response;
use txtx_core::kit::reqwest;

use super::auth::AuthConfig;

pub struct GqlClient {
    client: reqwest::Client,
    registry_gql_url: String,
    id_service_url: String,
    auth_config: AuthConfig,
}

impl GqlClient {
    pub fn new(auth_config: &AuthConfig, id_service_url: &str, registry_gql_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            registry_gql_url: registry_gql_url.to_string(),
            auth_config: auth_config.clone(),
            id_service_url: id_service_url.to_string(),
        }
    }

    pub async fn send_request<T>(
        &mut self,
        variables: T::Variables,
    ) -> Result<T::ResponseData, String>
    where
        T: GraphQLQuery,
    {
        let request_body = T::build_query(variables);

        self.auth_config
            .refresh_session_if_needed(&self.id_service_url, &self.auth_config.pat)
            .await?;

        let response = self
            .client
            .post(&self.registry_gql_url)
            .bearer_auth(&self.auth_config.access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send graphql request to cloud service: {}", e))?;

        let response_body: Response<T::ResponseData> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse graphql response: {}", e))?;

        if let Some(error) = response_body.errors {
            match response_body.extensions {
                Some(extensions) => {
                    return Err(format!(
                        "Failed to execute graphql query: {:?}, extensions: {:?}",
                        error, extensions
                    ));
                }
                None => {
                    return Err(format!("Failed to execute graphql query: {:?}", error));
                }
            }
        }
        let response_data: T::ResponseData = response_body.data.expect("missing response data");
        Ok(response_data)
    }
}
