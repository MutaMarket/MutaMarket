//! The module routes: `/modules/{query}` shows a single module when the
//! query is a module slug or item id, and the module browser otherwise.
//! The card mirrors the legacy Vue module card: meta-group accent, header
//! with the type icon, and per-attribute rows with formatted values,
//! colored differences and the center-origin roll bar. Filter segments
//! (type, attributes, sorting) arrive with the search milestone.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::modules::view::{
    ModuleAttributeView, ModuleDetail, format_fraction, meta_group_key, module_id_from_slug,
};

/// One module with everything the detail page needs.
#[server]
pub async fn fetch_module(item_id: i64) -> Result<Option<ModuleDetail>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::modules::queries::module_detail(&state.pool, item_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The newest modules for the browser, with full card data.
#[server]
pub async fn fetch_recent_modules() -> Result<Vec<ModuleDetail>, ServerFnError> {
    /// Modules shown on the browser page.
    const BROWSER_PAGE_SIZE: i64 = 30;

    let state = expect_context::<crate::server::AppState>();

    crate::modules::queries::recent_module_cards(&state.pool, BROWSER_PAGE_SIZE)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[component]
pub fn ModulesPage() -> impl IntoView {
    let params = use_params_map();
    let query = Memo::new(move |_| params.read().get("query").unwrap_or_default());

    view! {
        {move || match module_id_from_slug(&query.get()) {
            Some(item_id) => view! { <ModuleDetailView item_id/> }.into_any(),
            None => view! { <ModuleBrowser/> }.into_any(),
        }}
    }
}

#[component]
pub fn ModuleBrowser() -> impl IntoView {
    let modules = OnceResource::new(fetch_recent_modules());

    view! {
        <h1 class="mb-4 text-xl font-semibold">"Abyssal Modules"</h1>
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading modules..."</p> }>
            {move || Suspend::new(async move {
                match modules.await {
                    Ok(modules) if !modules.is_empty() => view! {
                        <div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                            {modules
                                .into_iter()
                                .map(|module| view! { <ModuleCard module/> })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                    Ok(_) => view! { <p class="text-muted-foreground">"No modules yet."</p> }.into_any(),
                    Err(_) => view! { <p>"Modules are unavailable right now."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn ModuleDetailView(item_id: i64) -> impl IntoView {
    let module = OnceResource::new(fetch_module(item_id));

    view! {
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading module..."</p> }>
            {move || Suspend::new(async move {
                match module.await {
                    Ok(Some(module)) => view! {
                        <article class="grid gap-4 md:grid-cols-[minmax(280px,380px)_1fr]">
                            <ModuleCard module=module.clone()/>
                            <section>
                                <h1 class="text-xl font-semibold">
                                    {module.summary.type_name.clone()}
                                </h1>
                                <p class="mt-1 text-sm text-muted-foreground">
                                    {module
                                        .source_type_name
                                        .as_ref()
                                        .map(|source| format!("Mutated from {source}"))}
                                    {module
                                        .mutaplasmid_name
                                        .as_ref()
                                        .map(|mutaplasmid| format!(" with {mutaplasmid}"))}
                                </p>
                                {module.summary.average_fraction.map(|fraction| {
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
                    Err(_) => view! { <p>"This module is unavailable right now."</p> }.into_any(),
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
pub fn ModuleCard(module: ModuleDetail) -> impl IntoView {
    let header_border = meta_group_border(module.source_meta_group_id);
    let icon_url = format!(
        "https://images.evetech.net/types/{}/icon?size=64",
        module.summary.type_id,
    );
    let href = format!("/modules/{}", module.summary.slug);

    let visual_attributes: Vec<ModuleAttributeView> = module
        .attributes
        .iter()
        .filter(|attribute| attribute.is_visual())
        .cloned()
        .collect();

    view! {
        <div class="grid overflow-hidden rounded-lg border border-border">
            <div class=format!(
                "relative grid h-[50px] grid-cols-[36px_1fr] content-center items-center gap-x-2 border-b-2 bg-card-1 p-2 {header_border}",
            )>
                <img alt="" class="row-span-2 size-8 rounded-lg" src=icon_url/>
                <a class="truncate text-sm text-white" href=href>
                    {module
                        .source_type_name
                        .clone()
                        .unwrap_or_else(|| module.summary.type_name.clone())}
                    <span aria-hidden="true" class="absolute inset-0"></span>
                </a>
                <span class="mt-1 truncate text-xs text-muted-foreground">
                    {module.mutaplasmid_name.clone().unwrap_or_default()}
                </span>
            </div>
            {visual_attributes
                .into_iter()
                .map(|attribute| view! { <AttributeRow attribute/> })
                .collect_view()}
            <div class="bg-card-1 px-2 py-1.5 text-xs text-muted-foreground">
                "Est. value: N/A"
            </div>
        </div>
    }
}

#[component]
fn AttributeRow(attribute: ModuleAttributeView) -> impl IntoView {
    let variant = attribute.variant();
    let display_name = if attribute.display_name.is_empty() {
        attribute.name.clone()
    } else {
        attribute.display_name.clone()
    };

    view! {
        <div class="grid grid-cols-[1fr_auto] content-center items-center gap-x-2 bg-card-2 px-2 py-1">
            <div class="text-xs text-muted-foreground">{display_name}</div>
            <div class="flex gap-1 text-sm text-white">
                <span>{attribute.formatted_value()}</span>
                <span class=variant_text_class(variant)>{attribute.formatted_difference()}</span>
            </div>
            <div class="col-span-full my-1">
                <RollBar fraction=attribute.fraction variant/>
            </div>
        </div>
    }
}

/// The center-origin roll bar: positive fractions grow right from the
/// middle, negative ones left; the fill carries the variant styling.
#[component]
fn RollBar(fraction: f64, variant: &'static str) -> impl IntoView {
    let width = format!("width: {}%", (fraction.abs() * 50.0).min(50.0));
    let fill = variant_fill_class(variant);

    view! {
        <div class="relative h-[3px] bg-card-1">
            {if fraction > 0.0 {
                view! {
                    <div
                        class="absolute left-1/2 h-full origin-left border-r border-foreground"
                        style=width
                    >
                        <div class=format!("h-full w-full {fill}")></div>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div
                        class="absolute right-1/2 h-full origin-right border-l border-foreground"
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
