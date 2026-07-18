//! Client for EVE Online's ESI API. Only the endpoints the app actually
//! uses are implemented; more arrive with their features (SSO, contracts,
//! assets, mails).

use std::fmt;

use serde::Deserialize;

pub const DEFAULT_BASE_URL: &str = "https://esi.evetech.net";

/// A mutated item's rolled dogma data, from
/// `GET /latest/dogma/dynamic/items/{type_id}/{item_id}/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiDynamicItem {
    pub created_by: i64,
    pub mutator_type_id: i64,
    pub source_type_id: i64,
    #[serde(default)]
    pub dogma_attributes: Vec<EsiDogmaAttribute>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct EsiDogmaAttribute {
    pub attribute_id: i64,
    pub value: f64,
}

#[derive(Debug)]
pub enum EsiError {
    /// ESI does not know the item (or it is not a dynamic item).
    NotFound,
    Http(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
}

impl fmt::Display for EsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EsiError::NotFound => write!(f, "not found on ESI"),
            EsiError::Http(error) => write!(f, "ESI request failed: {error}"),
            EsiError::UnexpectedStatus(status) => write!(f, "unexpected ESI status: {status}"),
        }
    }
}

impl std::error::Error for EsiError {}

impl From<reqwest::Error> for EsiError {
    fn from(error: reqwest::Error) -> Self {
        EsiError::Http(error)
    }
}

#[derive(Clone)]
pub struct EsiClient {
    base_url: String,
    http: reqwest::Client,
}

impl EsiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .user_agent("MutaMarket (https://mutamarket.com)")
                .build()
                .expect("reqwest client"),
        }
    }

    /// Base URL from `ESI_BASE_URL`, falling back to the public ESI.
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("ESI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        Self::new(&base_url)
    }

    /// The rolled dogma attributes of a mutated item.
    pub async fn dynamic_item(
        &self,
        type_id: i64,
        item_id: i64,
    ) -> Result<EsiDynamicItem, EsiError> {
        let response = self
            .http
            .get(format!(
                "{}/latest/dogma/dynamic/items/{type_id}/{item_id}/",
                self.base_url,
            ))
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => Err(EsiError::UnexpectedStatus(status)),
        }
    }
}
