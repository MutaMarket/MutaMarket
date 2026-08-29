//! Client for EVE Online's ESI API. Only the endpoints the app actually
//! uses are implemented; more arrive with their features (SSO, contracts,
//! assets, mails).

pub mod telemetry;

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

/// An owned item from `GET /latest/characters/{character_id}/assets/`
/// (and the corporation equivalent).
#[derive(Debug, Clone, Deserialize)]
pub struct EsiAsset {
    pub item_id: i64,
    pub type_id: i64,
    pub location_id: i64,
    /// `Hangar`, `Cargo`, `HiSlot0`, ... (the legacy `LocationFlag` enum).
    pub location_flag: String,
    /// `station`, `solar_system`, `item` or `other`.
    pub location_type: String,
    pub quantity: i64,
    /// Assembled/unstacked items; containers and rolled modules are ones.
    pub is_singleton: bool,
}

/// From `POST /latest/characters/{character_id}/assets/names/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiAssetName {
    pub item_id: i64,
    pub name: String,
}

/// From `GET /latest/characters/{character_id}/contracts/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiCharacterContract {
    pub contract_id: i64,
    /// `auction`, `item_exchange`, `courier`, ...
    #[serde(rename = "type")]
    pub contract_type: String,
    pub issuer_id: i64,
    pub issuer_corporation_id: i64,
    #[serde(default)]
    pub for_corporation: Option<bool>,
    /// `public`, `personal`, `corporation` or `alliance`.
    pub availability: String,
    /// `outstanding`, `finished`, `deleted`, ... (stored raw, like legacy).
    pub status: String,
    #[serde(default)]
    pub title: Option<String>,
    pub date_issued: String,
    pub date_expired: String,
    #[serde(default)]
    pub date_accepted: Option<String>,
    #[serde(default)]
    pub date_completed: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub buyout: Option<f64>,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub acceptor_id: Option<i64>,
    #[serde(default)]
    pub assignee_id: Option<i64>,
}

/// From `GET /latest/characters/{character_id}/wallet/journal/` — only
/// the fields donation ingestion reads (the legacy `WalletJournalEntry`
/// carries more; serde ignores the rest).
#[derive(Debug, Clone, Deserialize)]
pub struct EsiWalletJournalEntry {
    pub id: i64,
    /// `player_donation`, `market_transaction`, ... (stored raw).
    pub ref_type: String,
    /// Positive for incoming ISK, negative for outgoing.
    #[serde(default)]
    pub amount: Option<f64>,
    pub date: String,
    /// The sender for incoming donations.
    #[serde(default)]
    pub first_party_id: Option<i64>,
    #[serde(default)]
    pub second_party_id: Option<i64>,
}

/// From `POST /latest/universe/names/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiName {
    pub id: i64,
    pub name: String,
    /// `character`, `corporation`, `alliance`, `station`, ...
    pub category: String,
}

/// A recipient of an EVE mail (`recipient_type` is `character`,
/// `corporation`, `alliance` or `mailing_list`).
#[derive(Debug, Clone, Deserialize)]
pub struct EsiMailRecipient {
    pub recipient_id: i64,
    pub recipient_type: String,
}

/// A mail header from `GET /latest/characters/{character_id}/mail/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiMailHeader {
    pub mail_id: i64,
    pub from: i64,
    pub subject: String,
    pub timestamp: String,
    #[serde(default, alias = "read")]
    pub is_read: bool,
    #[serde(default)]
    pub recipients: Vec<EsiMailRecipient>,
}

/// A full mail from `GET /latest/characters/{character_id}/mail/{mail_id}/`
/// (the detail endpoint carries no mail_id of its own).
#[derive(Debug, Clone, Deserialize)]
pub struct EsiMail {
    pub from: i64,
    pub subject: String,
    pub timestamp: String,
    #[serde(default, alias = "is_read")]
    pub read: bool,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub recipients: Vec<EsiMailRecipient>,
}

/// From `GET /latest/alliances/{alliance_id}/`. The optional fields
/// mirror the legacy Alliance DTO's nullable columns.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiAlliance {
    pub name: String,
    #[serde(default)]
    pub ticker: Option<String>,
    pub creator_id: i64,
    #[serde(default)]
    pub date_founded: Option<String>,
    #[serde(default)]
    pub executor_corporation_id: Option<i64>,
    #[serde(default)]
    pub faction_id: Option<i64>,
}

/// From `GET /latest/corporations/{corporation_id}/`. Required and
/// optional fields follow the ESI `CorporationsDetail` schema; only the
/// columns of the legacy corporations table are kept, and `alliance_id`
/// is omitted because the legacy `CreateCorporationAction` never stores
/// it.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiCorporation {
    pub name: String,
    pub ticker: String,
    pub ceo_id: i64,
    pub creator_id: i64,
    pub member_count: i64,
    pub tax_rate: f64,
    #[serde(default)]
    pub date_founded: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub faction_id: Option<i64>,
    #[serde(default)]
    pub home_station_id: Option<i64>,
    #[serde(default)]
    pub shares: Option<i64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub war_eligible: Option<bool>,
}

/// From `GET /latest/universe/stations/{station_id}/` (public, no scope).
#[derive(Debug, Clone, Deserialize)]
pub struct EsiStation {
    pub name: String,
    #[serde(default)]
    pub type_id: Option<i64>,
    pub system_id: i64,
}

/// From `GET /latest/universe/structures/{structure_id}/`.
#[derive(Debug, Clone, Deserialize)]
pub struct EsiStructure {
    pub name: String,
    pub owner_id: i64,
    pub solar_system_id: i64,
    #[serde(default)]
    pub type_id: Option<i64>,
}

/// The `X-Pages` pagination header of ESI list endpoints.
fn page_count(response: &reqwest::Response) -> Option<u32> {
    response
        .headers()
        .get("x-pages")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

/// Logs a non-success ESI response with its URL so failures are
/// diagnosable from the logs alone.
fn note_failure(url: &str, status: reqwest::StatusCode) {
    tracing::warn!(%status, url, "ESI request failed");
}

#[derive(Debug)]
pub enum EsiError {
    /// ESI does not know the item (or it is not a dynamic item).
    NotFound,
    /// 401/403 on an authenticated call: the token is dead or lacks
    /// access. Callers delete the stored token, like the legacy
    /// connector's `handleFailedResponse`.
    Forbidden(reqwest::StatusCode),
    Http(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
}

impl fmt::Display for EsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EsiError::NotFound => write!(f, "not found on ESI"),
            EsiError::Forbidden(status) => write!(f, "ESI denied the token ({status})"),
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
    /// Shared across clones, so every caller lands in one stream.
    telemetry: std::sync::Arc<telemetry::EsiTelemetry>,
}

impl EsiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .user_agent("MutaMarket (https://mutamarket.com)")
                .build()
                .expect("reqwest client"),
            telemetry: std::sync::Arc::default(),
        }
    }

    pub fn telemetry(&self) -> std::sync::Arc<telemetry::EsiTelemetry> {
        self.telemetry.clone()
    }

    /// Sends a built request, recording it under the endpoint group.
    async fn send(
        &self,
        endpoint: &'static str,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let started = std::time::Instant::now();
        let result = request.send().await;
        let status = result
            .as_ref()
            .ok()
            .map(|response| response.status().as_u16());
        self.telemetry.record(endpoint, status, started.elapsed());
        result
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
        let request = self
            .http
            .post(format!("{}/latest/characters/affiliation/", self.base_url))
            .json(character_ids);
        let response = self.send("characters/affiliation", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// Opens the in-game contract window for the token's character, from
    /// `POST /latest/ui/openwindow/contract/?contract_id=` (the legacy
    /// `Esi::openContract` / OpenContractRequest). ESI answers 204 on
    /// success; 401/403 surface as [`EsiError::Forbidden`] so the caller
    /// can drop the token, like the legacy connector.
    pub async fn open_contract_window(
        &self,
        access_token: &str,
        contract_id: i64,
    ) -> Result<(), EsiError> {
        let request = self
            .http
            .post(format!(
                "{}/latest/ui/openwindow/contract/?contract_id={contract_id}",
                self.base_url
            ))
            .bearer_auth(access_token);
        let response = self.send("ui/openwindow/contract", request).await?;

        match response.status() {
            status if status.is_success() => Ok(()),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// Names for a set of ids, `POST /universe/names/`. ESI answers 404 for
    /// the whole batch when any id is unresolvable, which the caller
    /// handles by bisecting (like the legacy name command).
    pub async fn names(&self, ids: &[i64]) -> Result<Vec<EsiName>, EsiError> {
        let request = self
            .http
            .post(format!("{}/latest/universe/names/", self.base_url))
            .json(ids);
        let response = self.send("universe/names", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of a region's public contracts, with the total page count
    /// from the `X-Pages` header. A 204 means no contracts.
    pub async fn public_contracts(
        &self,
        region_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiPublicContract>, u32), EsiError> {
        let request = self.http.get(format!(
            "{}/latest/contracts/public/{region_id}/?page={page}",
            self.base_url,
        ));
        let response = self.send("contracts/public", request).await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok((Vec::new(), page)),
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of a public contract's items. 4xx means the contract
    /// vanished before its items could be fetched.
    pub async fn public_contract_items(
        &self,
        contract_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiContractItem>, u32), EsiError> {
        let request = self.http.get(format!(
            "{}/latest/contracts/public/items/{contract_id}/?page={page}",
            self.base_url,
        ));
        let response = self.send("contracts/public/items", request).await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok((Vec::new(), page)),
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// The items endpoint's error message for a vanished contract, the
    /// legacy `GetContractStatusAction` probe: the 4xx body tells apart
    /// an accepted contract, a hidden one and a deleted one. `None` when
    /// the endpoint did not answer with a client error.
    pub async fn public_contract_items_error(
        &self,
        contract_id: i64,
    ) -> Result<Option<String>, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/contracts/public/items/{contract_id}/?page=1",
            self.base_url,
        ));
        let response = self.send("contracts/public/items", request).await?;

        if !response.status().is_client_error() {
            return Ok(None);
        }

        #[derive(serde::Deserialize)]
        struct EsiErrorBody {
            error: Option<String>,
        }
        let body: EsiErrorBody = response
            .json()
            .await
            .unwrap_or(EsiErrorBody { error: None });
        Ok(body.error)
    }

    /// The bids on a public auction contract.
    pub async fn public_contract_bids(
        &self,
        contract_id: i64,
    ) -> Result<Vec<EsiContractBid>, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/contracts/public/bids/{contract_id}/",
            self.base_url,
        ));
        let response = self.send("contracts/public/bids", request).await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(Vec::new()),
            status if status.is_success() => Ok(response.json().await?),
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// Every alliance id that exists, from `GET /latest/alliances/`.
    pub async fn alliance_ids(&self) -> Result<Vec<i64>, EsiError> {
        let request = self
            .http
            .get(format!("{}/latest/alliances/", self.base_url));
        let response = self.send("alliances", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One alliance's public sheet, from
    /// `GET /latest/alliances/{alliance_id}/`.
    pub async fn alliance(&self, alliance_id: i64) -> Result<EsiAlliance, EsiError> {
        let request = self
            .http
            .get(format!("{}/latest/alliances/{alliance_id}/", self.base_url));
        let response = self.send("alliances/sheet", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One corporation's public sheet, from
    /// `GET /latest/corporations/{corporation_id}/`.
    pub async fn corporation(&self, corporation_id: i64) -> Result<EsiCorporation, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/corporations/{corporation_id}/",
            self.base_url
        ));
        let response = self.send("corporations/sheet", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// A type's daily market history in a region.
    pub async fn market_history(
        &self,
        region_id: i64,
        type_id: i64,
    ) -> Result<Vec<EsiMarketDay>, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/markets/{region_id}/history/?type_id={type_id}",
            self.base_url,
        ));
        let response = self.send("markets/history", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of an authenticated GET list endpoint, with the total page
    /// count from `X-Pages`. 401/403 surface as [`EsiError::Forbidden`] so
    /// the caller can drop the token, like the legacy connector.
    async fn authed_page<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &'static str,
        access_token: &str,
        path: &str,
        page: u32,
    ) -> Result<(Vec<T>, u32), EsiError> {
        let request = self
            .http
            .get(format!("{}{path}?page={page}", self.base_url))
            .bearer_auth(access_token);
        let response = self.send(endpoint, request).await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok((Vec::new(), page)),
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of a character's assets, from
    /// `GET /latest/characters/{character_id}/assets/`.
    pub async fn character_assets(
        &self,
        access_token: &str,
        character_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiAsset>, u32), EsiError> {
        self.authed_page(
            "characters/assets",
            access_token,
            &format!("/latest/characters/{character_id}/assets/"),
            page,
        )
        .await
    }

    /// One page of a corporation's assets, from
    /// `GET /latest/corporations/{corporation_id}/assets/`.
    pub async fn corporation_assets(
        &self,
        access_token: &str,
        corporation_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiAsset>, u32), EsiError> {
        self.authed_page(
            "corporations/assets",
            access_token,
            &format!("/latest/corporations/{corporation_id}/assets/"),
            page,
        )
        .await
    }

    /// Custom names of owned items, from
    /// `POST /latest/characters/{character_id}/assets/names/` (or the
    /// corporation equivalent). The caller chunks the ids to ESI's limit.
    pub async fn asset_names(
        &self,
        access_token: &str,
        path_owner: &str,
        item_ids: &[i64],
    ) -> Result<Vec<EsiAssetName>, EsiError> {
        let request = self
            .http
            .post(format!(
                "{}/latest/{path_owner}/assets/names/",
                self.base_url
            ))
            .bearer_auth(access_token)
            .json(item_ids);
        let response = self.send("assets/names", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// Sends an EVE in-game mail as the character, via
    /// `POST /latest/characters/{character_id}/mail/` (scope
    /// `esi-mail.send_mail.v1`). Returns the new mail's id.
    pub async fn send_mail(
        &self,
        access_token: &str,
        character_id: i64,
        recipient_character_id: i64,
        subject: &str,
        body: &str,
    ) -> Result<i64, EsiError> {
        let request = self
            .http
            .post(format!(
                "{}/latest/characters/{character_id}/mail/",
                self.base_url
            ))
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "approved_cost": 0,
                "body": body,
                "recipients": [{
                    "recipient_id": recipient_character_id,
                    "recipient_type": "character",
                }],
                "subject": subject,
            }));
        let response = self.send("characters/mail", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// The newest mail headers of a character's inbox, from
    /// `GET /latest/characters/{character_id}/mail/` (scope
    /// `esi-mail.read_mail.v1`; ESI returns the latest 50).
    pub async fn mail_headers(
        &self,
        access_token: &str,
        character_id: i64,
    ) -> Result<Vec<EsiMailHeader>, EsiError> {
        let request = self
            .http
            .get(format!(
                "{}/latest/characters/{character_id}/mail/",
                self.base_url
            ))
            .bearer_auth(access_token);
        let response = self.send("characters/mail", request).await?;

        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(Vec::new()),
            status if status.is_success() => Ok(response.json().await?),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One full mail, from
    /// `GET /latest/characters/{character_id}/mail/{mail_id}/`.
    pub async fn mail(
        &self,
        access_token: &str,
        character_id: i64,
        mail_id: i64,
    ) -> Result<EsiMail, EsiError> {
        let request = self
            .http
            .get(format!(
                "{}/latest/characters/{character_id}/mail/{mail_id}/",
                self.base_url
            ))
            .bearer_auth(access_token);
        let response = self.send("characters/mail/sheet", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// Marks a mail read in-game, via
    /// `PUT /latest/characters/{character_id}/mail/{mail_id}/` (scope
    /// `esi-mail.organize_mail.v1`), the legacy `Esi::updateEveMail`.
    pub async fn set_mail_read(
        &self,
        access_token: &str,
        character_id: i64,
        mail_id: i64,
    ) -> Result<(), EsiError> {
        let request = self
            .http
            .put(format!(
                "{}/latest/characters/{character_id}/mail/{mail_id}/",
                self.base_url
            ))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "read": true }));
        let response = self.send("characters/mail/update", request).await?;

        match response.status() {
            status if status.is_success() => Ok(()),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of a character's contracts, from
    /// `GET /latest/characters/{character_id}/contracts/`.
    pub async fn character_contracts(
        &self,
        access_token: &str,
        character_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiCharacterContract>, u32), EsiError> {
        self.authed_page(
            "characters/contracts",
            access_token,
            &format!("/latest/characters/{character_id}/contracts/"),
            page,
        )
        .await
    }

    /// The items of one of a character's contracts, from
    /// `GET /latest/characters/{character_id}/contracts/{contract_id}/items/`
    /// — the authenticated sibling of the public items endpoint.
    pub async fn character_contract_items(
        &self,
        access_token: &str,
        character_id: i64,
        contract_id: i64,
    ) -> Result<Vec<EsiContractItem>, EsiError> {
        let (items, _pages) = self
            .authed_page(
                "characters/contracts/items",
                access_token,
                &format!("/latest/characters/{character_id}/contracts/{contract_id}/items/"),
                1,
            )
            .await?;
        Ok(items)
    }

    /// One page of a character's wallet journal, from
    /// `GET /latest/characters/{character_id}/wallet/journal/` (scope
    /// `esi-wallet.read_character_wallet.v1`).
    pub async fn wallet_journal(
        &self,
        access_token: &str,
        character_id: i64,
        page: u32,
    ) -> Result<(Vec<EsiWalletJournalEntry>, u32), EsiError> {
        self.authed_page(
            "characters/wallet/journal",
            access_token,
            &format!("/latest/characters/{character_id}/wallet/journal/"),
            page,
        )
        .await
    }

    /// Names and categories of ids, from `POST /latest/universe/names/`.
    pub async fn universe_names(&self, ids: &[i64]) -> Result<Vec<EsiName>, EsiError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let request = self
            .http
            .post(format!("{}/latest/universe/names/", self.base_url))
            .json(ids);
        let response = self.send("universe/names", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// One page of the public structure ids, from
    /// `GET /latest/universe/structures/`.
    pub async fn public_structures(&self, page: u32) -> Result<(Vec<i64>, u32), EsiError> {
        let request = self.http.get(format!(
            "{}/latest/universe/structures/?page={page}",
            self.base_url
        ));
        let response = self.send("universe/structures", request).await?;

        match response.status() {
            status if status.is_success() => {
                let pages = page_count(&response).unwrap_or(page);
                Ok((response.json().await?, pages))
            }
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// A structure's public sheet, from
    /// `GET /latest/universe/structures/{structure_id}/`. Needs a token
    /// with the structures scope; 403 means the character has no access.
    /// A public NPC station, `GET /universe/stations/{station_id}/`.
    pub async fn station(&self, station_id: i64) -> Result<EsiStation, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/universe/stations/{station_id}/",
            self.base_url
        ));
        let response = self.send("universe/stations", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status if status.is_client_error() => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    pub async fn structure(
        &self,
        access_token: &str,
        structure_id: i64,
    ) -> Result<EsiStructure, EsiError> {
        let request = self
            .http
            .get(format!(
                "{}/latest/universe/structures/{structure_id}/",
                self.base_url
            ))
            .bearer_auth(access_token);
        let response = self.send("universe/structures/sheet", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            status @ (reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN) => {
                Err(EsiError::Forbidden(status))
            }
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }

    /// The rolled dogma attributes of a mutated item.
    pub async fn dynamic_item(
        &self,
        type_id: i64,
        item_id: i64,
    ) -> Result<EsiDynamicItem, EsiError> {
        let request = self.http.get(format!(
            "{}/latest/dogma/dynamic/items/{type_id}/{item_id}/",
            self.base_url,
        ));
        let response = self.send("dogma/dynamic-items", request).await?;

        match response.status() {
            status if status.is_success() => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(EsiError::NotFound),
            status => {
                note_failure(response.url().as_str(), status);
                Err(EsiError::UnexpectedStatus(status))
            }
        }
    }
}
