//! Behavior tests for the Twitch / Discord / Patreon account-linking
//! flows, with each provider's OAuth and API endpoints replaced by local
//! mocks on ephemeral ports.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::Query;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use mutamarket::auth::linked::{DiscordClient, LinkedClients, PatreonClient, TwitchClient};
use mutamarket::auth::session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::ReferenceData;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

/// Users created by this suite carry this name so reruns can clean up.
const TEST_USER_NAME: &str = "Linked Accounts Test User";

/// The one authorization code the mock token endpoints accept; any other
/// code gets a 400, like a real provider.
const VALID_CODE: &str = "mock-code";

const TWITCH_USER_ID: &str = "141981764";
const DISCORD_USER_ID: &str = "80351110224678912";
const DISCORD_CHANNEL_ID: &str = "319674150115610528";
const PATREON_USER_ID: &str = "12345678";

/// The `Authorization` header and `recipient_id` of the last mock Discord
/// DM-channel request.
type ChannelCapture = Arc<Mutex<Option<(String, String)>>>;

fn token_response() -> Json<Value> {
    Json(json!({
        "access_token": "mock-access-token",
        "token_type": "bearer",
        "expires_in": 3600,
    }))
}

async fn token_endpoint(Form(form): Form<HashMap<String, String>>) -> axum::response::Response {
    if form.get("grant_type").map(String::as_str) != Some("authorization_code")
        || form.get("code").map(String::as_str) != Some(VALID_CODE)
    {
        return StatusCode::BAD_REQUEST.into_response();
    }

    token_response().into_response()
}

/// Twitch mock: `id.twitch.tv` and `api.twitch.tv` on one server.
async fn start_mock_twitch() -> String {
    let app = Router::new()
        .route("/oauth2/token", post(token_endpoint))
        .route(
            "/helix/users",
            get(|headers: axum::http::HeaderMap| async move {
                let authorized = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    == Some("Bearer mock-access-token")
                    && headers.get("Client-ID").is_some();
                if !authorized {
                    return StatusCode::UNAUTHORIZED.into_response();
                }

                Json(json!({
                    "data": [{
                        "id": TWITCH_USER_ID,
                        "login": "twitchdev",
                        "display_name": "TwitchDev",
                        "profile_image_url": "https://static-cdn.jtvnw.net/jtv_user_pictures/twitchdev.png",
                        "email": "not-real@email.com",
                    }]
                }))
                .into_response()
            }),
        );

    serve(app).await
}

/// Discord mock: OAuth, `users/@me` and the bot DM-channel endpoint.
async fn start_mock_discord(capture: ChannelCapture) -> String {
    let app = Router::new()
        .route("/oauth2/token", post(token_endpoint))
        .route(
            "/users/@me",
            get(|| async {
                Json(json!({
                    "id": DISCORD_USER_ID,
                    "username": "Nelly",
                    "discriminator": "0",
                    "avatar": "8342729096ea3675442027381ff50dfe",
                    "email": "nelly@discord.com",
                }))
            }),
        )
        .route(
            "/users/@me/channels",
            post(move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                let capture = capture.clone();
                async move {
                    let authorization = headers
                        .get(header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_owned();
                    let recipient = body["recipient_id"].as_str().unwrap_or_default().to_owned();
                    *capture.lock().expect("channel capture lock") =
                        Some((authorization, recipient));

                    Json(json!({ "id": DISCORD_CHANNEL_ID }))
                }
            }),
        );

    serve(app).await
}

/// Patreon mock: token and v2 identity endpoints.
async fn start_mock_patreon() -> String {
    let app = Router::new()
        .route("/oauth2/token", post(token_endpoint))
        .route(
            "/api/oauth2/v2/identity",
            get(|Query(query): Query<HashMap<String, String>>| async move {
                // The exact legacy field selection must arrive.
                if query.get("fields[user]").map(String::as_str)
                    != Some("email,full_name,image_url,vanity")
                {
                    return StatusCode::BAD_REQUEST.into_response();
                }

                Json(json!({
                    "data": {
                        "id": PATREON_USER_ID,
                        "type": "user",
                        "attributes": {
                            "email": "patron@example.com",
                            "full_name": "Full Name",
                            "image_url": "https://c8.patreon.com/2/patron.png",
                            "vanity": "corgi",
                        },
                    },
                }))
                .into_response()
            }),
        );

    serve(app).await
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock provider");
    let address = listener.local_addr().expect("mock provider address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock provider");
    });

    format!("http://{address}")
}


/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

async fn test_app(pool: PgPool, linked: LinkedClients) -> Router {
    mutamarket::server::router(
        pool,
        EsiClient::from_env(),
        SsoClient::from_env(),
        linked,
        estimator_stub(),
        Arc::new(ReferenceData::default()),
        None,
    )
}

async fn test_pool() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Isolate from previous runs of this suite — once per binary: the
    // tests run in parallel, so deleting on every setup races the users
    // the sibling tests just created.
    static CLEANED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    CLEANED
        .get_or_init(|| async {
            sqlx::query("delete from users where name = $1")
                .bind(TEST_USER_NAME)
                .execute(&pool)
                .await
                .expect("clean test users");
        })
        .await;

    pool
}

/// A logged-in user, created directly against the real session layer.
async fn logged_in_user(pool: &PgPool) -> (i64, String) {
    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
        .bind(TEST_USER_NAME)
        .fetch_one(pool)
        .await
        .expect("create user");
    let token = session::create_session(pool, user_id, None)
        .await
        .expect("create session");

    (user_id, token)
}

fn cookie_from(response: &axum::response::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|cookie| cookie.starts_with(&format!("{name}=")) && !cookie.contains("Max-Age=0"))
        .and_then(|cookie| cookie.split(';').next())
        .and_then(|pair| pair.split_once('='))
        .map(|(_, value)| value.to_owned())
}

fn location(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

async fn send(app: &Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("infallible")
}

async fn get_path(app: &Router, path: &str) -> axum::response::Response {
    send(
        app,
        Request::builder().uri(path).body(Body::empty()).expect("request"),
    )
    .await
}

/// Starts a link flow and returns the OAuth state issued with it.
async fn begin_flow(app: &Router, path: &str) -> String {
    let response = get_path(app, path).await;
    assert!(response.status().is_redirection());
    cookie_from(&response, "mm_oauth_state").expect("state cookie")
}

/// Sends the provider callback with a valid state, as the given session.
async fn callback(app: &Router, provider: &str, state: &str, session: Option<&str>) -> axum::response::Response {
    let mut cookies = format!("mm_oauth_state={state}");
    if let Some(token) = session {
        cookies.push_str(&format!("; mm_session={token}"));
    }

    send(
        app,
        Request::builder()
            .uri(format!("/{provider}/callback?code={VALID_CODE}&state={state}"))
            .header(header::COOKIE, cookies)
            .body(Body::empty())
            .expect("request"),
    )
    .await
}

fn twitch_clients(auth_base: &str) -> LinkedClients {
    let mut linked = LinkedClients::from_env();
    linked.twitch = TwitchClient::new(
        auth_base,
        auth_base,
        "twitch-client",
        "twitch-secret",
        "http://test/twitch/callback",
    );
    linked
}

#[tokio::test]
async fn twitch_flow_redirects_and_links_the_logged_in_user() {
    let pool = test_pool().await;
    let mock_url = start_mock_twitch().await;
    let app = test_app(pool.clone(), twitch_clients(&mock_url)).await;

    // The login redirect mirrors the legacy Socialite URL exactly,
    // including force_verify=false and PHP's query encoding.
    let login = get_path(&app, "/twitch").await;
    let state = cookie_from(&login, "mm_oauth_state").expect("state cookie");
    assert_eq!(
        location(&login),
        format!(
            "{mock_url}/oauth2/authorize?client_id=twitch-client\
             &redirect_uri=http%3A%2F%2Ftest%2Ftwitch%2Fcallback\
             &scope=user%3Aread%3Aemail&response_type=code&state={state}\
             &force_verify=false"
        ),
    );

    // The ?switch= flow forces re-verification.
    let switch = get_path(&app, "/twitch?switch=1").await;
    assert!(location(&switch).ends_with("&force_verify=true"));

    // The callback links the account and lands on settings.
    let (user_id, session) = logged_in_user(&pool).await;
    let state = begin_flow(&app, "/twitch").await;
    let linked = callback(&app, "twitch", &state, Some(&session)).await;
    assert!(linked.status().is_redirection());
    assert_eq!(location(&linked), "/settings");

    let (twitch_id, name, avatar, email): (Option<i64>, Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "select twitch_id, twitch_name, twitch_avatar, twitch_email from users where id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("user row");
    assert_eq!(twitch_id, Some(141_981_764));
    assert_eq!(name.as_deref(), Some("TwitchDev"));
    assert_eq!(
        avatar.as_deref(),
        Some("https://static-cdn.jtvnw.net/jtv_user_pictures/twitchdev.png"),
    );
    assert_eq!(email.as_deref(), Some("not-real@email.com"));
}

#[tokio::test]
async fn twitch_callback_failures_redirect_to_settings() {
    let pool = test_pool().await;
    let mock_url = start_mock_twitch().await;
    let app = test_app(pool.clone(), twitch_clients(&mock_url)).await;

    // A state mismatch is the Socialite InvalidStateException path.
    let mismatched = send(
        &app,
        Request::builder()
            .uri(format!("/twitch/callback?code={VALID_CODE}&state=wrong"))
            .header(header::COOKIE, "mm_oauth_state=other")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(mismatched.status().is_redirection());
    assert_eq!(location(&mismatched), "/settings");

    // A missing state cookie also fails the state check.
    let no_cookie = get_path(&app, "/twitch/callback?code=mock-code&state=abc").await;
    assert!(no_cookie.status().is_redirection());
    assert_eq!(location(&no_cookie), "/settings");

    // A provider-side failure (bad code) is caught into the same redirect.
    let state = begin_flow(&app, "/twitch").await;
    let provider_error = send(
        &app,
        Request::builder()
            .uri(format!("/twitch/callback?code=bad-code&state={state}"))
            .header(header::COOKIE, format!("mm_oauth_state={state}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(provider_error.status().is_redirection());
    assert_eq!(location(&provider_error), "/settings");

    // A guest completing the flow hits the legacy null-user crash: 500.
    let state = begin_flow(&app, "/twitch").await;
    let guest = callback(&app, "twitch", &state, None).await;
    assert_eq!(guest.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn discord_flow_redirects_and_links_with_the_bot_channel() {
    let pool = test_pool().await;
    let capture: ChannelCapture = Arc::new(Mutex::new(None));
    let mock_url = start_mock_discord(capture.clone()).await;

    let mut linked = LinkedClients::from_env();
    linked.discord = DiscordClient::new(
        &mock_url,
        "discord-client",
        "discord-secret",
        "http://test/discord/callback",
        "test-bot-token",
    );
    let app = test_app(pool.clone(), linked).await;

    // Default login prompts none; the ?switch= flow asks for consent again.
    let login = get_path(&app, "/discord").await;
    let state = cookie_from(&login, "mm_oauth_state").expect("state cookie");
    assert_eq!(
        location(&login),
        format!(
            "{mock_url}/oauth2/authorize?client_id=discord-client\
             &redirect_uri=http%3A%2F%2Ftest%2Fdiscord%2Fcallback\
             &scope=identify+email&response_type=code&state={state}&prompt=none"
        ),
    );
    let switch = get_path(&app, "/discord?switch=true").await;
    let switch_location = location(&switch);
    assert!(!switch_location.contains("prompt="));
    assert!(switch_location.contains("&state="));

    let (user_id, session) = logged_in_user(&pool).await;
    let state = begin_flow(&app, "/discord").await;
    let response = callback(&app, "discord", &state, Some(&session)).await;
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/settings");

    // The DM channel was requested as the bot, for the linked user.
    let (authorization, recipient) = capture
        .lock()
        .expect("channel capture lock")
        .clone()
        .expect("channel request captured");
    assert_eq!(authorization, "Bot test-bot-token");
    assert_eq!(recipient, DISCORD_USER_ID);

    let (discord_id, name, avatar, channel_id): (Option<i64>, Option<String>, Option<String>, Option<i64>) =
        sqlx::query_as(
            "select discord_id, discord_name, discord_avatar, discord_channel_id
             from users where id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("user row");
    assert_eq!(discord_id, Some(80_351_110_224_678_912));
    assert_eq!(name.as_deref(), Some("Nelly"));
    assert_eq!(
        avatar.as_deref(),
        Some(
            "https://cdn.discordapp.com/avatars/80351110224678912/8342729096ea3675442027381ff50dfe.jpg"
        ),
    );
    assert_eq!(channel_id, Some(319_674_150_115_610_528));
}

#[tokio::test]
async fn patreon_flow_redirects_and_links_the_logged_in_user() {
    let pool = test_pool().await;
    let mock_url = start_mock_patreon().await;

    let mut linked = LinkedClients::from_env();
    linked.patreon = PatreonClient::new(
        &mock_url,
        &mock_url,
        "patreon-client",
        "patreon-secret",
        "http://test/patreon/callback",
    );
    let app = test_app(pool.clone(), linked).await;

    let login = get_path(&app, "/patreon").await;
    let state = cookie_from(&login, "mm_oauth_state").expect("state cookie");
    assert_eq!(
        location(&login),
        format!(
            "{mock_url}/oauth2/authorize?client_id=patreon-client\
             &redirect_uri=http%3A%2F%2Ftest%2Fpatreon%2Fcallback\
             &scope=campaigns+identity+identity%5Bemail%5D\
             &response_type=code&state={state}"
        ),
    );

    let (user_id, session) = logged_in_user(&pool).await;
    let state = begin_flow(&app, "/patreon").await;
    let response = callback(&app, "patreon", &state, Some(&session)).await;
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/settings");

    type PatreonRow = (Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>);
    let row: PatreonRow = sqlx::query_as(
            "select patreon_id, patreon_name, patreon_avatar, patreon_email, patreon_nickname
             from users where id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("user row");
    assert_eq!(row.0, Some(12_345_678));
    assert_eq!(row.1.as_deref(), Some("Full Name"));
    assert_eq!(row.2.as_deref(), Some("https://c8.patreon.com/2/patron.png"));
    assert_eq!(row.3.as_deref(), Some("patron@example.com"));
    assert_eq!(row.4.as_deref(), Some("corgi"));
}
