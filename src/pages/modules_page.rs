//! The module routes: `/modules/{query}` shows a single module when the
//! query is a module slug or item id, and the module browser otherwise.
//!
//! The rendering mirrors the legacy Vue components:
//! - a masonry-style browser grid (`auto-fill` columns, cards spanning grid
//!   rows per content row so attribute rows align across cards),
//! - the module card with meta-group accent, type icon and per-attribute
//!   rows (formatted value, colored difference vs base),
//! - all attribute bar modes from the display settings: the default
//!   center-origin bar, the type-normalized bar with the mutaplasmid range
//!   band, the left-origin absolute bar with tick marks, or none, plus the
//!   optional -10..+10 attribute score.
//!
//! Filter segments (type, attributes, sorting) arrive with the search
//! milestone.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use super::filters::{FilterPanel, ModuleOptionsBar};
use crate::modules::view::{
    AssetLocationView, DisplaySettings, ModuleAttributeView, ModuleDetail, SearchFailure,
    format_fraction, location_flag_label, meta_group_key, module_id_from_slug,
};

/// One module with everything the detail page needs.
#[server]
pub async fn fetch_module(item_id: i64) -> Result<Option<ModuleDetail>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::modules::queries::module_detail(&state.pool, &state.reference, item_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The modules matching a filter query path, with full card data. The
/// browser shows the for-sale set like the legacy home; the all-modules
/// page includes unlisted modules.
#[server]
pub async fn fetch_search_modules(
    query: String,
    include_unlisted: bool,
) -> Result<Result<Vec<ModuleDetail>, SearchFailure>, ServerFnError> {
    /// Modules shown on the browser page.
    const BROWSER_PAGE_SIZE: i64 = 30;

    use crate::modules::search::{self, SearchError, Visibility};

    let state = expect_context::<crate::server::AppState>();

    let search = match search::parse(&state.pool, &state.reference, &query).await {
        Ok(search) => search,
        Err(SearchError::TypeNotFound) => {
            return Ok(Err(SearchFailure {
                message: "Please provide a valid type.".to_owned(),
                not_found: true,
            }));
        }
        Err(SearchError::Invalid(message)) => {
            return Ok(Err(SearchFailure { message, not_found: false }));
        }
        Err(SearchError::Db(error)) => return Err(ServerFnError::new(error.to_string())),
    };

    let visibility = if include_unlisted { Visibility::All } else { Visibility::ForSale };
    let ids = search::module_ids(&state.pool, &search, visibility, BROWSER_PAGE_SIZE)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;

    crate::modules::queries::details_for(&state.pool, &state.reference, ids)
        .await
        .map(Ok)
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The visitor's display preferences from the legacy display cookies.
#[server]
pub async fn fetch_display_settings() -> Result<DisplaySettings, ServerFnError> {
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;

    Ok(crate::server::display::settings_from_headers(&headers))
}

/// The display settings with the legacy defaults on any failure.
pub(crate) async fn fetch_display_settings_or_default() -> DisplaySettings {
    fetch_display_settings().await.unwrap_or_default()
}

/// Market-wide statistics for the browser header, the legacy
/// `getAllModulesStats`.
#[server]
pub async fn fetch_module_stats() -> Result<crate::modules::view::ModulesStats, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::modules::stats::all_modules_stats(&state.pool)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// A compact number for the stats strip, e.g. `12.3k`.
fn compact_count(value: i64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

#[component]
fn StatsStrip() -> impl IntoView {
    let stats = OnceResource::new(fetch_module_stats());

    view! {
        <Suspense fallback=|| ()>
            {move || Suspend::new(async move {
                let Ok(stats) = stats.await else {
                    return ().into_any();
                };

                let cell = |label: &'static str, value: i64| {
                    view! {
                        <div class="rounded-lg border border-border bg-card-1 px-3 py-2">
                            <div class="text-sm font-semibold text-white">
                                {compact_count(value)}
                            </div>
                            <div class="text-xs text-muted-foreground">{label}</div>
                        </div>
                    }
                };

                view! {
                    <div class="mb-4 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
                        {cell("Total modules", stats.total_count)}
                        {cell("For sale", stats.contracts_count)}
                        {cell("Auctions", stats.auctions_count)}
                        {cell("Added today", stats.added_last_day_count)}
                        {cell("Gold bars", stats.goldbars_count)}
                        {cell("Diamond bars", stats.diamondbars_count)}
                    </div>
                }
                .into_any()
            })}
        </Suspense>
    }
}

#[component]
pub fn ModulesPage() -> impl IntoView {
    let params = use_params_map();
    let query = Memo::new(move |_| params.read().get("query").unwrap_or_default());
    // Only the browser-vs-detail decision should rebuild this subtree. As a
    // `Memo` it notifies only when the id itself changes, so navigating
    // between filter states (both `None`) leaves the browser - and its module
    // grid - mounted, letting refetches swap in place instead of tearing the
    // grid down and flashing a loading state.
    let detail_id = Memo::new(move |_| module_id_from_slug(&query.get()));

    view! {
        {move || match detail_id.get() {
            Some(item_id) => view! { <ModuleDetailView item_id/> }.into_any(),
            None => view! { <ModuleBrowser/> }.into_any(),
        }}
    }
}

/// The all-modules page: the same browser including modules that are not
/// currently for sale, like the legacy AllModulesController.
#[component]
pub fn AllModulesPage() -> impl IntoView {
    view! { <ModuleBrowser include_unlisted=true/> }
}

#[component]
pub fn ModuleBrowser(#[prop(optional)] include_unlisted: bool) -> impl IntoView {
    // The URL is the source of truth. Reading the query reactively (rather
    // than as a snapshot prop) lets a filter change refetch this component in
    // place instead of remounting it.
    let params = use_params_map();
    let query = Memo::new(move |_| params.read().get("query").unwrap_or_default());

    // Keyed on the query: a filter change re-runs the fetch, and the
    // <Transition> below keeps the current grid visible until the new data
    // resolves, so the modules sit in place and swap instantly - no flash.
    let modules = Resource::new(
        move || query.get(),
        move |query| fetch_search_modules(query, include_unlisted),
    );
    let settings = OnceResource::new(fetch_display_settings());

    view! {
        <h1 class="mb-4 text-xl font-semibold">"Abyssal Modules"</h1>
        <StatsStrip/>
        <div class="my-4 flex flex-col items-start gap-4 lg:grid lg:grid-cols-[280px_1fr]">
            <FilterPanel query include_unlisted/>
            <div class="w-full">
                <Transition fallback=|| {
                    view! { <p class="text-muted-foreground">"Loading modules..."</p> }
                }>
                    {move || Suspend::new(async move {
                        let settings = settings.await.unwrap_or_default();

                        match modules.await {
                            Ok(Ok(modules)) if !modules.is_empty() => {
                                // A shared signal: the options bar mutates it
                                // and every card re-renders reactively.
                                let settings = RwSignal::new(settings);
                                view! {
                                    <ModuleOptionsBar settings/>
                                    <div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                                        {modules
                                            .into_iter()
                                            .map(|module| {
                                                view! { <ModuleCard module settings/> }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                .into_any()
                            }
                            Ok(Ok(_)) => view! { <p class="text-muted-foreground">"No modules match this search."</p> }.into_any(),
                            Ok(Err(failure)) => {
                                #[cfg(feature = "ssr")]
                                if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                                    response.set_status(if failure.not_found {
                                        axum::http::StatusCode::NOT_FOUND
                                    } else {
                                        axum::http::StatusCode::BAD_REQUEST
                                    });
                                }

                                view! { <p class="text-negative">{failure.message}</p> }.into_any()
                            }
                            Err(_) => {
                                #[cfg(feature = "ssr")]
                                if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                                    response.set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                                }
                                view! { <p>"Modules are unavailable right now."</p> }.into_any()
                            }
                        }
                    })}
                </Transition>
            </div>
        </div>
    }
}

#[component]
fn ModuleDetailView(item_id: i64) -> impl IntoView {
    let module = OnceResource::new(fetch_module(item_id));
    let settings = OnceResource::new(fetch_display_settings());

    view! {
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading module..."</p> }>
            {move || Suspend::new(async move {
                let settings = settings.await.unwrap_or_default();

                match module.await {
                    Ok(Some(module)) => view! {
                        <article class="grid items-start gap-4 md:grid-cols-[minmax(280px,380px)_1fr]">
                            <ModuleCard module=module.clone() settings/>
                            <section>
                                <h1 class="text-xl font-semibold">
                                    {module.r#type.name.clone()}
                                </h1>
                                <p class="mt-1 text-sm text-muted-foreground">
                                    {module
                                        .source_type
                                        .as_ref()
                                        .map(|source| format!("Mutated from {}", source.name))}
                                    {module
                                        .mutaplasmid
                                        .as_ref()
                                        .map(|mutaplasmid| format!(" with {}", mutaplasmid.name))}
                                </p>
                                {module.average_fraction.map(|fraction| {
                                    let quality_class =
                                        if fraction < 0.0 { "text-negative" } else { "text-positive" };

                                    view! {
                                        <p class="mt-2 text-sm">
                                            "Roll quality: "
                                            <span class=quality_class>{format_fraction(fraction)}</span>
                                        </p>
                                    }
                                })}
                                <p class="mt-2 text-sm text-muted-foreground">
                                    "Est. value: N/A"
                                </p>
                            </section>
                        </article>
                    }
                    .into_any(),
                    Ok(None) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) =
                            use_context::<leptos_axum::ResponseOptions>()
                        {
                            response.set_status(axum::http::StatusCode::NOT_FOUND);
                        }

                        view! {
                            <h1 class="text-xl font-semibold">"Module not found"</h1>
                            <p class="mt-2 text-muted-foreground">
                                "No module with this item id is known to MutaMarket."
                            </p>
                        }
                        .into_any()
                    }
                    Err(_) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                            response.set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        }
                        view! { <p>"This module is unavailable right now."</p> }.into_any()
                    }
                }
            })}
        </Suspense>
    }
}

/// The meta-group accent of the card header, like the legacy component.
fn meta_group_border(meta_group_id: Option<i64>) -> &'static str {
    match meta_group_key(meta_group_id) {
        "t2" => "border-b-orange-500",
        "storyline" => "border-b-green-300",
        "faction" => "border-b-green-500",
        "officer" => "border-b-purple-500",
        "deadspace" => "border-b-blue-500",
        _ => "border-b-gray-500",
    }
}

fn variant_text_class(variant: &'static str) -> &'static str {
    match variant {
        "gold" => "text-gold",
        "diamond" => "text-diamond",
        "brown" => "text-brown",
        "positive" => "text-positive",
        "positive-derived" => "text-positive-derived",
        "negative-derived" => "text-negative-derived",
        _ => "text-negative",
    }
}

fn variant_fill_class(variant: &'static str) -> &'static str {
    match variant {
        "gold" => "attribute-gold",
        "diamond" => "attribute-diamond",
        "brown" => "attribute-brown",
        "positive" => "attribute-positive",
        "positive-derived" => "attribute-positive-derived",
        "negative-derived" => "attribute-negative-derived",
        _ => "attribute-negative",
    }
}

#[component]
pub fn ModuleCard(
    module: ModuleDetail,
    /// Reactive display settings (a plain value or an `RwSignal`), threaded
    /// to the attribute rows so the options bar can re-render them live.
    #[prop(into)]
    settings: Signal<DisplaySettings>,
    /// The owner's asset location row, the legacy Grid `Asset.vue` footer.
    #[prop(optional)]
    asset: Option<AssetLocationView>,
) -> impl IntoView {
    let header_border = meta_group_border(module.source_type.as_ref().and_then(|source| source.meta_group_id));
    let icon_url = format!(
        "https://images.evetech.net/types/{}/icon?size=64",
        module.r#type.id,
    );
    let href = format!("/modules/{}", module.slug);

    let visual_attributes: Vec<ModuleAttributeView> = module
        .mutated_attributes
        .iter()
        .filter(|attribute| attribute.is_visual())
        .cloned()
        .collect();

    // Masonry alignment like the legacy grid: the card spans one container
    // row per content row (header + attributes + footer + location), so
    // attribute rows line up across neighboring cards.
    let row_span = format!(
        "grid-row: span {}",
        2 + visual_attributes.len() + usize::from(asset.is_some()),
    );

    let location = asset.map(|asset| {
        let icon = asset
            .parent_type_id
            .map(|type_id| format!("https://images.evetech.net/types/{type_id}/icon?size=64"));
        let flag = location_flag_label(&asset.location_flag);
        let href = format!("/locations/{}", asset.parent_slug);

        view! {
            <a
                class="relative grid grid-cols-[36px_1fr_auto] items-center gap-2 bg-card p-2"
                href=href
            >
                {icon.map(|icon| view! { <img alt="" class="size-9 rounded-lg" src=icon/> })}
                <div class="overflow-hidden py-[3px] text-xs">
                    <span class="block truncate font-medium">{asset.parent_name.clone()}</span>
                    <span class="truncate text-muted-foreground">{flag}</span>
                </div>
                <div class="pr-2 pl-4 font-medium">{asset.location_index + 1}</div>
            </a>
        }
    });

    view! {
        <div class="grid overflow-hidden rounded-lg border border-border" style=row_span>
            <div class=format!(
                "relative grid h-[50px] grid-cols-[36px_1fr] content-center items-center gap-x-2 border-b-2 bg-card-1 p-2 {header_border}",
            )>
                <img alt="" class="row-span-2 size-8 rounded-lg" src=icon_url/>
                <a class="truncate text-sm text-white" href=href>
                    {module
                        .source_type
                        .as_ref()
                        .map(|source| source.name.clone())
                        .unwrap_or_else(|| module.r#type.name.clone())}
                    <span aria-hidden="true" class="absolute inset-0"></span>
                </a>
                <span class="mt-1 truncate text-xs text-muted-foreground">
                    {module.mutaplasmid.as_ref().map(|m| m.name.clone()).unwrap_or_default()}
                </span>
            </div>
            {visual_attributes
                .into_iter()
                .map(|attribute| view! { <AttributeRow attribute settings/> })
                .collect_view()}
            <div class="grid h-[50px] content-center bg-card-1 px-2 text-xs text-muted-foreground">
                "Est. value: N/A"
            </div>
            {location}
        </div>
    }
}

#[component]
fn AttributeRow(
    attribute: ModuleAttributeView,
    /// A reactive signal so toggling the display-options bar re-renders the
    /// score and bar without reloading. Callers may pass a plain
    /// `DisplaySettings` (a static signal) or an `RwSignal`.
    #[prop(into)]
    settings: Signal<DisplaySettings>,
) -> impl IntoView {
    let variant = attribute.variant();
    let display_name = if attribute.display_name.is_empty() {
        attribute.name.clone()
    } else {
        attribute.display_name.clone()
    };
    let formatted_value = attribute.formatted_value();
    let formatted_difference = attribute.formatted_difference();
    let attr = StoredValue::new(attribute);

    let score = move || {
        settings.get().show_attribute_scores.then(|| {
            let attribute = attr.get_value();
            view! {
                <span class=format!(
                    "col-start-3 row-span-2 row-start-1 inline-block text-sm font-medium {}",
                    attribute.score_class(),
                )>{attribute.score_label()}</span>
            }
        })
    };

    let bar = move || {
        let attribute = attr.get_value();
        let inner = match settings.get().attribute_bar_mode.as_str() {
            "none" => None,
            "type" => Some(
                view! {
                    <RollBarTypeNormalized
                        fraction_type=attribute.fraction_type
                        band=attribute.type_band
                        variant
                    /> }
                .into_any(),
            ),
            "absolute" => Some(
                view! {
                    <RollBarAbsolute fraction_absolute=attribute.fraction_absolute bar=attribute.bar/>
                }
                .into_any(),
            ),
            _ => Some(view! { <RollBar fraction=attribute.fraction variant/> }.into_any()),
        };
        inner.map(|bar| view! { <div class="col-span-full my-1">{bar}</div> })
    };

    view! {
        <div class="grid grid-cols-[1fr_auto_auto] content-center items-center gap-x-2 bg-card-2 px-2 py-1">
            <div class="text-xs text-muted-foreground">{display_name}</div>
            <div class="flex gap-1 text-sm text-white">
                <span>{formatted_value}</span>
                <span class=variant_text_class(variant)>{formatted_difference}</span>
            </div>
            {score}
            {bar}
        </div>
    }
}

/// The default bar: center origin, positive rolls grow right, negative left.
#[component]
fn RollBar(fraction: f64, variant: &'static str) -> impl IntoView {
    let width = format!("width: {}%", (fraction.abs() * 50.0).min(50.0));
    let fill = variant_fill_class(variant);

    view! {
        <div class="relative h-[3px] bg-card-1">
            {if fraction > 0.0 {
                view! {
                    <div
                        class="absolute left-1/2 h-full origin-left border-r border-white"
                        style=width
                    >
                        <div class=format!("h-full w-full {fill}")></div>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div
                        class="absolute right-1/2 h-full origin-right border-l border-white"
                        style=width
                    >
                        <div class=format!("h-full w-full {fill}")></div>
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

/// The type-normalized bar: the fill shows `fraction_type` against the
/// whole type's range, and the band highlights the share of that range the
/// module's own mutaplasmid can roll.
#[component]
fn RollBarTypeNormalized(
    fraction_type: f64,
    band: Option<(f64, f64)>,
    variant: &'static str,
) -> impl IntoView {
    let width = format!("width: {}%", (fraction_type.abs() * 50.0).min(50.0));
    let fill = variant_fill_class(variant);

    let band = band.map(|(min, max)| {
        let style = format!("left: {}%; right: {}%", 50.0 - min * 50.0, 50.0 - max * 50.0);
        view! { <div class="absolute top-0 bottom-0 left-0 bg-white/25" style=style></div> }
    });

    view! {
        <div class="relative h-[3px] bg-background">
            {band}
            {if fraction_type >= 0.0 {
                view! {
                    <div
                        class="absolute left-1/2 h-full origin-left border-r border-white"
                        style=width
                    >
                        <div class=format!("h-full w-full {fill}")></div>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div
                        class="absolute right-1/2 h-full origin-right border-l border-white"
                        style=width
                    >
                        <div class=format!("h-full w-full {fill}")></div>
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

/// The absolute bar: left origin over the whole 0..1 absolute fraction,
/// with 20 tick marks and the primary fill unless a best/worst marker wins.
#[component]
fn RollBarAbsolute(fraction_absolute: f64, bar: i16) -> impl IntoView {
    /// Tick marks of the legacy component.
    const STEPS: i32 = 20;

    let width = format!("width: {}%", (fraction_absolute * 100.0).clamp(0.0, 100.0));
    let fill = match bar {
        1 => "attribute-gold",
        2 => "attribute-diamond",
        -1 => "attribute-brown",
        _ => "attribute-absolute",
    };

    view! {
        <div class="relative h-[3px] bg-card">
            {(1..=STEPS)
                .map(|i| {
                    let style = format!("left: {}%", f64::from(i) * 100.0 / f64::from(STEPS + 1));
                    view! { <div class="absolute h-full w-[1px] bg-card-2" style=style></div> }
                })
                .collect_view()}
            <div class="absolute left-0 h-full origin-left border-r border-white" style=width>
                <div class=format!("h-full w-full {fill}")></div>
            </div>
        </div>
    }
}
