//! Client for EVE's SSO OAuth2 endpoints: building the authorize redirect,
//! exchanging the authorization code, and resolving the character behind an
//! access token via the verify endpoint.

use std::fmt;

use serde::Deserialize;

pub const DEFAULT_SSO_BASE_URL: &str = "https://login.eveonline.com";

/// Where EVE sends the user back to after authorizing, unless configured.
const DEFAULT_CALLBACK_URL: &str = "http://127.0.0.1:3000/eve/callback";

/// Issuers EVE's access tokens are allowed to carry, per the legacy
/// provider's validation.
const ACCEPTED_ISSUERS: [&str; 2] = ["login.eveonline.com", "https://login.eveonline.com"];

/// The prefix of the JWT `sub` claim in front of the character id.
const SUBJECT_CHARACTER_PREFIX: &str = "CHARACTER:EVE:";

#[derive(Debug)]
pub enum SsoError {
    Http(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
    InvalidToken(jsonwebtoken::errors::Error),
    MalformedClaims(&'static str),
}

impl fmt::Display for SsoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsoError::Http(error) => write!(f, "SSO request failed: {error}"),
            SsoError::UnexpectedStatus(status) => write!(f, "unexpected SSO status: {status}"),
            SsoError::InvalidToken(error) => write!(f, "invalid SSO access token: {error}"),
            SsoError::MalformedClaims(what) => write!(f, "malformed SSO token claims: {what}"),
        }
    }
}

impl std::error::Error for SsoError {}

impl From<reqwest::Error> for SsoError {
    fn from(error: reqwest::Error) -> Self {
        SsoError::Http(error)
    }
}

impl From<jsonwebtoken::errors::Error> for SsoError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        SsoError::InvalidToken(error)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SsoTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// The character behind a verified access token.
#[derive(Debug, Clone)]
pub struct VerifiedCharacter {
    pub character_id: i64,
    pub character_name: String,
    pub character_owner_hash: String,
    pub scopes: Vec<String>,
}

/// The claims of an EVE SSO access token.
#[derive(Deserialize)]
struct EveTokenClaims {
    /// `CHARACTER:EVE:{character_id}`.
    sub: String,
    /// Character name.
    name: String,
    /// Character owner hash; changes when the character changes hands.
    owner: String,
    /// Granted scopes: absent, a single string, or a list.
    #[serde(default, deserialize_with = "string_or_list")]
    scp: Vec<String>,
}

fn string_or_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    Ok(match Option::<OneOrMany>::deserialize(deserializer)? {
        None => Vec::new(),
        Some(OneOrMany::One(scope)) => vec![scope],
        Some(OneOrMany::Many(scopes)) => scopes,
    })
}

#[derive(Clone)]
pub struct SsoClient {
    base_url: String,
    client_id: String,
    client_secret: String,
    callback_url: String,
    http: reqwest::Client,
}

impl SsoClient {
    pub fn new(base_url: &str, client_id: &str, client_secret: &str, callback_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            client_id: client_id.to_owned(),
            client_secret: client_secret.to_owned(),
            callback_url: callback_url.to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Configuration from `EVE_SSO_BASE_URL`, `EVE_CLIENT_ID`,
    /// `EVE_CLIENT_SECRET` and `EVE_CALLBACK_URL`.
    pub fn from_env() -> Self {
        let env = |name: &str, default: &str| {
            std::env::var(name).unwrap_or_else(|_| default.to_owned())
        };

        Self::new(
            &env("EVE_SSO_BASE_URL", DEFAULT_SSO_BASE_URL),
            &env("EVE_CLIENT_ID", ""),
            &env("EVE_CLIENT_SECRET", ""),
            &env("EVE_CALLBACK_URL", DEFAULT_CALLBACK_URL),
        )
    }

    pub fn authorize_url(&self, state: &str, scopes: &[&str]) -> String {
        format!(
            "{}/v2/oauth/authorize/?response_type=code&redirect_uri={}&client_id={}&scope={}&state={}",
            self.base_url,
            urlencode(&self.callback_url),
            urlencode(&self.client_id),
            urlencode(&scopes.join(" ")),
            urlencode(state),
        )
    }

    /// Exchanges an authorization code for access and refresh tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<SsoTokens, SsoError> {
        let response = self
            .http
            .post(format!("{}/v2/oauth/token", self.base_url))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("grant_type", "authorization_code"), ("code", code)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SsoError::UnexpectedStatus(response.status()));
        }

        Ok(response.json().await?)
    }

    /// Resolves the character behind an access token by validating the JWT
    /// against EVE's JWKS, like the legacy Socialite provider: signature,
    /// expiry and issuer are enforced.
    pub async fn verify(&self, access_token: &str) -> Result<VerifiedCharacter, SsoError> {
        let response = self
            .http
            .get(format!("{}/oauth/jwks", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SsoError::UnexpectedStatus(response.status()));
        }

        let jwks: jsonwebtoken::jwk::JwkSet = response.json().await?;

        let header = jsonwebtoken::decode_header(access_token)?;
        let key = match &header.kid {
            Some(kid) => jwks.find(kid),
            None => jwks.keys.first(),
        }
        .ok_or(SsoError::MalformedClaims("no matching JWKS key"))?;

        let mut validation = jsonwebtoken::Validation::new(header.alg);
        validation.set_issuer(&ACCEPTED_ISSUERS);
        // The audience is the app's client id; the legacy provider does not
        // validate it either.
        validation.validate_aud = false;

        let decoded = jsonwebtoken::decode::<EveTokenClaims>(
            access_token,
            &jsonwebtoken::DecodingKey::from_jwk(key)?,
            &validation,
        )?;
        let claims = decoded.claims;

        let character_id = claims
            .sub
            .strip_prefix(SUBJECT_CHARACTER_PREFIX)
            .and_then(|id| id.parse().ok())
            .ok_or(SsoError::MalformedClaims("subject is not a character"))?;

        Ok(VerifiedCharacter {
            character_id,
            character_name: claims.name,
            character_owner_hash: claims.owner,
            scopes: claims.scp,
        })
    }
}

/// Percent-encodes everything outside RFC 3986 unreserved characters.
fn urlencode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::SsoClient;

    #[test]
    fn authorize_urls_encode_their_parameters() {
        let client = SsoClient::new(
            "https://login.eveonline.com",
            "client-123",
            "secret",
            "https://mutamarket.com/eve/callback",
        );

        let url = client.authorize_url("state-abc", &["publicData", "esi-assets.read_assets.v1"]);

        assert!(url.starts_with("https://login.eveonline.com/v2/oauth/authorize/?"));
        assert!(url.contains("client_id=client-123"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fmutamarket.com%2Feve%2Fcallback"));
        assert!(url.contains("scope=publicData%20esi-assets.read_assets.v1"));
        assert!(url.contains("state=state-abc"));
    }
}
