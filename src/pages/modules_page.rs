//! The module routes: `/modules/{query}` shows a single module when the
//! query is a module slug or item id, and the module browser otherwise.
//! Filter segments (type, attributes, sorting) arrive with the search
//! milestone.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::modules::view::{
    ModuleDetail, ModuleSummary, format_fraction, format_number, module_id_from_slug,
};

/// One module with everything the detail page needs.
#[server]
pub async fn fetch_module(item_id: i64) -> Result<Option<ModuleDetail>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::modules::queries::module_detail(&state.pool, item_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The newest modules for the browser.
#[server]
pub async fn fetch_recent_modules() -> Result<Vec<ModuleSummary>, ServerFnError> {
    /// Modules shown on the browser page.
    const BROWSER_PAGE_SIZE: i64 = 50;

    let state = expect_context::<crate::server::AppState>();

    crate::modules::queries::recent_modules(&state.pool, BROWSER_PAGE_SIZE)
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
        <h1>"Abyssal Modules"</h1>
        <Suspense fallback=|| view! { <p>"Loading modules..."</p> }>
            {move || Suspend::new(async move {
                match modules.await {
                    Ok(modules) if !modules.is_empty() => view! {
                        <ul class="module-list">
                            {modules
                                .into_iter()
                                .map(|module| view! { <ModuleListEntry module/> })
                                .collect_view()}
                        </ul>
                    }
                    .into_any(),
                    Ok(_) => view! { <p>"No modules yet."</p> }.into_any(),
                    Err(_) => view! { <p>"Modules are unavailable right now."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn ModuleListEntry(module: ModuleSummary) -> impl IntoView {
    let href = format!("/modules/{}", module.slug);
    let roll_quality = module.average_fraction.map(format_fraction);

    view! {
        <li class="module-list-entry">
            <a href=href>{module.type_name}</a>
            {roll_quality.map(|quality| view! { <span class="roll-quality">{quality}</span> })}
        </li>
    }
}

#[component]
fn ModuleDetailView(item_id: i64) -> impl IntoView {
    let module = OnceResource::new(fetch_module(item_id));

    view! {
        <Suspense fallback=|| view! { <p>"Loading module..."</p> }>
            {move || Suspend::new(async move {
                match module.await {
                    Ok(Some(module)) => view! { <ModuleDetailContent module/> }.into_any(),
                    Ok(None) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) =
                            use_context::<leptos_axum::ResponseOptions>()
                        {
                            response.set_status(axum::http::StatusCode::NOT_FOUND);
                        }

                        view! {
                            <h1>"Module not found"</h1>
                            <p>"No module with this item id is known to MutaMarket."</p>
                        }
                        .into_any()
                    }
                    Err(_) => view! { <p>"This module is unavailable right now."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn ModuleDetailContent(module: ModuleDetail) -> impl IntoView {
    view! {
        <article class="module-detail">
            <h1>{module.summary.type_name.clone()}</h1>
            <p class="module-meta">
                {module
                    .source_type_name
                    .as_ref()
                    .map(|source| format!("Mutated from {source}"))}
                {module
                    .mutaplasmid_name
                    .as_ref()
                    .map(|mutaplasmid| format!(" with {mutaplasmid}"))}
            </p>
            {module.summary.average_fraction.map(|fraction| view! {
                <p class="average-fraction">
                    "Roll quality: " {format_fraction(fraction)}
                </p>
            })}
            <table class="attributes">
                <thead>
                    <tr>
                        <th>"Attribute"</th>
                        <th>"Value"</th>
                        <th>"Base"</th>
                        <th>"Roll"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {module
                        .attributes
                        .iter()
                        .map(|attribute| {
                            let bar = match attribute.bar {
                                -1 => Some(("bar-brown", "worst roll")),
                                1 => Some(("bar-gold", "best roll")),
                                2 => Some(("bar-diamond", "best known roll")),
                                _ => None,
                            };

                            view! {
                                <tr class:virtual-attribute=attribute.is_virtual>
                                    <td>{attribute.name.clone()}</td>
                                    <td>{format_number(attribute.value)}</td>
                                    <td>{format_number(attribute.base_value)}</td>
                                    <td>{format_fraction(attribute.fraction)}</td>
                                    <td>
                                        {bar.map(|(class, label)| view! {
                                            <span class=class>{label}</span>
                                        })}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </article>
    }
}
