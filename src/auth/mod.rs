//! EVE SSO authentication and server-side sessions, replacing the legacy
//! Socialite + `EsiAuthService` flow. Users have no password; identity per
//! character is tracked via the SSO character owner hash.

pub mod session;
pub mod sso;

/// ESI scopes the app requests. Values are CCP's scope identifiers.
pub mod scopes {
    pub const PUBLIC_DATA: &str = "publicData";
    pub const READ_STRUCTURES: &str = "esi-universe.read_structures.v1";
    pub const READ_ASSETS: &str = "esi-assets.read_assets.v1";
    pub const OPEN_WINDOW: &str = "esi-ui.open_window.v1";
    pub const READ_CONTRACTS: &str = "esi-contracts.read_character_contracts.v1";
    pub const READ_CORPORATION_ASSETS: &str = "esi-assets.read_corporation_assets.v1";
    pub const READ_MAIL: &str = "esi-mail.read_mail.v1";
    pub const SEND_MAIL: &str = "esi-mail.send_mail.v1";
    pub const ORGANIZE_MAIL: &str = "esi-mail.organize_mail.v1";
    pub const READ_WALLET: &str = "esi-wallet.read_character_wallet.v1";

    /// Requested on a normal login, like the legacy `/eve` defaults.
    pub const DEFAULT_LOGIN: [&str; 4] = [READ_STRUCTURES, READ_ASSETS, OPEN_WINDOW, READ_CONTRACTS];

    /// Requested on the admin login, from the legacy
    /// `services.eveonline.required_scopes` config.
    pub const ADMIN_LOGIN: [&str; 10] = [
        PUBLIC_DATA,
        READ_ASSETS,
        OPEN_WINDOW,
        READ_CONTRACTS,
        READ_MAIL,
        SEND_MAIL,
        ORGANIZE_MAIL,
        READ_STRUCTURES,
        READ_CORPORATION_ASSETS,
        READ_WALLET,
    ];
}
