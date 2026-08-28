//! EVE SSO authentication and server-side sessions, replacing the legacy
//! Socialite + `EsiAuthService` flow. Users have no password; identity per
//! character is tracked via the SSO character owner hash.

pub mod linked;
pub mod session;
pub mod sso;
pub mod tokens;

/// ESI scopes the app requests. Values are CCP's current scope identifiers
/// (CCP retired and renamed several legacy scopes in the March 2026 ESI
/// cleanup — the legacy app's mail and wallet scopes no longer exist, and
/// structure reads moved from `esi-universe.read_structures.v1` to
/// `esi-structures.read_character.v1`).
pub mod scopes {
    pub const PUBLIC_DATA: &str = "publicData";
    pub const READ_STRUCTURES: &str = "esi-structures.read_character.v1";
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

    /// Requested on the admin login: the legacy required-scopes config
    /// (the wallet scope included, so the service character's token can
    /// read the donations wallet) minus the retired mail scope — the
    /// EVE-mail module submission feature needs a new approach when its
    /// milestone comes up.
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
