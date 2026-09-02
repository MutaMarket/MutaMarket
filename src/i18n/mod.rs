//! Server-side translation of the sentences the API answers with, the
//! legacy `lang/{de,zh}.json` tables keyed by the English text (Laravel's
//! `__()`). The locale follows the legacy `SetLocale` middleware: the
//! `locale` cookie, else the `Accept-Language` preference among the
//! supported locales, else English. It rides on a task-local per request
//! so `tr` needs no request in hand.

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::http::HeaderMap;

/// The legacy cookie name, shared with the frontend switcher.
pub const LOCALE_COOKIE: &str = "locale";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    En,
    De,
    Zh,
    Ru,
}

impl Locale {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "de" => Some(Self::De),
            "zh" => Some(Self::Zh),
            "ru" => Some(Self::Ru),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Zh => "zh",
            Self::Ru => "ru",
        }
    }
}

tokio::task_local! {
    static LOCALE: Locale;
}

fn table(json: &'static str) -> HashMap<String, String> {
    serde_json::from_str(json).expect("legacy translation table parses")
}

static DE: LazyLock<HashMap<String, String>> = LazyLock::new(|| table(include_str!("de.json")));
static ZH: LazyLock<HashMap<String, String>> = LazyLock::new(|| table(include_str!("zh.json")));
static RU: LazyLock<HashMap<String, String>> = LazyLock::new(|| table(include_str!("ru.json")));

/// The current request's locale; English outside a request scope.
pub fn current() -> Locale {
    LOCALE.try_with(|locale| *locale).unwrap_or_default()
}

/// The sentence in the request's locale, unchanged when the table has no
/// entry (like Laravel's `__()`).
pub fn tr(message: &str) -> String {
    let table = match current() {
        Locale::En => return message.to_owned(),
        Locale::De => &*DE,
        Locale::Zh => &*ZH,
        Locale::Ru => &*RU,
    };
    table
        .get(message)
        .map_or_else(|| message.to_owned(), Clone::clone)
}

/// The best supported locale from an `Accept-Language` header, by weight.
pub fn preferred(accept_language: &str) -> Option<Locale> {
    let mut ranked: Vec<(f32, usize, Locale)> = accept_language
        .split(',')
        .enumerate()
        .filter_map(|(index, entry)| {
            let mut parts = entry.trim().split(';');
            let tag = parts.next()?.trim().to_ascii_lowercase();
            let weight = parts
                .find_map(|option| option.trim().strip_prefix("q=").map(str::to_owned))
                .map_or(1.0, |q| q.parse::<f32>().unwrap_or(0.0));
            let language = tag.split('-').next().unwrap_or("");
            let locale = Locale::parse(language)?;
            (weight > 0.0).then_some((weight, index, locale))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
    ranked.first().map(|(_, _, locale)| *locale)
}

/// The legacy `SetLocale` decision for a request.
pub fn from_headers(headers: &HeaderMap) -> Locale {
    crate::auth::session::cookie_value(headers, LOCALE_COOKIE)
        .and_then(|value| Locale::parse(&value))
        .or_else(|| {
            headers
                .get("accept-language")
                .and_then(|value| value.to_str().ok())
                .and_then(preferred)
        })
        .unwrap_or_default()
}

/// Middleware: scopes every handler in the request's locale.
pub async fn layer(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let locale = from_headers(request.headers());
    LOCALE.scope(locale, next.run(request)).await
}

#[cfg(test)]
mod tests {
    use super::{LOCALE, Locale, preferred, tr};

    #[test]
    fn accept_language_picks_the_heaviest_supported_language() {
        assert_eq!(preferred("de-AT,de;q=0.9,en;q=0.8"), Some(Locale::De));
        assert_eq!(preferred("en;q=0.5, zh-CN;q=0.9"), Some(Locale::Zh));
        assert_eq!(preferred("fr-FR,fr;q=0.9"), None);
        assert_eq!(preferred("*"), None);
    }

    #[tokio::test]
    async fn sentences_translate_inside_a_locale_scope_and_pass_through_otherwise() {
        assert_eq!(tr("Failed to add module!"), "Failed to add module!");
        let german = LOCALE
            .scope(Locale::De, async {
                (tr("Failed to add module!"), tr("No such sentence."))
            })
            .await;
        assert_eq!(
            german,
            (
                "Modul konnte nicht hinzugefügt werden!".to_owned(),
                "No such sentence.".to_owned()
            )
        );
        let chinese = LOCALE
            .scope(Locale::Zh, async { tr("Unauthorized!") })
            .await;
        assert_ne!(chinese, "Unauthorized!");
        let russian = LOCALE
            .scope(Locale::Ru, async { tr("Unauthorized!") })
            .await;
        assert!(
            !russian.is_ascii() && russian != chinese,
            "Russian table: {russian}"
        );
    }
}
