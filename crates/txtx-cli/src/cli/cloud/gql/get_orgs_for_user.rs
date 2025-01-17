use graphql_client::GraphQLQuery;

type Uuid = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/cli/cloud/gql/fixtures/schema.graphql",
    query_path = "src/cli/cloud/gql/fixtures/queries.graphql",
    response_derives = "Debug",
    normalization = "rust",
    skip_serializing_none
)]
pub struct OrgsForUser;

pub struct OrgsForUserHelper;
impl OrgsForUserHelper {
    pub fn get_variable(user_id: &str) -> orgs_for_user::Variables {
        orgs_for_user::Variables { user_id: Some(user_id.to_string()) }
    }
}
