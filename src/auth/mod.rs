//! EVE SSO authentication and server-side sessions, replacing the legacy
//! Socialite + `EsiAuthService` flow. Users have no password; identity per
//! character is tracked via the SSO character owner hash.

pub mod linked;
pub mod session;
pub mod sso;
pub mod tokens;

/// ESI scopes the app requests. Values are CCP's current scope identifiers
/// (CCP retired several legacy scopes in the March 2026 ESI cleanup — the
/// legacy app's mail scopes no longer exist). Structure reads still use
/// `esi-universe.read_structures.v1`, verified against the live ESI spec;
/// an earlier note here claiming a rename to
/// `esi-structures.read_character.v1` was wrong, and EVE SSO refuses
/// authorize requests carrying that identifier.
pub mod scopes {
    pub const PUBLIC_DATA: &str = "publicData";
    pub const READ_STRUCTURES: &str = "esi-universe.read_structures.v1";
    pub const READ_ASSETS: &str = "esi-assets.read_assets.v1";
    pub const OPEN_WINDOW: &str = "esi-ui.open_window.v1";
    pub const READ_CONTRACTS: &str = "esi-contracts.read_character_contracts.v1";
    pub const READ_CORPORATION_ASSETS: &str = "esi-assets.read_corporation_assets.v1";
    pub const READ_CORPORATION_CONTRACTS: &str = "esi-contracts.read_corporation_contracts.v1";
    /// The service character's wallet journal, feeding donation
    /// ingestion (the legacy `EsiScope::ReadWallet`; like the mail
    /// scope in `notifications`, the legacy identifier is used as-is).
    pub const READ_WALLET: &str = "esi-wallet.read_character_wallet.v1";

    /// Requested on a normal login, like the legacy `/eve` defaults.
    pub const DEFAULT_LOGIN: [&str; 4] = [READ_STRUCTURES, READ_ASSETS, OPEN_WINDOW, READ_CONTRACTS];

    /// The legacy mail scopes, retired in the ESI cleanup and therefore
    /// kept out of the login scope lists: the ported mail ingestion
    /// (`crate::mails`) runs only while the service character still
    /// holds a token carrying them and reports itself skipped otherwise.
    pub const READ_MAIL: &str = "esi-mail.read_mail.v1";
    pub const ORGANIZE_MAIL: &str = "esi-mail.organize_mail.v1";

    /// Requested on the admin login: the legacy required-scopes config
    /// (the wallet scope included, so the service character's token can
    /// read the donations wallet) minus the retired mail scopes (see
    /// `READ_MAIL`, which the ported mail ingestion uses only from a
    /// still-valid legacy token).
    pub const ADMIN_LOGIN: [&str; 8] = [
        PUBLIC_DATA,
        READ_ASSETS,
        OPEN_WINDOW,
        READ_CONTRACTS,
        READ_STRUCTURES,
        READ_CORPORATION_ASSETS,
        READ_CORPORATION_CONTRACTS,
        READ_WALLET,
    ];
}
