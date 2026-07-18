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

/// From `POST /latest/characters/affiliation/`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct EsiAffiliation {
    pub character_id: i64,
    pub corporation_id: i64,
    #[serde(default)]
    pub alliance_id: Option<i64>,
}

/// From `GET /latest/contracts/public/{region_id}/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiPublicContract {
    pub contract_id: i64,
    /// `auction`, `item_exchange`, `courier`, ...
    #[serde(rename = "type")]
    pub contract_type: String,
    pub issuer_id: i64,
    pub issuer_corporation_id: i64,
    #[serde(default)]
    pub for_corporation: Option<bool>,
    #[serde(default)]
    pub start_location_id: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    pub date_issued: String,
    pub date_expired: String,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub buyout: Option<f64>,
}

/// From `GET /latest/contracts/public/items/{contract_id}/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiContractItem {
    pub record_id: i64,
    pub type_id: i64,
    /// Only singleton items carry an item id; abyssal modules always do.
    #[serde(default)]
    pub item_id: Option<i64>,
    pub quantity: i64,
    /// Included items are offered; non-included ones are asked for.
    pub is_included: bool,
}

/// From `GET /latest/contracts/public/bids/{contract_id}/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiContractBid {
    pub bid_id: i64,
    pub amount: f64,
    pub date_bid: String,
}

/// From `GET /latest/markets/{region_id}/history/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiMarketDay {
    pub date: String,
    pub average: f64,
    pub highest: f64,
    pub lowest: f64,
    pub order_count: i64,
    pub volume: i64,
}

/// The `X-Pages` pagination header of ESI list endpoints.
fn page_count(response: &reqwest::Response) -> Option<u32> {
    response
        .headers()
        .get("x-pages")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
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

    /// Corporation/alliance affiliation of the given characters.
    pub async fn affiliations(
        &self,
        character_ids: &[i64],
    ) -> Result<Vec<EsiAffiliation>, EsiError> {
        let response = self
            .http
            .post(format!("{}/latest/characters/affiliation/", self.base_url))
            .json(character_ids)
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status => Err(EsiError::UnexpectedStatus(status)),
        }
    }

    /// One page of a region's public contracts, with the total page count
    /// from the `X-Pages` header. A 204 means no contracts.
    pub async fn public_contracts(
        &self,
        region_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiPublicContract>, u32), EsiError> {
        let response = self
            .http
            .get(format!(
                "{}/latest/contracts/public/{region_id}/?page={page}",
                self.base_url,
            ))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok((Vec::new(), page)),
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => Err(EsiError::UnexpectedStatus(status)),
        }
    }

    /// One page of a public contract's items. 4xx means the contract
    /// vanished before its items could be fetched.
    pub async fn public_contract_items(
        &self,
        contract_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiContractItem>, u32), EsiError> {
        let response = self
            .http
            .get(format!(
                "{}/latest/contracts/public/items/{contract_id}/?page={page}",
                self.base_url,
            ))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok((Vec::new(), page)),
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => Err(EsiError::UnexpectedStatus(status)),
        }
    }

    /// The bids on a public auction contract.
    pub async fn public_contract_bids(
        &self,
        contract_id: i64,
    ) -> Result<Vec<EsiContractBid>, EsiError> {
        let response = self
            .http
            .get(format!(
                "{}/latest/contracts/public/bids/{contract_id}/",
                self.base_url,
            ))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(Vec::new()),
            status if status.is_success() => Ok(response.json().await?),
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => Err(EsiError::UnexpectedStatus(status)),
        }
    }

    /// A type's daily market history in a region.
    pub async fn market_history(
        &self,
        region_id: i64,
        type_id: i64,
    ) -> Result<Vec<EsiMarketDay>, EsiError> {
        let response = self
            .http
            .get(format!(
                "{}/latest/markets/{region_id}/history/?type_id={type_id}",
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
