//! EVE SSO authentication and server-side sessions, replacing the legacy
//! Socialite + `EsiAuthService` flow. Users have no password; identity per
//! character is tracked via the SSO character owner hash.

pub mod linked;
pub mod session;
pub mod sso;
pub mod tokens;

/// ESI scopes the app requests, verified against the live ESI spec
/// (https://esi.evetech.net/meta/openapi.json). Earlier notes here claimed
/// CCP renamed the structures scope and retired the mail scopes in the
/// March 2026 ESI cleanup; both claims were wrong, and EVE SSO refuses an
/// authorize request carrying an unknown scope, so misremembered
/// identifiers break the whole login. Check the spec, not memory.
pub mod scopes {
    pub const PUBLIC_DATA: &str = "publicData";
    pub const READ_STRUCTURES: &str = "esi-universe.read_structures.v1";
    pub const READ_ASSETS: &str = "esi-assets.read_assets.v1";
    pub const OPEN_WINDOW: &str = "esi-ui.open_window.v1";
    pub const READ_CONTRACTS: &str = "esi-contracts.read_character_contracts.v1";
    pub const READ_CORPORATION_ASSETS: &str = "esi-assets.read_corporation_assets.v1";
    /// The service character's wallet journal, feeding donation
    /// ingestion (the legacy `EsiScope::ReadWallet`; like the mail
    /// scope in `notifications`, the legacy identifier is used as-is).
    pub const READ_WALLET: &str = "esi-wallet.read_character_wallet.v1";

    /// Requested on a normal login, like the legacy `/eve` defaults.
    pub const DEFAULT_LOGIN: [&str; 4] =
        [READ_STRUCTURES, READ_ASSETS, OPEN_WINDOW, READ_CONTRACTS];

    /// The mail scopes the service character needs: ingestion reads and
    /// marks mails (`crate::mails`), the notification outbox sends them
    /// (`crate::notifications`).
    pub const READ_MAIL: &str = "esi-mail.read_mail.v1";
    pub const ORGANIZE_MAIL: &str = "esi-mail.organize_mail.v1";
    pub const SEND_MAIL: &str = "esi-mail.send_mail.v1";

    /// Whether the site ever asks for the scope; `/eve?scopes=` accepts
    /// nothing else.
    pub fn is_known(scope: &str) -> bool {
        ADMIN_LOGIN.contains(&scope)
    }

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

/// What one requestable scope lets the site do, for the settings page's
/// access summary and the missing-scope warnings. A rewrite addition:
/// legacy only ever showed a generic "grant ESI scope" prompt.
pub struct ScopeInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Not part of a normal login; granted through its own flow and
    /// never counted as missing.
    pub optional: bool,
}

/// Every scope a character can hold, in the order the settings page
/// lists them.
pub const SCOPE_CATALOGUE: [ScopeInfo; 5] = [
    ScopeInfo {
        id: scopes::READ_ASSETS,
        label: "Asset import",
        description: "Finds the abyssal modules in your hangars and ships.",
        optional: false,
    },
    ScopeInfo {
        id: scopes::READ_CONTRACTS,
        label: "Personal contracts",
        description: "Shows your own contracts and their history.",
        optional: false,
    },
    ScopeInfo {
        id: scopes::READ_STRUCTURES,
        label: "Structure names",
        description: "Names the citadels your assets and contracts sit in.",
        optional: false,
    },
    ScopeInfo {
        id: scopes::OPEN_WINDOW,
        label: "Open in-game windows",
        description: "Opens a contract in the EVE client from the site.",
        optional: false,
    },
    ScopeInfo {
        id: scopes::READ_CORPORATION_ASSETS,
        label: "Corporation assets",
        description: "Includes your corporation's hangars in asset imports.",
        optional: true,
    },
];
