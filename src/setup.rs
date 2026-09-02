//! The pure parts of the guided deployment setup (`src/bin/setup.rs`,
//! run by `deploy/setup.sh`): the `.env` file format, the DNS
//! comparison and the credential-check classification. Everything that
//! talks to the network or the terminal stays in the binary.

use std::collections::BTreeMap;
use std::net::IpAddr;

use rand::Rng;

/// One `.env` section: a comment line and its variables in order.
pub struct Section {
    pub comment: &'static str,
    pub values: Vec<(&'static str, String)>,
}

/// `KEY=VALUE` lines of an existing `.env`; comments, blanks and
/// malformed lines are skipped, `export` prefixes and matching quotes
/// are stripped.
pub fn parse_env(contents: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        values.insert(key.trim().to_owned(), value.to_owned());
    }
    values
}

/// The `.env` text: the setup's sections, then every variable of the
/// previous file the setup does not own, so hand-added settings survive
/// a rerun. Empty values are left out entirely: docker compose forwards
/// the file as is, and the integrations treat an unset variable as off.
pub fn render_env(sections: &[Section], previous: &BTreeMap<String, String>) -> String {
    let mut out = String::from(
        "# Written by deploy/setup.sh. Rerun it to change anything; values you\n\
         # add by hand below the managed sections are kept.\n",
    );
    let mut owned = Vec::new();
    for section in sections {
        let set: Vec<&(&str, String)> = section
            .values
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .collect();
        owned.extend(section.values.iter().map(|(key, _)| *key));
        if set.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str("# ");
        out.push_str(section.comment);
        out.push('\n');
        for (key, value) in set {
            out.push_str(&format!("{key}={}\n", quote(value)));
        }
    }
    let extra: Vec<(&String, &String)> = previous
        .iter()
        .filter(|(key, _)| !owned.contains(&key.as_str()))
        .collect();
    if !extra.is_empty() {
        out.push_str("\n# Kept from the previous file.\n");
        for (key, value) in extra {
            out.push_str(&format!("{key}={}\n", quote(value)));
        }
    }
    out
}

fn quote(value: &str) -> String {
    if value
        .chars()
        .any(|c| c.is_whitespace() || c == '#' || c == '"')
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

/// The host of a public origin, which must be an absolute `https://`
/// (or, for a throwaway box, `http://`) URL without a path.
pub fn origin_host(origin: &str) -> Result<String, String> {
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .ok_or_else(|| "the origin must start with https:// (or http://)".to_owned())?;
    if rest.is_empty() || rest.contains('/') || rest.contains(' ') {
        return Err(
            "the origin is scheme and host only, like https://staging.example.com".to_owned(),
        );
    }
    Ok(rest.split(':').next().unwrap_or(rest).to_owned())
}

/// Whether the name points at this machine: any resolved address is one
/// of the machine's public addresses.
pub fn dns_matches(resolved: &[IpAddr], public: &[IpAddr]) -> bool {
    resolved.iter().any(|address| public.contains(address))
}

/// A Discord invite as the app stores it (`https://discord.gg/<code>`)
/// from whatever was typed: the bare code, a discord.gg link or a
/// discord.com/invite link.
pub fn invite_url(input: &str) -> String {
    format!("https://discord.gg/{}", invite_code(input))
}

pub fn invite_code(input: &str) -> String {
    input
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// A 32-character alphanumeric password for the stack's Postgres.
pub fn random_password() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..32)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_files_round_trip_and_keep_unmanaged_values() {
        let previous = parse_env(
            "# comment\nexport EVE_CLIENT_ID=abc\nQUOTED=\"a value\"\nbroken line\nCUSTOM=kept\n",
        );
        assert_eq!(previous["EVE_CLIENT_ID"], "abc");
        assert_eq!(previous["QUOTED"], "a value");
        assert_eq!(previous.len(), 3);

        let sections = [
            Section {
                comment: "EVE SSO app.",
                values: vec![
                    ("EVE_CLIENT_ID", "abc".to_owned()),
                    ("EVE_CLIENT_SECRET", String::new()),
                ],
            },
            Section {
                comment: "Nothing set here.",
                values: vec![("PATREON_ACCESS_TOKEN", String::new())],
            },
        ];
        let rendered = render_env(&sections, &previous);
        assert!(rendered.contains("# EVE SSO app.\nEVE_CLIENT_ID=abc\n"));
        assert!(
            !rendered.contains("EVE_CLIENT_SECRET"),
            "empty values are left out"
        );
        assert!(
            !rendered.contains("Nothing set here"),
            "empty sections are left out"
        );
        assert!(
            rendered.contains("# Kept from the previous file.\nCUSTOM=kept\nQUOTED=\"a value\"\n")
        );
        assert_eq!(parse_env(&rendered)["QUOTED"], "a value");
    }

    #[test]
    fn origins_are_scheme_and_host_only() {
        assert_eq!(
            origin_host("https://staging.mutamarket.com"),
            Ok("staging.mutamarket.com".to_owned())
        );
        assert_eq!(origin_host("http://box:5100"), Ok("box".to_owned()));
        assert!(origin_host("staging.mutamarket.com").is_err());
        assert!(origin_host("https://mutamarket.com/").is_err());
    }

    #[test]
    fn dns_matches_when_any_record_is_this_machine() {
        let v4: IpAddr = "203.0.113.5".parse().unwrap();
        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        let other: IpAddr = "198.51.100.9".parse().unwrap();
        assert!(dns_matches(&[v4, v6], &[v4]));
        assert!(!dns_matches(&[other], &[v4, v6]));
        assert!(!dns_matches(&[], &[v4]));
    }

    #[test]
    fn invites_normalize_to_the_stored_link_form() {
        assert_eq!(invite_url("FuwdBZ5cXq"), "https://discord.gg/FuwdBZ5cXq");
        assert_eq!(
            invite_url("https://discord.gg/FuwdBZ5cXq/"),
            "https://discord.gg/FuwdBZ5cXq"
        );
        assert_eq!(
            invite_code("https://discord.com/invite/FuwdBZ5cXq"),
            "FuwdBZ5cXq"
        );
    }

    #[test]
    fn passwords_are_long_and_random() {
        let one = random_password();
        assert_eq!(one.len(), 32);
        assert!(one.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(one, random_password());
    }
}
