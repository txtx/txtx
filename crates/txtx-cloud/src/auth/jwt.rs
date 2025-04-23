use jsonwebtoken::{decode, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Deserializer};
use txtx_addon_kit::reqwest::Client;

const JWT_KEY_ID: &str = "id-svc-key-id";

pub struct JwtManager {
    jwks: JwkSet,
}

impl JwtManager {
    pub fn new(jwks: JwkSet) -> Self {
        Self { jwks }
    }

    pub async fn initialize(id_service_url: &str) -> Result<Self, String> {
        let client = Client::new();
        let url = format!("{}/.well-known/jwks.json", id_service_url);
        let res =
            client.get(&url).send().await.map_err(|e| format!("Unable to retrieve JWKS: {e}"))?;
        let jwks = res.json::<JwkSet>().await.map_err(|e| format!("Unable to parse JWKS: {e}"))?;

        Ok(Self { jwks })
    }

    pub fn decode_jwt(&self, token: &str, do_validate_time: bool) -> Result<Claims, String> {
        let jwk = self.jwks.find(JWT_KEY_ID).ok_or(format!("unable to load JWK"))?;

        let decoding_key =
            DecodingKey::from_jwk(jwk).map_err(|e| format!("unable to load JWK: {}", e))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = do_validate_time;
        validation.validate_nbf = do_validate_time;

        decode::<Claims>(&token, &decoding_key, &validation)
            .map_err(|e| format!("unable to validate JWT: {}", e))
            .map(|token_data| token_data.claims)
    }
}

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub sub: String,
    #[serde(rename = "https://hasura.io/jwt/claims")]
    pub hasura: HasuraClaims,
}

#[derive(Debug, Deserialize)]
pub struct HasuraClaims {
    #[serde(rename = "x-hasura-allowed-roles")]
    pub allowed_roles: Vec<String>,
    #[serde(rename = "x-hasura-default-role")]
    pub default_role: String,
    #[serde(rename = "x-hasura-team-ids", deserialize_with = "deserialize_set_string")]
    pub team_ids: Vec<String>,
    #[serde(rename = "x-hasura-user-id")]
    pub user_id: String,
    #[serde(rename = "x-hasura-user-is-anonymous")]
    pub user_is_anonymous: String,
}

pub fn deserialize_set_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let trimmed = s.trim_matches(|c| c == '{' || c == '}');
    Ok(trimmed
        .split(',')
        .map(|part| {
            let part = part.trim_matches('"'); // remove optional double quotes
            part.to_string()
        })
        .collect())
}
