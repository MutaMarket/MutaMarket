//! Serializable offer payloads, shaped after the legacy
//! `OfferResource`/`MessageResource` (characters flattened to id+name;
//! the index carries a module summary instead of the full card).

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OfferParticipant {
    pub id: i64,
    pub name: String,
}

/// One row of the offers index.
#[derive(Debug, Serialize)]
pub struct OfferListView {
    pub id: i64,
    pub sender: OfferParticipant,
    pub receiver: OfferParticipant,
    pub module: OfferModuleSummary,
    pub price: f64,
    pub latest_message: LatestMessageView,
    /// The legacy `is_read`: the latest message is mine or already read.
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct OfferModuleSummary {
    pub id: i64,
    pub type_id: i64,
    pub type_name: String,
}

#[derive(Debug, Serialize)]
pub struct LatestMessageView {
    pub content: String,
    pub sender_id: i64,
    pub created_at: String,
}

/// The thread payload of one offer's page.
#[derive(Debug, Serialize)]
pub struct OfferThreadView {
    pub id: i64,
    pub sender: OfferParticipant,
    pub receiver: OfferParticipant,
    pub price: f64,
    /// The viewer's side of the thread.
    pub own_character_id: i64,
    pub left_by_sender: bool,
    pub left_by_receiver: bool,
    /// The full module card payload.
    pub module: Option<crate::modules::view::ModuleDetail>,
    pub messages: Vec<MessageView>,
}

#[derive(Debug, Serialize)]
pub struct MessageView {
    pub id: i64,
    pub sender: OfferParticipant,
    pub content: String,
    pub created_at: String,
    /// Sent by the viewer's side.
    pub mine: bool,
}
