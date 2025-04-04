pub mod get_orgs_for_user;
pub mod insert_runbook;

use graphql_client::GraphQLQuery;
use graphql_client::Response;
use txtx_core::kit::reqwest;

use crate::get_env_var;

use super::auth::AuthConfig;

pub const NHOST_REGION: &str = "NHOST_REGION";
pub const NHOST_SUBDOMAIN: &str = "NHOST_SUBDOMAIN";

pub struct GqlClient {
    client: reqwest::Client,
    endpoint: String,
    auth_config: AuthConfig,
}

impl GqlClient {
    pub fn new(auth_config: &AuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: format!(
                "https://{}.graphql.{}.nhost.run/v1",
                get_env_var(NHOST_SUBDOMAIN),
                get_env_var(NHOST_REGION)
            ),
            auth_config: auth_config.clone(),
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
        if self.auth_config.is_expired() {
            self.auth_config = self.auth_config.refresh_session().await.map_err(|e| {
                format!("Failed to refresh session: {}. Run `txtx cloud login` to log in again.", e)
            })?;
        }

        let response = self
            .client
            .post(&self.endpoint)
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
