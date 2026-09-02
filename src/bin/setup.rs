//! The guided deployment setup: checks the domain, walks through the
//! credentials and the optional integrations, verifies each against its
//! provider, and writes `.env`. `deploy/setup.sh` builds the image, runs
//! this inside it with the checkout mounted, then starts the stack.
//!
//! Usage: `deploy/setup.sh` (or `cargo run --bin setup` with the
//! checkout as the working directory; `SETUP_ENV_PATH` overrides the
//! file written).

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

use mutamarket::auth::scopes;
use mutamarket::setup::{
    CredentialCheck, Section, classify_sso_answer, dns_matches, invite_code, invite_url,
    looks_like_client_id, origin_host, parse_env, random_password, render_env,
};

const ENV_PATH_VAR: &str = "SETUP_ENV_PATH";
const EVE_SSO_TOKEN_URL: &str = "https://login.eveonline.com/v2/oauth/token";
const ESI_CHARACTER_URL: &str = "https://esi.evetech.net/latest/characters";
const DISCORD_INVITE_URL: &str = "https://discord.com/api/v10/invites";
const PATREON_CAMPAIGNS_URL: &str = "https://www.patreon.com/api/oauth2/v2/campaigns?include=tiers&fields%5Btier%5D=title,amount_cents";
/// Where the machine learns its own public addresses.
const PUBLIC_IP_URLS: [&str; 2] = ["https://api.ipify.org", "https://api64.ipify.org"];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

struct Console {
    input: io::StdinLock<'static>,
}

impl Console {
    fn new() -> Self {
        Self {
            input: io::stdin().lock(),
        }
    }

    fn line(&mut self, prompt: &str) -> String {
        print!("{prompt}");
        io::stdout().flush().ok();
        let mut line = String::new();
        self.input.read_line(&mut line).ok();
        line.trim().to_owned()
    }

    /// A value with the previous one as the default (Enter keeps it).
    fn ask(&mut self, label: &str, current: &str) -> String {
        let prompt = if current.is_empty() {
            format!("  {label}: ")
        } else {
            format!("  {label} [{current}]: ")
        };
        let answer = self.line(&prompt);
        if answer.is_empty() {
            current.to_owned()
        } else {
            answer
        }
    }

    /// An optional value: Enter keeps the current one, "-" clears it.
    fn ask_optional(&mut self, label: &str, current: &str) -> String {
        let prompt = if current.is_empty() {
            format!("  {label} (Enter to skip): ")
        } else {
            format!("  {label} [{current}] (\"-\" clears): ")
        };
        match self.line(&prompt).as_str() {
            "" => current.to_owned(),
            "-" => String::new(),
            answer => answer.to_owned(),
        }
    }

    fn confirm(&mut self, question: &str, default_yes: bool) -> bool {
        let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
        match self
            .line(&format!("  {question} {hint} "))
            .to_lowercase()
            .as_str()
        {
            "" => default_yes,
            "y" | "yes" => true,
            _ => false,
        }
    }
}

fn heading(title: &str) {
    println!("\n== {title}");
}

fn ok(message: &str) {
    println!("  ok: {message}");
}

fn warn(message: &str) {
    println!("  !! {message}");
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("mutamarket-setup")
        .build()
        .expect("http client")
}

async fn public_addresses(http: &reqwest::Client) -> Vec<IpAddr> {
    let mut addresses = Vec::new();
    for url in PUBLIC_IP_URLS {
        if let Ok(response) = http.get(url).send().await
            && let Ok(text) = response.text().await
            && let Ok(address) = text.trim().parse::<IpAddr>()
        {
            addresses.push(address);
        }
    }
    addresses
}

fn resolve(host: &str) -> Vec<IpAddr> {
    (host, 443)
        .to_socket_addrs()
        .map(|addresses| addresses.map(|a| a.ip()).collect())
        .unwrap_or_default()
}

async fn check_eve_credentials(http: &reqwest::Client, id: &str, secret: &str) -> CredentialCheck {
    let response = http
        .post(EVE_SSO_TOKEN_URL)
        .basic_auth(id, Some(secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", "setup-check"),
        ])
        .send()
        .await;
    match response {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            classify_sso_answer(status, &body)
        }
        Err(_) => CredentialCheck::NotRejected,
    }
}

async fn character_name(http: &reqwest::Client, id: &str) -> Option<String> {
    let id: i64 = id.parse().ok()?;
    let json: serde_json::Value = http
        .get(format!("{ESI_CHARACTER_URL}/{id}/"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    json["name"].as_str().map(str::to_owned)
}

async fn discord_invite_guild(http: &reqwest::Client, code: &str) -> Option<String> {
    let json: serde_json::Value = http
        .get(format!("{DISCORD_INVITE_URL}/{code}"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    json["guild"]["name"].as_str().map(str::to_owned)
}

async fn discord_webhook_name(http: &reqwest::Client, url: &str) -> Option<String> {
    let json: serde_json::Value = http
        .get(url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    json["name"].as_str().map(str::to_owned)
}

/// The creator's campaign tiers: (id, title, monthly cents).
async fn patreon_tiers(
    http: &reqwest::Client,
    token: &str,
) -> Result<Vec<(String, String, i64)>, String> {
    let json: serde_json::Value = http
        .get(PATREON_CAMPAIGNS_URL)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(json["included"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| item["type"] == "tier")
                .map(|item| {
                    (
                        item["id"].as_str().unwrap_or_default().to_owned(),
                        item["attributes"]["title"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        item["attributes"]["amount_cents"]
                            .as_i64()
                            .unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default())
}

fn get<'a>(env: &'a BTreeMap<String, String>, key: &str) -> &'a str {
    env.get(key).map_or("", String::as_str)
}

#[tokio::main]
async fn main() {
    let env_path = std::env::var(ENV_PATH_VAR).unwrap_or_else(|_| ".env".to_owned());
    let previous = std::fs::read_to_string(&env_path)
        .map(|contents| parse_env(&contents))
        .unwrap_or_default();
    let mut console = Console::new();
    let http = client();

    println!("MutaMarket setup");
    println!("Answers are written to {env_path}; Enter keeps the value shown in brackets.");
    if !previous.is_empty() {
        println!(
            "Found an existing file with {} values; they are the defaults.",
            previous.len()
        );
    }

    // 1. The public origin and its DNS.
    heading("Domain");
    println!("  The public origin the site is served on. Caddy requests the certificate");
    println!("  for it, so its DNS record must already point at this machine.");
    let origin = loop {
        let origin = console.ask("Public origin", get(&previous, "STACK_ORIGIN"));
        match origin_host(&origin) {
            Ok(host) => {
                if origin.starts_with("http://")
                    && !console.confirm(
                        "Plain http: logins only work over https. Continue anyway?",
                        false,
                    )
                {
                    continue;
                }
                let resolved = resolve(&host);
                let public = public_addresses(&http).await;
                if resolved.is_empty() {
                    warn(&format!("{host} does not resolve yet"));
                } else if public.is_empty() {
                    warn("could not learn this machine's public address; DNS was not compared");
                    ok(&format!("{host} resolves to {}", join_ips(&resolved)));
                } else if dns_matches(&resolved, &public) {
                    ok(&format!(
                        "{host} points at this machine ({})",
                        join_ips(&resolved)
                    ));
                } else {
                    warn(&format!(
                        "{host} resolves to {} but this machine is {}",
                        join_ips(&resolved),
                        join_ips(&public)
                    ));
                }
                if resolved.is_empty() || (!public.is_empty() && !dns_matches(&resolved, &public)) {
                    if console.confirm("Continue with this origin anyway?", false) {
                        break origin;
                    }
                    continue;
                }
                break origin;
            }
            Err(message) => warn(&message),
        }
    };

    // 2. Database.
    heading("Database");
    let postgres_password = {
        let current = get(&previous, "POSTGRES_PASSWORD");
        if current.is_empty() {
            ok("generated a password for the stack's Postgres");
            random_password()
        } else {
            ok("keeping the existing Postgres password");
            current.to_owned()
        }
    };

    // 3. EVE SSO application.
    heading("EVE Online application");
    println!("  Create one at https://developers.eveonline.com/applications (login with");
    println!("  the account that owns the site). Settings:");
    println!("    Connection type: Authentication & API Access");
    println!("    Callback URL:    {origin}/eve/callback");
    println!("    Scopes:");
    for scope in scopes::ADMIN_LOGIN {
        println!("      {scope}");
    }
    let (eve_client_id, eve_client_secret) = loop {
        let id = console.ask("Client ID", get(&previous, "EVE_CLIENT_ID"));
        let secret = console.ask("Secret key", get(&previous, "EVE_CLIENT_SECRET"));
        if id.is_empty() || secret.is_empty() {
            warn("both are required; the site cannot log anyone in without them");
            continue;
        }
        if !looks_like_client_id(&id) {
            warn("a client id is 32 hex characters; check it against the application page");
            continue;
        }
        match check_eve_credentials(&http, &id, &secret).await {
            CredentialCheck::Rejected(message) => {
                warn(&message);
                if console.confirm("Keep them anyway?", false) {
                    break (id, secret);
                }
            }
            CredentialCheck::NotRejected => {
                ok("EVE did not reject them; the first login on the site is the full test");
                break (id, secret);
            }
        }
    };

    // 4. Structure names.
    heading("Structure names (optional)");
    println!("  Player structures are named through the token of one character that");
    println!("  logged in with the structures scope. Enter that character's id, or skip");
    println!("  and pick it later in the admin console.");
    let structures_character = ask_character(
        &mut console,
        &http,
        "Character id",
        get(&previous, "EVE_STRUCTURES_CHARACTER_ID"),
    )
    .await;

    // 5. Notifications.
    heading("EVE mail notifications (optional)");
    println!("  Offers, raffle wins and contract mails go out as EVE mail from one");
    println!("  character that logs in with the mail scopes on the admin page. Without");
    println!("  it, deliveries are only simulated in the log.");
    let notify_sender = ask_character(
        &mut console,
        &http,
        "Sender character id",
        get(&previous, "NOTIFY_SENDER_CHARACTER_ID"),
    )
    .await;
    let notify_delivery = if notify_sender.is_empty() {
        String::new()
    } else {
        "esi".to_owned()
    };

    // 6. Discord alert webhook.
    heading("Discord alerts (optional)");
    println!("  The hourly check posts to this webhook when an admin character lost a scope.");
    let alert_webhook = loop {
        let url = console.ask_optional("Webhook URL", get(&previous, "DISCORD_ALERT_WEBHOOK"));
        if url.is_empty() {
            break url;
        }
        match discord_webhook_name(&http, &url).await {
            Some(name) => {
                ok(&format!("webhook \"{name}\" answers"));
                break url;
            }
            None => {
                warn("Discord does not know this webhook");
                if console.confirm("Keep it anyway?", false) {
                    break url;
                }
            }
        }
    };

    // 7. Community and partner links.
    heading("Community and partner links (optional)");
    println!("  Shown in the sidebar; unset ones stay hidden. Invites can be the link or");
    println!("  the code after discord.gg/ and are checked with Discord.");
    let mut invites = Vec::new();
    for (key, label) in [
        ("DISCORD_INVITE", "MutaMarket Discord invite"),
        ("ABYSSAL_TRADING_INVITE", "Abyssal Trading Discord invite"),
        ("ECTRADE_INVITE", "EC Trade Discord invite"),
    ] {
        let url = loop {
            let input = console.ask_optional(label, get(&previous, key));
            if input.is_empty() {
                break input;
            }
            match discord_invite_guild(&http, &invite_code(&input)).await {
                Some(guild) => {
                    ok(&format!("invite opens \"{guild}\""));
                    break invite_url(&input);
                }
                None => {
                    warn("Discord does not know this invite");
                    if console.confirm("Keep it anyway?", false) {
                        break invite_url(&input);
                    }
                }
            }
        };
        invites.push((key, url));
    }
    let patreon_link =
        console.ask_optional("Patreon page URL", get(&previous, "PUBLIC_PATREON_LINK"));
    let kofi_link = console.ask_optional("Ko-fi page URL", get(&previous, "PUBLIC_KOFI_LINK"));
    println!("  MarkeeDragon: the store URL the synced ads and the sidebar link to (your");
    println!("  affiliate link) and the coupon code shown beside it.");
    let store_url = console.ask_optional("Store URL", get(&previous, "MARKEEDRAGON_STORE_URL"));
    let store_code =
        console.ask_optional("Coupon code", get(&previous, "PUBLIC_MARKEEDRAGON_CODE"));

    // 8. Patreon premium sync.
    heading("Patreon premium sync (optional)");
    println!("  Patrons of chosen tiers get premium automatically. Register a client at");
    println!("  https://www.patreon.com/portal/registration/register-clients and use its");
    println!("  creator's access token.");
    let patreon_token;
    let mut patreon_tier_ids = get(&previous, "PATREON_PREMIUM_TIERS").to_owned();
    loop {
        let token = console.ask_optional(
            "Creator access token",
            get(&previous, "PATREON_ACCESS_TOKEN"),
        );
        if token.is_empty() {
            patreon_token = token;
            patreon_tier_ids = String::new();
            break;
        }
        match patreon_tiers(&http, &token).await {
            Ok(tiers) => {
                patreon_token = token;
                ok(&format!("token accepted, {} tiers found", tiers.len()));
                for (id, title, cents) in &tiers {
                    println!(
                        "    {id}  {title} ({}.{:02} per month)",
                        cents / 100,
                        cents % 100
                    );
                }
                patreon_tier_ids =
                    console.ask("Premium tier ids, comma separated", &patreon_tier_ids);
                break;
            }
            Err(message) => {
                warn(&format!("Patreon rejected the token: {message}"));
                if console.confirm("Keep it anyway?", false) {
                    patreon_token = token;
                    break;
                }
            }
        }
    }

    // 9. Account linking.
    heading("Account linking (optional)");
    println!("  Users can link Twitch, Discord and Patreon accounts in their settings.");
    println!("  Each needs an OAuth app with the callback shown; skip any you do not use.");
    let mut linking = Vec::new();
    for (name, id_key, secret_key, redirect_key, path) in [
        (
            "Twitch",
            "TWITCH_CLIENT_ID",
            "TWITCH_CLIENT_SECRET",
            "TWITCH_REDIRECT_URI",
            "/twitch/callback",
        ),
        (
            "Discord",
            "DISCORD_CLIENT_ID",
            "DISCORD_CLIENT_SECRET",
            "DISCORD_REDIRECT_URI",
            "/discord/callback",
        ),
        (
            "Patreon",
            "PATREON_CLIENT_ID",
            "PATREON_CLIENT_SECRET",
            "PATREON_REDIRECT_URI",
            "/patreon/callback",
        ),
    ] {
        println!("  {name} callback: {origin}{path}");
        let id = console.ask_optional(&format!("{name} client id"), get(&previous, id_key));
        let secret = if id.is_empty() {
            String::new()
        } else {
            console.ask(&format!("{name} client secret"), get(&previous, secret_key))
        };
        let redirect = if id.is_empty() {
            String::new()
        } else {
            format!("{origin}{path}")
        };
        linking.push((id_key, id));
        linking.push((secret_key, secret));
        linking.push((redirect_key, redirect));
    }
    let discord_bot_token = if get(&previous, "DISCORD_CLIENT_ID").is_empty()
        && linking
            .iter()
            .any(|(key, value)| *key == "DISCORD_CLIENT_ID" && value.is_empty())
    {
        String::new()
    } else {
        println!("  The Discord bot token opens the DM channel used for notifications.");
        console.ask_optional("Discord bot token", get(&previous, "DISCORD_BOT_TOKEN"))
    };

    // 10. Premium.
    heading("Premium");
    println!("  The in-game character donations are sent to, and the ISK prices.");
    let premium_character = console.ask("Donation character", {
        let current = get(&previous, "APP_PREMIUM_CHARACTER");
        if current.is_empty() {
            "MutaMate"
        } else {
            current
        }
    });
    let premium_cost = console.ask("Monthly price (ISK)", {
        let current = get(&previous, "APP_PREMIUM_COST");
        if current.is_empty() {
            "100000000"
        } else {
            current
        }
    });
    let premium_yearly = console.ask("Yearly price (ISK)", {
        let current = get(&previous, "APP_PREMIUM_YEARLY_COST");
        if current.is_empty() {
            "1000000000"
        } else {
            current
        }
    });

    let mut sections = vec![
        Section {
            comment: "The public origin and the stack's database.",
            values: vec![
                ("STACK_ORIGIN", origin.clone()),
                ("STACK_HOST", origin_host(&origin).unwrap_or_default()),
                ("POSTGRES_PASSWORD", postgres_password),
                ("SCHEDULER_ENABLED", "true".to_owned()),
            ],
        },
        Section {
            comment: "EVE SSO application (developers.eveonline.com).",
            values: vec![
                ("EVE_CLIENT_ID", eve_client_id),
                ("EVE_CLIENT_SECRET", eve_client_secret),
                ("EVE_STRUCTURES_CHARACTER_ID", structures_character),
            ],
        },
        Section {
            comment: "EVE mail notifications.",
            values: vec![
                ("NOTIFY_DELIVERY", notify_delivery),
                ("NOTIFY_SENDER_CHARACTER_ID", notify_sender),
            ],
        },
        Section {
            comment: "Discord alerts for the admin scope check.",
            values: vec![("DISCORD_ALERT_WEBHOOK", alert_webhook)],
        },
    ];
    let mut links: Vec<(&'static str, String)> = invites;
    links.push(("PUBLIC_PATREON_LINK", patreon_link));
    links.push(("PUBLIC_KOFI_LINK", kofi_link));
    links.push(("MARKEEDRAGON_STORE_URL", store_url.clone()));
    links.push(("PUBLIC_MARKEEDRAGON_URL", store_url));
    links.push(("PUBLIC_MARKEEDRAGON_CODE", store_code));
    sections.push(Section {
        comment: "Community and partner links.",
        values: links,
    });
    sections.push(Section {
        comment: "Patreon premium sync.",
        values: vec![
            ("PATREON_ACCESS_TOKEN", patreon_token),
            ("PATREON_PREMIUM_TIERS", patreon_tier_ids),
        ],
    });
    let mut linking_values = linking;
    linking_values.push(("DISCORD_BOT_TOKEN", discord_bot_token));
    sections.push(Section {
        comment: "Account linking OAuth apps.",
        values: linking_values,
    });
    sections.push(Section {
        comment: "Premium pricing and the donation character.",
        values: vec![
            ("APP_PREMIUM_CHARACTER", premium_character),
            ("APP_PREMIUM_COST", premium_cost),
            ("APP_PREMIUM_YEARLY_COST", premium_yearly),
        ],
    });

    let contents = render_env(&sections, &previous);
    if let Err(error) = std::fs::write(&env_path, contents) {
        eprintln!("could not write {env_path}: {error}");
        std::process::exit(1);
    }
    heading("Done");
    ok(&format!("wrote {env_path}"));
    println!(
        "  Next: docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build"
    );
    println!("  then open {origin}/api/health and log in as the first admin.");
}

fn join_ips(addresses: &[IpAddr]) -> String {
    addresses
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

async fn ask_character(
    console: &mut Console,
    http: &reqwest::Client,
    label: &str,
    current: &str,
) -> String {
    loop {
        let id = console.ask_optional(label, current);
        if id.is_empty() {
            return id;
        }
        match character_name(http, &id).await {
            Some(name) => {
                ok(&format!("that is {name}"));
                return id;
            }
            None => {
                warn("ESI knows no character with that id");
                if console.confirm("Keep it anyway?", false) {
                    return id;
                }
            }
        }
    }
}
