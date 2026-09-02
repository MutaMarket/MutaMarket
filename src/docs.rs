//! The documentation pages, ported from the legacy `DocumentationService`:
//! markdown files named `NN-slug.md` with a simple `key: value` front
//! matter block (`section`, defaulting to General), the first H1 as the
//! page title, ordered by the numeric filename prefix, and rendered as
//! GitHub-flavored markdown with heading permalinks and hardened external
//! links.
//!
//! Divergence from legacy: the content lives in `assets/docs` in this
//! repository instead of being fetched from GitHub at runtime, so the site
//! has no GitHub dependency and the pages are reviewed with the code they
//! describe (see [`edit_url`]).

use std::io;
use std::path::Path;
use std::sync::OnceLock;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};

use crate::i18n::Locale;
use crate::view::docs::{DocNavItem, DocNavSection, DocumentationData, DocumentationOutcome};

/// Where the vendored documentation lives, relative to the crate root.
const DOCS_DIR: &str = "assets/docs";

/// The front matter key naming the sidebar section.
const SECTION_KEY: &str = "section";

/// Pages without a section front matter land here, like the legacy
/// default.
const DEFAULT_SECTION: &str = "General";

/// The upstream repository documentation edits go to, like the legacy
/// edit link.
const EDIT_REPO: &str = "MutaMarket/mutamarket";
const EDIT_BRANCH: &str = "main";
/// The pages live in this repository, beside the code they describe.
const EDIT_PATH: &str = "assets/docs";

/// Hosts treated as internal by the external-link hardening.
const INTERNAL_HOSTS: [&str; 2] = ["mutamarket.com", "www.mutamarket.com"];

#[derive(Debug, Clone)]
pub struct DocPage {
    pub slug: String,
    pub order: u32,
    pub section: String,
    pub title: String,
    pub file: String,
    pub html: String,
}

/// The documentation in the request's locale, ordered by filename prefix
/// and loaded once per locale (the content ships with the deploy). An
/// error mirrors the legacy 503 path.
pub fn pages() -> Result<&'static [DocPage], &'static str> {
    pages_for(crate::i18n::current())
}

/// English is the complete set under `assets/docs/en`; another locale's
/// folder overrides page by page, so a missing translation shows the
/// English page (and links to the English file for editing) instead of
/// a hole in the navigation.
pub fn pages_for(locale: Locale) -> Result<&'static [DocPage], &'static str> {
    static EN: OnceLock<Result<Vec<DocPage>, String>> = OnceLock::new();
    static DE: OnceLock<Vec<DocPage>> = OnceLock::new();
    static ZH: OnceLock<Vec<DocPage>> = OnceLock::new();

    let english = EN
        .get_or_init(|| load_pages(Locale::En).map_err(|error| error.to_string()))
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|_| "The documentation is temporarily unavailable.")?;
    let localized = |cell: &'static OnceLock<Vec<DocPage>>| {
        cell.get_or_init(|| merge(english, load_pages(locale).unwrap_or_default()))
            .as_slice()
    };
    Ok(match locale {
        Locale::En => english,
        Locale::De => localized(&DE),
        Locale::Zh => localized(&ZH),
    })
}

/// The English order with each slug's translated page swapped in where
/// one exists.
fn merge(english: &[DocPage], translated: Vec<DocPage>) -> Vec<DocPage> {
    english
        .iter()
        .map(|page| {
            translated
                .iter()
                .find(|candidate| candidate.slug == page.slug)
                .cloned()
                .unwrap_or_else(|| page.clone())
        })
        .collect()
}

pub fn page(slug: &str) -> Result<Option<&'static DocPage>, &'static str> {
    Ok(pages()?.iter().find(|page| page.slug == slug))
}

/// The full page payload for a slug (`None` shows the first page, like the
/// legacy controller default): sidebar sections grouped in first-seen
/// order, the rendered article, and the previous/next neighbours.
pub fn documentation_outcome(page: Option<String>) -> DocumentationOutcome {
    let pages = match pages() {
        Ok(pages) => pages,
        Err(_) => return DocumentationOutcome::Unavailable,
    };

    let slug = page.unwrap_or_else(|| pages[0].slug.clone());

    let Some(index) = pages.iter().position(|entry| entry.slug == slug) else {
        return DocumentationOutcome::NotFound;
    };
    let current = &pages[index];

    // Group by section, preserving first-seen order like the legacy
    // collection groupBy.
    let mut sections: Vec<DocNavSection> = Vec::new();
    for entry in pages {
        let item = DocNavItem {
            slug: entry.slug.clone(),
            title: entry.title.clone(),
        };
        match sections.iter_mut().find(|s| s.title == entry.section) {
            Some(section) => section.pages.push(item),
            None => sections.push(DocNavSection {
                title: entry.section.clone(),
                pages: vec![item],
            }),
        }
    }

    let neighbour = |index: Option<usize>| {
        index
            .and_then(|index| pages.get(index))
            .map(|entry| DocNavItem {
                slug: entry.slug.clone(),
                title: entry.title.clone(),
            })
    };

    DocumentationOutcome::Page(Box::new(DocumentationData {
        sections,
        slug: current.slug.clone(),
        section: current.section.clone(),
        title: current.title.clone(),
        html: current.html.clone(),
        edit_url: edit_url(current),
        previous: neighbour(index.checked_sub(1)),
        next: neighbour(Some(index + 1)),
    }))
}

/// The GitHub edit link of a page, like the legacy editUrl.
pub fn edit_url(page: &DocPage) -> String {
    format!(
        "https://github.com/{EDIT_REPO}/edit/{EDIT_BRANCH}/{EDIT_PATH}/{}",
        page.file,
    )
}

fn load_pages(locale: Locale) -> io::Result<Vec<DocPage>> {
    load_pages_from(&Path::new(DOCS_DIR).join(locale.as_str()), locale)
}

fn load_pages_from(dir: &Path, locale: Locale) -> io::Result<Vec<DocPage>> {
    let mut pages = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file = entry.file_name().to_string_lossy().into_owned();

        let Some((order, slug)) = parse_file_name(&file) else {
            continue;
        };

        let raw = std::fs::read_to_string(entry.path())?;
        let (meta, markdown) = extract_front_matter(&raw);
        let (title, markdown) = extract_title(&markdown, &slug);

        pages.push(DocPage {
            section: meta
                .iter()
                .find(|(key, _)| key == SECTION_KEY)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| DEFAULT_SECTION.to_owned()),
            html: render_markdown(&markdown),
            slug,
            order,
            title,
            file: format!("{}/{file}", locale.as_str()),
        });
    }

    if pages.is_empty() {
        return Err(io::Error::other(format!(
            "no documentation files found in {}",
            dir.display(),
        )));
    }

    pages.sort_by_key(|page| page.order);
    Ok(pages)
}

/// The legacy `NN-slug.md` pattern: numeric order prefix, dash, slug of
/// letters/digits/dashes (lowercased).
fn parse_file_name(name: &str) -> Option<(u32, String)> {
    let stem = name.strip_suffix(".md")?;
    let (order, slug) = stem.split_once('-')?;

    if order.is_empty() || !order.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }

    Some((order.parse().ok()?, slug.to_ascii_lowercase()))
}

/// Parses a leading `---` front matter block of simple `key: value` lines
/// and strips it from the body.
fn extract_front_matter(markdown: &str) -> (Vec<(String, String)>, String) {
    let mut lines = markdown.lines();

    if lines.next().map(str::trim) != Some("---") {
        return (Vec::new(), markdown.to_owned());
    }

    let mut meta = Vec::new();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            let consumed: usize = markdown
                .lines()
                .take(meta.len() + 2)
                .map(|l| l.len() + 1)
                .sum();
            let body = markdown.get(consumed..).unwrap_or_default().to_owned();
            return (
                meta.into_iter()
                    .filter(|(key, value): &(String, String)| !key.is_empty() && !value.is_empty())
                    .collect(),
                body,
            );
        }

        let (key, value) = line.split_once(':').unwrap_or((line, ""));
        meta.push((key.trim().to_lowercase(), value.trim().to_owned()));
    }

    // No closing marker: not front matter after all.
    (Vec::new(), markdown.to_owned())
}

/// Extracts the first H1 as the page title and strips it from the body;
/// pages without one get a headline-cased slug, like the legacy fallback.
fn extract_title(markdown: &str, slug: &str) -> (String, String) {
    for line in markdown.lines() {
        let trimmed = line.trim_end();
        if let Some(title) = trimmed.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                let body = markdown.replacen(trimmed, "", 1);
                return (title.to_owned(), body);
            }
        }
    }

    (headline(slug), markdown.to_owned())
}

/// `Str::headline` for slugs: dashes to spaces, words capitalized.
fn headline(slug: &str) -> String {
    slug.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Renders GitHub-flavored markdown like the legacy converter: raw HTML
/// stripped, unsafe links dropped, external links opened in new windows
/// with noopener/noreferrer, and headings carrying an id plus a `#`
/// permalink anchor.
fn render_markdown(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    let mut output_events: Vec<Event> = Vec::new();
    let mut heading: Option<(pulldown_cmark::HeadingLevel, Vec<Event>)> = None;
    let mut skip_link_end = false;
    // The language of the fence being collected, and its source so far.
    let mut code: Option<(String, String)> = None;

    for event in Parser::new_ext(markdown, options) {
        let event = match event {
            // html_input: strip.
            Event::Html(_) | Event::InlineHtml(_) => continue,
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => match classify_link(&dest_url) {
                LinkKind::Unsafe => {
                    skip_link_end = true;
                    continue;
                }
                LinkKind::External => Event::Html(CowStr::from(format!(
                    r#"<a href="{}" title="{}" target="_blank" rel="noopener noreferrer">"#,
                    escape_attribute(&dest_url),
                    escape_attribute(&title),
                ))),
                LinkKind::Internal => Event::Start(Tag::Link {
                    link_type: pulldown_cmark::LinkType::Inline,
                    dest_url,
                    title,
                    id: CowStr::from(""),
                }),
            },
            Event::End(TagEnd::Link) => {
                if skip_link_end {
                    skip_link_end = false;
                    continue;
                }
                Event::End(TagEnd::Link)
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((level, Vec::new()));
                continue;
            }
            Event::End(TagEnd::Heading(_)) => {
                let Some((level, inner)) = heading.take() else {
                    continue;
                };

                let text: String = inner
                    .iter()
                    .filter_map(|event| match event {
                        Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
                        _ => None,
                    })
                    .collect();
                let id = heading_id(&text);

                let mut fragment = String::new();
                html::push_html(&mut fragment, inner.into_iter());

                output_events.push(Event::Html(CowStr::from(format!(
                    r##"<{level} id="{id}">{fragment}<a href="#{id}" class="docs-anchor" aria-hidden="true">#</a></{level}>"##,
                ))));
                continue;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match &kind {
                    pulldown_cmark::CodeBlockKind::Fenced(language) => language.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                code = Some((language, String::new()));
                continue;
            }
            Event::Text(text) if code.is_some() => {
                if let Some((_, source)) = code.as_mut() {
                    source.push_str(&text);
                }
                continue;
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((language, source)) = code.take() {
                    output_events.push(Event::Html(CowStr::from(highlight(&language, &source))));
                }
                continue;
            }
            other => other,
        };

        match &mut heading {
            Some((_, inner)) => inner.push(event),
            None => output_events.push(event),
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, output_events.into_iter());
    output
}

/// The highlighting theme. Dark, because the site has no light mode.
const CODE_THEME: &str = "base16-ocean.dark";

/// One fenced block as highlighted HTML. An unknown or absent language
/// falls back to plain text rather than guessing wrong.
fn highlight(language: &str, source: &str) -> String {
    use syntect::highlighting::ThemeSet;
    use syntect::html::highlighted_html_for_string;
    use syntect::parsing::SyntaxSet;

    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let themes = THEMES.get_or_init(ThemeSet::load_defaults);

    let syntax = syntaxes
        .find_syntax_by_token(language)
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let theme = &themes.themes[CODE_THEME];

    let language_class = if language.is_empty() {
        String::new()
    } else {
        format!(" data-language=\"{}\"", escape_html(language))
    };

    match highlighted_html_for_string(source, syntaxes, syntax, theme) {
        Ok(html) => format!("<div class=\"docs-code\"{language_class}>{html}</div>"),
        // Highlighting must never lose the code itself.
        Err(_) => format!(
            "<div class=\"docs-code\"{language_class}><pre>{}</pre></div>",
            escape_html(source),
        ),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

enum LinkKind {
    Internal,
    External,
    Unsafe,
}

fn classify_link(dest: &str) -> LinkKind {
    let lower = dest.to_ascii_lowercase();

    if let Some(rest) = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
    {
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if INTERNAL_HOSTS.contains(&host) {
            return LinkKind::Internal;
        }
        return LinkKind::External;
    }

    if lower.starts_with("mailto:") {
        return LinkKind::External;
    }

    // Only relative paths and fragments remain safe; anything with another
    // scheme (javascript:, data:, ...) is dropped like allow_unsafe_links.
    if lower.contains(':') {
        return LinkKind::Unsafe;
    }

    LinkKind::Internal
}

/// The heading permalink id: lowercase, alphanumeric words joined by
/// dashes, like the legacy slug normalizer.
fn heading_id(text: &str) -> String {
    let mut id = String::with_capacity(text.len());

    for c in text.chars() {
        if c.is_alphanumeric() {
            id.extend(c.to_lowercase());
        } else if (c.is_whitespace() || c == '-') && !id.ends_with('-') && !id.is_empty() {
            id.push('-');
        }
    }

    id.trim_end_matches('-').to_owned()
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_parse_like_the_legacy_pattern() {
        assert_eq!(
            parse_file_name("01-getting-started.md"),
            Some((1, "getting-started".to_owned()))
        );
        assert_eq!(
            parse_file_name("15-Legal.md"),
            Some((15, "legal".to_owned()))
        );
        assert_eq!(parse_file_name("readme.md"), None);
        assert_eq!(parse_file_name("2-bad_slug.md"), None);
        assert_eq!(parse_file_name("3-nope.txt"), None);
    }

    #[test]
    fn front_matter_parses_and_strips() {
        let (meta, body) = extract_front_matter("---\nsection: Introduction\n---\n\n# Hi\n");
        assert_eq!(
            meta,
            vec![("section".to_owned(), "Introduction".to_owned())]
        );
        assert_eq!(body, "\n# Hi\n");

        let (meta, body) = extract_front_matter("# No front matter\n");
        assert!(meta.is_empty());
        assert_eq!(body, "# No front matter\n");
    }

    #[test]
    fn titles_extract_with_headline_fallback() {
        let (title, body) = extract_title("\n# Getting Started\n\nText\n", "getting-started");
        assert_eq!(title, "Getting Started");
        assert!(!body.contains("# Getting Started"));

        let (title, _) = extract_title("no heading\n", "rolling-guide");
        assert_eq!(title, "Rolling Guide");
    }

    #[test]
    fn markdown_renders_with_permalinks_and_hardened_links() {
        let html = render_markdown(
            "## What is MutaMarket?\n\n[EVE](https://www.eveonline.com) \
             [home](/modules) [bad](javascript:alert(1)) <script>alert(1)</script>\n",
        );

        assert!(html.contains(r##"<h2 id="what-is-mutamarket">"##));
        assert!(html.contains(
            r##"<a href="#what-is-mutamarket" class="docs-anchor" aria-hidden="true">#</a>"##
        ));
        assert!(html.contains(r#"target="_blank" rel="noopener noreferrer""#));
        assert!(html.contains(r#"<a href="/modules">"#));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn a_locale_overrides_page_by_page_and_falls_back_to_english() {
        let root = std::env::temp_dir().join(format!("docs-locale-{}", std::process::id()));
        for (locale, files) in [
            ("en", vec![("01-first.md", "# First\n\nen"), ("02-second.md", "# Second\n\nen")]),
            ("de", vec![("01-first.md", "# Erste\n\nde")]),
        ] {
            std::fs::create_dir_all(root.join(locale)).expect("dir");
            for (name, body) in files {
                std::fs::write(root.join(locale).join(name), body).expect("file");
            }
        }

        let english = load_pages_from(&root.join("en"), Locale::En).expect("english loads");
        let german = load_pages_from(&root.join("de"), Locale::De).expect("german loads");
        let merged = merge(&english, german);
        let summary: Vec<(&str, &str, &str)> = merged
            .iter()
            .map(|page| (page.slug.as_str(), page.title.as_str(), page.file.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                ("first", "Erste", "de/01-first.md"),
                ("second", "Second", "en/02-second.md"),
            ]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn every_translation_covers_the_whole_english_set() {
        let english = load_pages(Locale::En).expect("docs load");
        for locale in [Locale::De, Locale::Zh] {
            let translated = load_pages(locale).expect("translated docs load");
            let slugs = |pages: &[DocPage]| pages.iter().map(|p| p.slug.clone()).collect::<Vec<_>>();
            assert_eq!(slugs(&translated), slugs(&english), "{locale:?} mirrors the English pages");
        }
    }

    #[test]
    fn the_vendored_content_loads_ordered_with_sections() {
        let pages = load_pages(Locale::En).expect("docs load");

        // The legacy set plus the two API prose pages this rewrite adds.
        assert_eq!(pages.len(), 17, "the vendored docs set");
        assert_eq!(pages[0].slug, "getting-started");
        assert_eq!(pages[0].section, "Introduction");
        assert_eq!(pages[0].title, "Getting started");
        assert!(pages.windows(2).all(|pair| pair[0].order <= pair[1].order));
        assert!(pages.iter().any(|page| page.slug == "about"));
        assert!(
            edit_url(&pages[0]).ends_with("assets/docs/en/01-getting-started.md"),
            "edit link points at the pages in this repository",
        );
    }
}
