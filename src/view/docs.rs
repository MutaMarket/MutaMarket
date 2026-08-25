//! View DTOs of the documentation pages.

use serde::{Deserialize, Serialize};

/// A sidebar link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocNavItem {
    pub slug: String,
    pub title: String,
}

/// A sidebar section, like the legacy groupBy(section) output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocNavSection {
    pub title: String,
    pub pages: Vec<DocNavItem>,
}

/// Everything the page renders, like the legacy Inertia props (with the
/// previous/next neighbours precomputed server-side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentationData {
    pub sections: Vec<DocNavSection>,
    pub slug: String,
    pub section: String,
    pub title: String,
    pub html: String,
    pub edit_url: String,
    pub previous: Option<DocNavItem>,
    pub next: Option<DocNavItem>,
}

/// The legacy controller outcomes: 503 when the docs cannot load, 404 for
/// an unknown slug, otherwise the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentationOutcome {
    Unavailable,
    NotFound,
    Page(Box<DocumentationData>),
}
