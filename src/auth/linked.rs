//! OAuth2 clients for linking Twitch, Discord and Patreon accounts to a
//! user account, ported from the legacy Socialite providers
//! (`SocialiteProviders\{Twitch,Discord,Patreon}`) plus the Discord bot
//! channel lookup (`NotificationChannels\Discord\Discord`).
//!
//! Authorize URLs mirror Socialite exactly: field order (`client_id`,
//! `redirect_uri`, `scope`, `response_type`, `state`, provider extras) and
//! PHP `http_build_query` RFC 1738 encoding, where spaces become `+`.

use std::fmt;

use serde_json::Value;

/// Twitch OAuth host; authorize and token endpoints live under it.
pub const DEFAULT_TWITCH_AUTH_BASE_URL: &str = "https://id.twitch.tv";
/// Twitch Helix API host, used to resolve the linked user.
pub const DEFAULT_TWITCH_API_BASE_URL: &str = "https://api.twitch.tv";
/// Discord API base; OAuth, user and channel endpoints all live under it.
pub const DEFAULT_DISCORD_API_BASE_URL: &str = "https://discord.com/api";
/// Patreon host serving the authorize page (not the API host).
pub const DEFAULT_PATREON_AUTHORIZE_BASE_URL: &str = "https://www.patreon.com";
/// Patreon API host for the token and identity endpoints.
pub const DEFAULT_PATREON_API_BASE_URL: &str = "https://api.patreon.com";

/// Discord CDN host avatar URLs are built on.
const DISCORD_CDN_BASE_URL: &str = "https://cdn.discordapp.com";

/// Scopes of the legacy Twitch Socialite provider.
const TWITCH_SCOPES: [&str; 1] = ["user:read:email"];
/// Scopes of the legacy Discord Socialite provider.
const DISCORD_SCOPES: [&str; 2] = ["identify", "email"];
/// Scopes of the legacy Patreon Socialite provider (`identity[email]` is
/// Patreon's syntax for requesting the email attribute).
const PATREON_SCOPES: [&str; 3] = ["campaigns", "identity", "identity[email]"];

#[derive(Debug)]
pub enum LinkError {
    Http(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
    Malformed(&'static str),
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkError::Http(error) => write!(f, "provider request failed: {error}"),
            LinkError::UnexpectedStatus(status) => {
                write!(f, "unexpected provider status: {status}")
            }
            LinkError::Malformed(what) => write!(f, "malformed provider response: {what}"),
        }
    }
}

impl std::error::Error for LinkError {}

impl From<reqwest::Error> for LinkError {
    fn from(error: reqwest::Error) -> Self {
        LinkError::Http(error)
    }
}

/// The provider-agnostic OAuth2 legs Socialite implements once: the
/// authorize redirect and the code-for-token exchange.
#[derive(Clone)]
struct OAuth2 {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    authorize_url: String,
    token_url: String,
    http: reqwest::Client,
}

impl OAuth2 {
    fn new(
        authorize_url: String,
        token_url: String,
        client_id: &str,
        client_secret: &str,
        redirect_url: &str,
    ) -> Self {
        Self {
            client_id: client_id.to_owned(),
            client_secret: client_secret.to_owned(),
            redirect_url: redirect_url.to_owned(),
            authorize_url,
            token_url,
            http: reqwest::Client::new(),
        }
    }

    /// The authorize URL, mirroring Socialite's `buildAuthUrlFromBase`:
    /// fixed field order with provider extras appended last, and PHP
    /// RFC 1738 value encoding.
    fn authorize_url(&self, scopes: &[&str], state: &str, extra: &[(&str, &str)]) -> String {
        let scope = scopes.join(" ");
        let mut fields: Vec<(&str, &str)> = vec![
            ("client_id", &self.client_id),
            ("redirect_uri", &self.redirect_url),
            ("scope", &scope),
            ("response_type", "code"),
            ("state", state),
        ];
        fields.extend_from_slice(extra);

        let query = fields
            .iter()
            .map(|(name, value)| format!("{name}={}", php_urlencode(value)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{query}", self.authorize_url)
    }

    /// Exchanges the authorization code for an access token, like
    /// Socialite's `getAccessTokenResponse` + the `access_token` pick.
    async fn exchange_code(&self, code: &str) -> Result<String, LinkError> {
        let response = self
            .http
            .post(&self.token_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("code", code),
                ("redirect_uri", &self.redirect_url),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }

        let body: Value = response.json().await?;
        body.get("access_token")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(LinkError::Malformed("token response without access_token"))
    }
}

/// A key the legacy PHP code accessed unconditionally: missing means the
/// mapping throws (caught upstream as a failed link).
fn required_str(value: &Value, key: &'static str) -> Result<String, LinkError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(LinkError::Malformed(key))
}

/// A key the legacy code read leniently (`Arr::get` / `??`).
fn optional_str(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

// ---------------------------------------------------------------- Twitch

#[derive(Debug, Clone)]
pub struct TwitchUser {
    pub id: String,
    /// Legacy maps both `name` and `nickname` to Helix `display_name`.
    pub display_name: String,
    pub avatar: String,
    pub email: Option<String>,
}

#[derive(Clone)]
pub struct TwitchClient {
    oauth: OAuth2,
    api_base: String,
}

impl TwitchClient {
    pub fn new(
        auth_base: &str,
        api_base: &str,
        client_id: &str,
        client_secret: &str,
        redirect_url: &str,
    ) -> Self {
        let auth_base = auth_base.trim_end_matches('/');
        Self {
            oauth: OAuth2::new(
                format!("{auth_base}/oauth2/authorize"),
                format!("{auth_base}/oauth2/token"),
                client_id,
                client_secret,
                redirect_url,
            ),
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    /// The legacy controller always sends `force_verify` as the string
    /// `'true'` or `'false'`, driven by the `?switch=` request boolean.
    pub fn authorize_url(&self, state: &str, force_verify: bool) -> String {
        let force_verify = if force_verify { "true" } else { "false" };
        self.oauth
            .authorize_url(&TWITCH_SCOPES, state, &[("force_verify", force_verify)])
    }

    /// Completes the flow: token exchange, then Helix `users`, mapped like
    /// the legacy provider (`$user['data']['0']`).
    pub async fn user(&self, code: &str) -> Result<TwitchUser, LinkError> {
        let token = self.oauth.exchange_code(code).await?;

        let response = self
            .oauth
            .http
            .get(format!("{}/helix/users", self.api_base))
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(&token)
            .header("Client-ID", &self.oauth.client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }

        let body: Value = response.json().await?;
        let user = body
            .get("data")
            .and_then(|data| data.get(0))
            .ok_or(LinkError::Malformed("helix users response without data"))?;

        Ok(TwitchUser {
            id: required_str(user, "id")?,
            display_name: required_str(user, "display_name")?,
            avatar: required_str(user, "profile_image_url")?,
            email: optional_str(user, "email"),
        })
    }
}

// --------------------------------------------------------------- Discord

#[derive(Debug, Clone)]
pub struct DiscordUser {
    pub id: String,
    /// Legacy stores `discord_name` from the bare `username` (the
    /// discriminator variant only feeds the unused `nickname`).
    pub username: String,
    /// CDN URL formatted like the legacy provider, or none without a hash.
    pub avatar: Option<String>,
}

#[derive(Clone)]
pub struct DiscordClient {
    oauth: OAuth2,
    api_base: String,
    bot_token: String,
}

impl DiscordClient {
    pub fn new(
        api_base: &str,
        client_id: &str,
        client_secret: &str,
        redirect_url: &str,
        bot_token: &str,
    ) -> Self {
        let api_base = api_base.trim_end_matches('/');
        Self {
            oauth: OAuth2::new(
                format!("{api_base}/oauth2/authorize"),
                format!("{api_base}/oauth2/token"),
                client_id,
                client_secret,
                redirect_url,
            ),
            api_base: api_base.to_owned(),
            bot_token: bot_token.to_owned(),
        }
    }

    /// Without consent (the default) the legacy provider appends
    /// `prompt=none`; the `?switch=` flow (`withConsent`) omits it.
    pub fn authorize_url(&self, state: &str, consent: bool) -> String {
        let extra: &[(&str, &str)] = if consent { &[] } else { &[("prompt", "none")] };
        self.oauth.authorize_url(&DISCORD_SCOPES, state, extra)
    }

    /// Completes the flow: token exchange, then `users/@me`.
    pub async fn user(&self, code: &str) -> Result<DiscordUser, LinkError> {
        let token = self.oauth.exchange_code(code).await?;

        let response = self
            .oauth
            .http
            .get(format!("{}/users/@me", self.api_base))
            .bearer_auth(&token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }

        let body: Value = response.json().await?;
        let id = required_str(&body, "id")?;
        let avatar = format_discord_avatar(&id, optional_str(&body, "avatar").as_deref());

        Ok(DiscordUser {
            username: required_str(&body, "username")?,
            id,
            avatar,
        })
    }

    /// The bot's private channel to the user, like
    /// `NotificationChannels\Discord\Discord::getPrivateChannel`:
    /// `POST users/@me/channels` authenticated as the bot.
    pub async fn private_channel_id(&self, user_id: &str) -> Result<String, LinkError> {
        let response = self
            .oauth
            .http
            .post(format!("{}/users/@me/channels", self.api_base))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bot {}", self.bot_token),
            )
            .json(&serde_json::json!({ "recipient_id": user_id }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }

        let body: Value = response.json().await?;
        required_str(&body, "id")
    }

    /// Posts a message to a channel as the bot, like the legacy
    /// `NotificationChannels\Discord\Discord::send`: `content` plus an
    /// optional single embed. Used by the notification delivery job to
    /// reach a user's linked-account DM channel.
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
        embed: Option<serde_json::Value>,
    ) -> Result<(), LinkError> {
        let mut payload = serde_json::json!({ "content": content });
        if let Some(embed) = embed {
            payload["embeds"] = serde_json::json!([embed]);
        }
        let response = self
            .oauth
            .http
            .post(format!("{}/channels/{channel_id}/messages", self.api_base))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bot {}", self.bot_token),
            )
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }
        Ok(())
    }
}

/// The Discord CDN avatar URL, ported with the legacy quirks: PHP
/// `empty()` also discards the hash `"0"`, and the animated check
/// `preg_match('/a_.+/m', $hash)` matches an `a_` (followed by at least
/// one character) anywhere in the hash, not only at the start.
fn format_discord_avatar(user_id: &str, hash: Option<&str>) -> Option<String> {
    let hash = hash.filter(|hash| !hash.is_empty() && *hash != "0")?;

    let is_gif = hash
        .match_indices("a_")
        .any(|(index, _)| index + 2 < hash.len());
    let extension = if is_gif { "gif" } else { "jpg" };

    Some(format!(
        "{DISCORD_CDN_BASE_URL}/avatars/{user_id}/{hash}.{extension}"
    ))
}

// --------------------------------------------------------------- Patreon

#[derive(Debug, Clone)]
pub struct PatreonUser {
    pub id: String,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub email: Option<String>,
    /// `vanity` when the attribute is present (even null), otherwise the
    /// full name, like the legacy `Arr::get` default.
    pub nickname: Option<String>,
}

#[derive(Clone)]
pub struct PatreonClient {
    oauth: OAuth2,
    api_base: String,
}

impl PatreonClient {
    pub fn new(
        authorize_base: &str,
        api_base: &str,
        client_id: &str,
        client_secret: &str,
        redirect_url: &str,
    ) -> Self {
        let api_base = api_base.trim_end_matches('/');
        Self {
            oauth: OAuth2::new(
                format!("{}/oauth2/authorize", authorize_base.trim_end_matches('/')),
                format!("{api_base}/oauth2/token"),
                client_id,
                client_secret,
                redirect_url,
            ),
            api_base: api_base.to_owned(),
        }
    }

    pub fn authorize_url(&self, state: &str) -> String {
        self.oauth.authorize_url(&PATREON_SCOPES, state, &[])
    }

    /// Completes the flow: token exchange, then the v2 identity endpoint
    /// with the exact legacy field selection.
    pub async fn user(&self, code: &str) -> Result<PatreonUser, LinkError> {
        let token = self.oauth.exchange_code(code).await?;

        let response = self
            .oauth
            .http
            .get(format!("{}/api/oauth2/v2/identity", self.api_base))
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(&token)
            .query(&[("fields[user]", "email,full_name,image_url,vanity")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LinkError::UnexpectedStatus(response.status()));
        }

        let body: Value = response.json().await?;
        let data = body
            .get("data")
            .ok_or(LinkError::Malformed("identity response without data"))?;
        let attributes = data
            .get("attributes")
            .ok_or(LinkError::Malformed("identity data without attributes"))?;

        let full_name = optional_str(attributes, "full_name");
        let nickname = match attributes.get("vanity") {
            Some(vanity) => vanity.as_str().map(str::to_owned),
            None => full_name.clone(),
        };

        Ok(PatreonUser {
            id: required_str(data, "id")?,
            avatar: optional_str(attributes, "image_url"),
            email: optional_str(attributes, "email"),
            full_name,
            nickname,
        })
    }
}

// ----------------------------------------------------------------- Suite

/// The three account-linking providers, as held in the app state.
#[derive(Clone)]
pub struct LinkedClients {
    pub twitch: TwitchClient,
    pub discord: DiscordClient,
    pub patreon: PatreonClient,
}

impl LinkedClients {
    /// Configuration from the same env names as the legacy
    /// `config/services.php`: `{PROVIDER}_CLIENT_ID`,
    /// `{PROVIDER}_CLIENT_SECRET`, `{PROVIDER}_REDIRECT_URI` and
    /// `DISCORD_BOT_TOKEN`.
    pub fn from_env() -> Self {
        let env =
            |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_owned());

        Self {
            twitch: TwitchClient::new(
                DEFAULT_TWITCH_AUTH_BASE_URL,
                DEFAULT_TWITCH_API_BASE_URL,
                &env("TWITCH_CLIENT_ID", ""),
                &env("TWITCH_CLIENT_SECRET", ""),
                &env(
                    "TWITCH_REDIRECT_URI",
                    "http://127.0.0.1:3000/twitch/callback",
                ),
            ),
            discord: DiscordClient::new(
                DEFAULT_DISCORD_API_BASE_URL,
                &env("DISCORD_CLIENT_ID", ""),
                &env("DISCORD_CLIENT_SECRET", ""),
                &env(
                    "DISCORD_REDIRECT_URI",
                    "http://127.0.0.1:3000/discord/callback",
                ),
                &env("DISCORD_BOT_TOKEN", ""),
            ),
            patreon: PatreonClient::new(
                DEFAULT_PATREON_AUTHORIZE_BASE_URL,
                DEFAULT_PATREON_API_BASE_URL,
                &env("PATREON_CLIENT_ID", ""),
                &env("PATREON_CLIENT_SECRET", ""),
                &env(
                    "PATREON_REDIRECT_URI",
                    "http://127.0.0.1:3000/patreon/callback",
                ),
            ),
        }
    }
}

/// PHP `urlencode` (the RFC 1738 flavor `http_build_query` uses):
/// alphanumerics and `-_.` pass through, a space becomes `+`, everything
/// else is percent-encoded.
fn php_urlencode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::{DiscordClient, PatreonClient, TwitchClient, format_discord_avatar, php_urlencode};

    #[test]
    fn urlencoding_matches_php_rfc1738() {
        assert_eq!(php_urlencode("identify email"), "identify+email");
        assert_eq!(php_urlencode("user:read:email"), "user%3Aread%3Aemail");
        assert_eq!(php_urlencode("identity[email]"), "identity%5Bemail%5D");
        assert_eq!(
            php_urlencode("http://test/callback"),
            "http%3A%2F%2Ftest%2Fcallback"
        );
        // PHP encodes the tilde under RFC 1738, unlike RFC 3986.
        assert_eq!(php_urlencode("a~b"), "a%7Eb");
    }

    #[test]
    fn twitch_authorize_url_matches_the_legacy_field_order() {
        let client = TwitchClient::new(
            "https://id.twitch.tv",
            "https://api.twitch.tv",
            "client-123",
            "secret",
            "http://test/twitch/callback",
        );

        assert_eq!(
            client.authorize_url("state-abc", false),
            "https://id.twitch.tv/oauth2/authorize?client_id=client-123\
             &redirect_uri=http%3A%2F%2Ftest%2Ftwitch%2Fcallback\
             &scope=user%3Aread%3Aemail&response_type=code&state=state-abc\
             &force_verify=false",
        );
        assert!(
            client
                .authorize_url("s", true)
                .ends_with("&force_verify=true")
        );
    }

    #[test]
    fn discord_authorize_url_prompts_none_unless_consent() {
        let client = DiscordClient::new(
            "https://discord.com/api",
            "client-123",
            "secret",
            "http://test/discord/callback",
            "bot-token",
        );

        assert_eq!(
            client.authorize_url("state-abc", false),
            "https://discord.com/api/oauth2/authorize?client_id=client-123\
             &redirect_uri=http%3A%2F%2Ftest%2Fdiscord%2Fcallback\
             &scope=identify+email&response_type=code&state=state-abc&prompt=none",
        );
        assert!(
            client
                .authorize_url("state-abc", true)
                .ends_with("&state=state-abc")
        );
    }

    #[test]
    fn patreon_authorize_url_encodes_the_identity_email_scope() {
        let client = PatreonClient::new(
            "https://www.patreon.com",
            "https://api.patreon.com",
            "client-123",
            "secret",
            "http://test/patreon/callback",
        );

        assert_eq!(
            client.authorize_url("state-abc"),
            "https://www.patreon.com/oauth2/authorize?client_id=client-123\
             &redirect_uri=http%3A%2F%2Ftest%2Fpatreon%2Fcallback\
             &scope=campaigns+identity+identity%5Bemail%5D\
             &response_type=code&state=state-abc",
        );
    }

    #[test]
    fn discord_avatars_format_with_the_legacy_quirks() {
        // No hash, and PHP-empty hashes, mean no avatar.
        assert_eq!(format_discord_avatar("1", None), None);
        assert_eq!(format_discord_avatar("1", Some("")), None);
        assert_eq!(format_discord_avatar("1", Some("0")), None);

        // Animated hashes get .gif; the legacy regex is unanchored, so an
        // `a_` anywhere in the hash counts.
        assert_eq!(
            format_discord_avatar("42", Some("a_beef")).as_deref(),
            Some("https://cdn.discordapp.com/avatars/42/a_beef.gif"),
        );
        assert_eq!(
            format_discord_avatar("42", Some("beefa_1")).as_deref(),
            Some("https://cdn.discordapp.com/avatars/42/beefa_1.gif"),
        );
        // A trailing `a_` with nothing after it is not animated.
        assert_eq!(
            format_discord_avatar("42", Some("beefa_")).as_deref(),
            Some("https://cdn.discordapp.com/avatars/42/beefa_.jpg"),
        );
        assert_eq!(
            format_discord_avatar("42", Some("beef")).as_deref(),
            Some("https://cdn.discordapp.com/avatars/42/beef.jpg"),
        );
    }
}
