//! The documentation pages: `/documentation` shows the first page and
//! `/documentation/{page}` a specific one, mirroring the legacy
//! ShowDocumentationPage.vue — sticky section sidebar, HUD-panel content
//! frame with the section label, GitHub edit link, rendered markdown
//! article, and previous/next footer links.
//!
//! Divergences from legacy: the mobile page picker is a native `<select>`
//! instead of the reka-ui Select, and the previous/next arrows are text
//! arrows instead of lucide icons.

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
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

/// The documentation page for a slug (`None` shows the first page, like
/// the legacy controller default).
#[server]
pub async fn fetch_documentation(
    page: Option<String>,
) -> Result<DocumentationOutcome, ServerFnError> {
    let pages = match crate::docs::pages() {
        Ok(pages) => pages,
        Err(_) => return Ok(DocumentationOutcome::Unavailable),
    };

    let slug = page.unwrap_or_else(|| pages[0].slug.clone());

    let Some(index) = pages.iter().position(|entry| entry.slug == slug) else {
        return Ok(DocumentationOutcome::NotFound);
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
        index.and_then(|index| pages.get(index)).map(|entry| DocNavItem {
            slug: entry.slug.clone(),
            title: entry.title.clone(),
        })
    };

    Ok(DocumentationOutcome::Page(Box::new(DocumentationData {
        sections,
        slug: current.slug.clone(),
        section: current.section.clone(),
        title: current.title.clone(),
        html: current.html.clone(),
        edit_url: crate::docs::edit_url(current),
        previous: neighbour(index.checked_sub(1)),
        next: neighbour(Some(index + 1)),
    })))
}

#[component]
pub fn DocumentationPage() -> impl IntoView {
    let params = use_params_map();
    let page = Memo::new(move |_| params.read().get("page"));
    let data = Resource::new(move || page.get(), fetch_documentation);

    view! {
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading documentation..."</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(DocumentationOutcome::Page(data)) => {
                        view! { <DocumentationView data=*data/> }.into_any()
                    }
                    Ok(DocumentationOutcome::NotFound) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                            response.set_status(axum::http::StatusCode::NOT_FOUND);
                        }

                        view! {
                            <Title text="Documentation - MutaMarket"/>
                            <h1 class="text-xl font-semibold">"Page not found"</h1>
                            <p class="mt-2 text-muted-foreground">
                                "This documentation page does not exist."
                            </p>
                        }
                        .into_any()
                    }
                    // The legacy 503 when the documentation cannot load.
                    _ => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                            response.set_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
                        }

                        view! {
                            <Title text="Documentation - MutaMarket"/>
                            <p>"The documentation is temporarily unavailable."</p>
                        }
                        .into_any()
                    }
                }
            })}
        </Suspense>
    }
}

#[component]
fn DocumentationView(data: DocumentationData) -> impl IntoView {
    let sidebar_sections = data.sections.clone();
    let select_sections = data.sections;
    let current_slug = data.slug.clone();
    let select_slug = data.slug;

    view! {
        <Title text=format!("{} - MutaMarket", data.title)/>
        <div class="lg:grid lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-6">
            <nav class="hud-panel hidden space-y-5 self-start p-4 lg:sticky lg:top-20 lg:block">
                {sidebar_sections
                    .into_iter()
                    .map(|section| {
                        let current_slug = current_slug.clone();
                        view! {
                            <div>
                                <span class="hud-label">{section.title}</span>
                                <ul class="mt-2 space-y-0.5">
                                    {section
                                        .pages
                                        .into_iter()
                                        .map(|entry| {
                                            let state = if entry.slug == current_slug {
                                                "border-primary text-foreground bg-primary/5"
                                            } else {
                                                "text-muted-foreground hover:text-foreground border-transparent"
                                            };
                                            let class = format!(
                                                "block border-l-2 px-3 py-1.5 text-sm transition-colors {state}",
                                            );

                                            view! {
                                                <li>
                                                    <A href=format!("/documentation/{}", entry.slug) attr:class=class>
                                                        {entry.title}
                                                    </A>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            </div>
                        }
                    })
                    .collect_view()}
            </nav>

            <div class="hud-panel min-w-0">
                <div class="border-border flex flex-wrap items-center justify-between gap-3 border-b px-6 py-4">
                    <div>
                        <span class="hud-label">{format!("Documentation // {}", data.section)}</span>
                        <h1 class="mt-1 text-2xl font-bold">{data.title}</h1>
                    </div>
                    <a
                        href=data.edit_url
                        class="text-muted-foreground hover:text-foreground inline-flex items-center gap-2 text-sm transition-colors"
                        rel="noopener noreferrer"
                        target="_blank"
                    >
                        "Edit this page on GitHub"
                    </a>
                </div>

                <div class="border-border border-b px-6 py-3 lg:hidden">
                    <select
                        class="border-border bg-background w-full border px-3 py-2 text-sm"
                        onchange="if (this.value) window.location.href = '/documentation/' + this.value"
                    >
                        {select_sections
                            .into_iter()
                            .map(|section| {
                                let select_slug = select_slug.clone();
                                view! {
                                    <optgroup label=section.title>
                                        {section
                                            .pages
                                            .into_iter()
                                            .map(|entry| {
                                                let selected = entry.slug == select_slug;
                                                view! {
                                                    <option value=entry.slug selected=selected>
                                                        {entry.title}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </optgroup>
                                }
                            })
                            .collect_view()}
                    </select>
                </div>

                <article class="docs-prose px-6 py-6 md:px-8" inner_html=data.html></article>

                <div class="border-border grid grid-cols-2 border-t">
                    {match data.previous {
                        Some(previous) => view! {
                            <A
                                href=format!("/documentation/{}", previous.slug)
                                attr:class="hover:bg-secondary/40 group flex flex-col gap-1 p-4 transition-colors"
                            >
                                <span class="hud-label inline-flex items-center gap-1.5">
                                    "\u{2190} Previous"
                                </span>
                                <span class="group-hover:text-primary text-sm font-medium transition-colors">
                                    {previous.title}
                                </span>
                            </A>
                        }
                        .into_any(),
                        None => view! { <div></div> }.into_any(),
                    }}
                    {data
                        .next
                        .map(|next| {
                            view! {
                                <A
                                    href=format!("/documentation/{}", next.slug)
                                    attr:class="hover:bg-secondary/40 group border-border flex flex-col items-end gap-1 border-l p-4 text-right transition-colors"
                                >
                                    <span class="hud-label inline-flex items-center gap-1.5">
                                        "Next \u{2192}"
                                    </span>
                                    <span class="group-hover:text-primary text-sm font-medium transition-colors">
                                        {next.title}
                                    </span>
                                </A>
                            }
                        })}
                </div>
            </div>
        </div>
    }
}
