//! The module browser's filter sidebar and display options bar, mirroring
//! the legacy Vue filter components:
//!
//! - `ModuleOptionsBar` — the display / attribute-bar-mode / score toggles
//!   that persist to the display cookies and reload (legacy
//!   `ModuleOptions.vue` PUTs to `/display`).
//! - `FilterPanel` — sort buttons, contract filters, price and estimated
//!   value bounds, the meta group select and, when a type is selected,
//!   one range slider per mutated attribute (legacy `AttributeFilters.vue`).
//! - `RangeSlider` — a pointer-driven two-thumb slider on the normalized
//!   0..100 scale, replacing the legacy `vue-slider-component`.
//!
//! The URL is the single source of truth: every control edits the parsed
//! [`UiSearch`] and navigates to the rebuilt query path, exactly like the
//! legacy `QueryBuilder`-driven Inertia visits.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use super::filter_controls::{ContractTypeButton, FilterCheckbox, SortButton};
use super::type_dialog::TypeDialog;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::input::Input;
use crate::components::ui::select::{
    Select, SelectContent, SelectGroup, SelectOption, SelectTrigger, SelectValue,
};


use crate::modules::view::{
    DisplaySettings, FilterAttribute, FilterPanelData, UiAttributeFilter, UiSearch,
    build_query_path, format_value, meta_group_key, to_normalized, to_original,
};

/// Resolves the type segment like the search does and returns the slider
/// bounds for each of its mutated attributes.
#[server]
pub async fn fetch_filter_panel(
    type_slug: String,
) -> Result<Option<FilterPanelData>, ServerFnError> {
    use crate::modules::search::{self, SearchError};

    let state = expect_context::<crate::server::AppState>();

    let type_filter = match search::resolve_type(&state.pool, &type_slug).await {
        Ok(type_filter) => type_filter,
        Err(SearchError::TypeNotFound) => return Ok(None),
        Err(error) => return Err(ServerFnError::new(error.to_string())),
    };

    let attributes = crate::modules::queries::type_filter_attributes(&state.pool, type_filter.id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;

    Ok(Some(FilterPanelData {
        type_id: type_filter.id,
        type_name: type_filter.name,
        attributes,
    }))
}

/// Persists the display settings to the legacy cookies, like `PUT /display`.
#[server]
pub async fn update_display_settings(settings: DisplaySettings) -> Result<(), ServerFnError> {
    use crate::modules::view::{ATTRIBUTE_BAR_MODES, DISPLAY_VALUES};

    if !DISPLAY_VALUES.contains(&settings.display.as_str())
        || !ATTRIBUTE_BAR_MODES.contains(&settings.attribute_bar_mode.as_str())
    {
        return Err(ServerFnError::new("The given data was invalid."));
    }

    let response = expect_context::<leptos_axum::ResponseOptions>();
    for cookie in crate::server::display::settings_cookies(&settings) {
        let value = axum::http::HeaderValue::from_str(&cookie)
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        response.append_header(axum::http::header::SET_COOKIE, value);
    }

    Ok(())
}

/// The bar above the browser grid switching display mode, attribute bar
/// mode and attribute scores. Takes a shared `RwSignal` so a toggle updates
/// every card reactively (no page reload); the cookie is persisted in the
/// background for the next visit.
#[component]
pub fn ModuleOptionsBar(settings: RwSignal<DisplaySettings>) -> impl IntoView {
    let save = Action::new(|settings: &DisplaySettings| update_display_settings(settings.clone()));

    let option_button = move |label: &'static str,
                             active: bool,
                             disabled: bool,
                             next: DisplaySettings| {
        let variant = if active { ButtonVariant::Default } else { ButtonVariant::Outline };
        let title = disabled.then_some("Coming soon");

        view! {
            <Button
                variant=variant
                size=ButtonSize::Sm
                class="h-7 px-2 text-xs"
                attr:disabled=disabled
                attr:title=title
                on:click=move |_| {
                    // Update the shared signal (instant re-render of the
                    // cards) and persist the cookie for the next visit.
                    settings.set(next.clone());
                    save.dispatch(next.clone());
                }
            >
                {label}
            </Button>
        }
    };

    view! {
        <div class="mb-2 flex flex-wrap items-center gap-4 rounded-lg border border-border bg-card-1 p-2">
            {move || {
                let current = settings.get();
                view! {
                    <div class="flex items-center gap-1">
                        <span class="mr-1 text-xs text-muted-foreground">"View"</span>
                        {["grid", "list", "table"]
                            .into_iter()
                            .map(|display| {
                                let mut next = current.clone();
                                next.display = display.to_owned();
                                option_button(
                                    display_label(display),
                                    current.display == display,
                                    display != "grid",
                                    next,
                                )
                            })
                            .collect_view()}
                    </div>
                    <div class="flex items-center gap-1">
                        <span class="mr-1 text-xs text-muted-foreground">"Bars"</span>
                        {["default", "type", "absolute", "none"]
                            .into_iter()
                            .map(|mode| {
                                let mut next = current.clone();
                                next.attribute_bar_mode = mode.to_owned();
                                option_button(
                                    bar_mode_label(mode),
                                    current.attribute_bar_mode == mode,
                                    false,
                                    next,
                                )
                            })
                            .collect_view()}
                    </div>
                    <div class="flex items-center gap-1">
                        <span class="mr-1 text-xs text-muted-foreground">"Scores"</span>
                        {[false, true]
                            .into_iter()
                            .map(|scores| {
                                let mut next = current.clone();
                                next.show_attribute_scores = scores;
                                option_button(
                                    if scores { "On" } else { "Off" },
                                    current.show_attribute_scores == scores,
                                    false,
                                    next,
                                )
                            })
                            .collect_view()}
                    </div>
                }
            }}
        </div>
    }
}

fn display_label(display: &str) -> &'static str {
    match display {
        "list" => "List",
        "table" => "Table",
        _ => "Grid",
    }
}

fn bar_mode_label(mode: &str) -> &'static str {
    match mode {
        "type" => "Type",
        "absolute" => "Absolute",
        "none" => "None",
        _ => "Default",
    }
}

/// The filter sidebar. Every control edits the current [`UiSearch`] and
/// navigates to the rebuilt query path. The panel persists across filter
/// navigations and reads the URL reactively, so changing a filter never
/// remounts the sidebar; each control derives just the slice it displays so
/// it only re-renders when that slice changes (a slider move never disturbs
/// the sort/contract sections or the sliders themselves).
#[component]
pub fn FilterPanel(
    #[prop(into)] query: Signal<String>,
    #[prop(optional)] include_unlisted: bool,
) -> impl IntoView {
    let prefix = if include_unlisted { "all-modules" } else { "modules" };
    // The parsed URL, always current. A `Memo` so it only notifies when the
    // search actually changes.
    let search = Memo::new(move |_| crate::modules::view::parse_query_ui(&query.get()));

    let navigate = use_navigate();
    let go = Callback::new(move |next: UiSearch| {
        navigate(&build_query_path(prefix, &next), Default::default());
    });

    // The attribute panel is keyed on the selected type only, so moving a
    // slider (which changes `search` but not the type) neither refetches the
    // bounds nor rebuilds the sliders. <Transition> keeps the current sliders
    // visible while a genuine type change loads.
    let type_slug = Memo::new(move |_| search.get().type_slug);
    let panel = Resource::new(
        move || type_slug.get(),
        move |type_slug| async move {
            match type_slug {
                Some(type_slug) => fetch_filter_panel(type_slug).await,
                None => Ok(None),
            }
        },
    );

    let attribute_section = view! {
        <Transition fallback=|| {
            view! { <p class="text-xs text-muted-foreground">"Loading attributes..."</p> }
        }>
            {move || Suspend::new(async move {
                match panel.await {
                    Ok(Some(data)) => view! {
                        <TypeDialog
                            prefix
                            search
                            current_type_id=data.type_id
                            current_type_name=data.type_name.clone()
                        />
                        <div class="mt-3 mb-1 flex items-center justify-between">
                            <h3 class="text-sm font-medium text-white">{data.type_name.clone()}</h3>
                            <button
                                class="text-xs text-muted-foreground hover:text-white"
                                on:click=move |_| {
                                    let mut next = search.get_untracked();
                                    next.type_slug = None;
                                    next.attributes.clear();
                                    next.sort = None;
                                    go.run(next);
                                }
                            >
                                "Clear type"
                            </button>
                        </div>
                        <div class="flex flex-col gap-3">
                            {data
                                .attributes
                                .into_iter()
                                .map(|attribute| {
                                    view! { <AttributeSlider attribute search go/> }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                    // A type was requested but did not resolve.
                    Ok(None) if type_slug.get_untracked().is_some() => view! {
                        <TypeDialog prefix search/>
                        <p class="mt-2 text-xs text-muted-foreground">"Unknown type."</p>
                    }
                    .into_any(),
                    Ok(None) => view! { <TypeDialog prefix search/> }.into_any(),
                    Err(_) => view! {
                        <TypeDialog prefix search/>
                        <p class="mt-2 text-xs text-muted-foreground">"Unknown type."</p>
                    }
                    .into_any(),
                }
            })}
        </Transition>
    }
    .into_any();

    // Boxed sections keep the statically-typed view tree shallow enough
    // for the compiler.
    let sections: Vec<(&'static str, AnyView)> = vec![
        ("Category", attribute_section),
        ("Sort", view! { <SortButtons search go/> }.into_any()),
        ("Contracts", view! { <ContractFilters search go/> }.into_any()),
        (
            "Price",
            view! {
                <BoundsInputs
                    lower_placeholder="Min price"
                    upper_placeholder="Max price"
                    initial=search.get_untracked().price
                    on_commit=Callback::new(move |bounds| {
                        let mut next = search.get_untracked();
                        next.price = bounds;
                        go.run(next);
                    })
                />
            }
            .into_any(),
        ),
        (
            "Estimated value",
            view! {
                <BoundsInputs
                    lower_placeholder="Min value"
                    upper_placeholder="Max value"
                    initial=search.get_untracked().value
                    on_commit=Callback::new(move |bounds| {
                        let mut next = search.get_untracked();
                        next.value = bounds;
                        go.run(next);
                    })
                />
            }
            .into_any(),
        ),
        ("Meta group", view! { <MetaGroupSelect search go/> }.into_any()),
    ];

    view! {
        <aside class="flex flex-col gap-4 rounded-lg border border-border bg-card-1 p-3">
            {sections
                .into_iter()
                .map(|(title, content)| {
                    view! {
                        <section>
                            <h2 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                                {title}
                            </h2>
                            {content}
                        </section>
                    }
                })
                .collect_view()}
        </aside>
    }
}

/// One attribute's range slider, working on the normalized 0..100 scale
/// like the legacy `AttributeFilter.vue` + `AttributeMapper`.
#[component]
fn AttributeSlider(
    attribute: FilterAttribute,
    #[prop(into)] search: Signal<UiSearch>,
    go: Callback<UiSearch>,
) -> impl IntoView {
    let FilterAttribute {
        name, display_name, unit_name, unit_display_name, best, worst, ..
    } = attribute;

    let title = if display_name.is_empty() { name.clone() } else { display_name };

    // Initial thumb positions from the URL, read untracked: the slider owns
    // its position after mount, so a drag must not re-seed it.
    let initial = search
        .get_untracked()
        .attributes
        .iter()
        .find(|filter| filter.name.eq_ignore_ascii_case(&name))
        .map(|filter| {
            let lower = to_normalized(filter.lower, best, worst).clamp(0.0, 100.0).round();
            let upper = filter
                .upper
                .map(|upper| to_normalized(upper, best, worst).clamp(0.0, 100.0).round())
                .unwrap_or(100.0);
            (lower.min(upper), lower.max(upper))
        })
        .unwrap_or((0.0, 100.0));

    let values = RwSignal::new(initial);

    let label = {
        let unit_name = unit_name.clone();
        let unit_display_name = unit_display_name.clone();
        Callback::new(move |normalized: f64| {
            format_value(
                to_original(normalized, best, worst),
                unit_name.as_deref(),
                unit_display_name.as_deref(),
            )
        })
    };

    let on_commit = Callback::new(move |(lower, upper): (f64, f64)| {
        let mut next = search.get_untracked();
        next.attributes.retain(|filter| !filter.name.eq_ignore_ascii_case(&name));

        // The legacy search composable: a fully open slider means no
        // filter; an open upper end keeps only the lower bound (the
        // backend resolves its direction from high-is-good).
        if (lower, upper) != (0.0, 100.0) {
            let (raw_lower, raw_upper) = (
                to_original(lower, best, worst),
                to_original(upper, best, worst),
            );
            if upper == 100.0 {
                next.attributes.push(UiAttributeFilter {
                    name: name.clone(),
                    lower: raw_lower,
                    upper: None,
                });
            } else {
                next.attributes.push(UiAttributeFilter {
                    name: name.clone(),
                    lower: raw_lower.min(raw_upper),
                    upper: Some(raw_lower.max(raw_upper)),
                });
            }
        }

        go.run(next);
    });

    view! {
        <div>
            <div class="mb-1 flex items-center justify-between text-xs">
                <span class="text-muted-foreground">{title}</span>
                <span class="text-white">
                    {move || {
                        let (lower, upper) = values.get();
                        format!("{} - {}", label.run(lower), label.run(upper))
                    }}
                </span>
            </div>
            <RangeSlider values on_commit/>
        </div>
    }
}

/// The thumb being dragged.
#[derive(Clone, Copy, PartialEq)]
enum Thumb {
    Lower,
    Upper,
}

/// A two-thumb range slider over 0..100 in steps of 1. Committing happens
/// on pointer release, so a drag causes one navigation, not many.
#[component]
pub fn RangeSlider(
    values: RwSignal<(f64, f64)>,
    #[prop(into)] on_commit: Callback<(f64, f64)>,
) -> impl IntoView {
    let track = NodeRef::<leptos::html::Div>::new();
    let dragging = RwSignal::new(None::<Thumb>);

    let value_at = move |client_x: i32| -> f64 {
        let Some(element) = track.get_untracked() else {
            return 0.0;
        };
        let rect = element.get_bounding_client_rect();
        let width = rect.width().max(1.0);
        ((f64::from(client_x) - rect.left()) / width * 100.0).clamp(0.0, 100.0).round()
    };

    let move_thumb = move |thumb: Thumb, value: f64| {
        values.update(|(lower, upper)| match thumb {
            Thumb::Lower => *lower = value.min(*upper),
            Thumb::Upper => *upper = value.max(*lower),
        });
    };

    let on_pointerdown = move |event: leptos::ev::PointerEvent| {
        let value = value_at(event.client_x());
        let (lower, upper) = values.get_untracked();

        // The nearest thumb moves; when both sit together, going left grabs
        // the lower thumb and going right the upper.
        let thumb = if (value - lower).abs() < (value - upper).abs() {
            Thumb::Lower
        } else if (value - lower).abs() > (value - upper).abs() {
            Thumb::Upper
        } else if value < lower {
            Thumb::Lower
        } else {
            Thumb::Upper
        };

        if let Some(element) = track.get_untracked() {
            let _ = element.set_pointer_capture(event.pointer_id());
        }
        dragging.set(Some(thumb));
        move_thumb(thumb, value);
    };

    let on_pointermove = move |event: leptos::ev::PointerEvent| {
        if let Some(thumb) = dragging.get_untracked() {
            move_thumb(thumb, value_at(event.client_x()));
        }
    };

    let on_pointerup = move |_: leptos::ev::PointerEvent| {
        if dragging.get_untracked().is_some() {
            dragging.set(None);
            on_commit.run(values.get_untracked());
        }
    };

    view! {
        <div
            class="cursor-pointer touch-none select-none py-2"
            node_ref=track
            on:pointerdown=on_pointerdown
            on:pointermove=on_pointermove
            on:pointerup=on_pointerup
            on:pointercancel=on_pointerup
        >
            <div class="relative h-1 rounded bg-card-2">
                <div
                    class="absolute h-full rounded bg-primary"
                    style=move || {
                        let (lower, upper) = values.get();
                        format!("left: {lower}%; width: {}%", upper - lower)
                    }
                ></div>
                <div
                    class="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-border bg-white"
                    style=move || format!("left: {}%", values.get().0)
                ></div>
                <div
                    class="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-border bg-white"
                    style=move || format!("left: {}%", values.get().1)
                ></div>
            </div>
        </div>
    }
}

/// Sort toggles for price, estimated value and roll quality, like the
/// legacy sort buttons: click cycles ascending, descending, off.
#[component]
fn SortButtons(#[prop(into)] search: Signal<UiSearch>, go: Callback<UiSearch>) -> impl IntoView {
    // Depend only on the sort slice: other filter changes leave the buttons
    // untouched. Each button renders once and updates its variant/arrow in
    // place, so changing the sort never rebuilds the group.
    let sort = Memo::new(move |_| search.get().sort);
    let on_change = Callback::new(move |next: Option<(String, bool)>| {
        let mut search = search.get_untracked();
        search.sort = next;
        go.run(search);
    });

    view! {
        <div class="flex flex-wrap gap-1">
            <SortButton field="price" label="Price" sort on_change/>
            <SortButton field="value" label="Est. value" sort on_change/>
            <SortButton field="fraction" label="Roll quality" sort on_change/>
        </div>
    }
}

/// Contract type radios and the boolean filter flags.
#[component]
fn ContractFilters(#[prop(into)] search: Signal<UiSearch>, go: Callback<UiSearch>) -> impl IntoView {
    let contract_type = Memo::new(move |_| search.get().contract_type);
    let on_select = Callback::new(move |value: Option<String>| {
        let mut search = search.get_untracked();
        search.contract_type = value;
        go.run(search);
    });

    // Each flag renders once with a reactive `checked` derived from its field,
    // so toggling one updates only that checkbox and an attribute or sort
    // change leaves them all untouched.
    let flag = move |label: &'static str,
                     get: fn(&UiSearch) -> bool,
                     set: fn(&mut UiSearch, bool)| {
        let checked = Signal::derive(move || get(&search.get()));
        let on_toggle = Callback::new(move |on: bool| {
            let mut search = search.get_untracked();
            set(&mut search, on);
            go.run(search);
        });

        view! { <FilterCheckbox label checked on_toggle/> }
    };

    view! {
        <div class="flex flex-col gap-2">
            <div class="flex gap-1">
                <ContractTypeButton label="Any" selected=contract_type on_select/>
                <ContractTypeButton
                    label="Item exchange"
                    value="item_exchange"
                    selected=contract_type
                    on_select
                />
                <ContractTypeButton
                    label="Auction"
                    value="auction"
                    selected=contract_type
                    on_select
                />
            </div>
            {flag("For sale only", |search| search.only_contracts, |search, on| {
                search.only_contracts = on;
            })}
            {flag(
                "No multi-item contracts",
                |search| search.no_multi_item_contracts,
                |search, on| search.no_multi_item_contracts = on,
            )}
            {flag(
                "Without other items",
                |search| search.without_other_items,
                |search, on| search.without_other_items = on,
            )}
            {flag("Gold bar rolls", |search| search.goldbar, |search, on| search.goldbar = on)}
            {flag("Diamond bar rolls", |search| search.diamondbar, |search, on| {
                search.diamondbar = on;
            })}
            {flag("Brown bar rolls", |search| search.brownbar, |search, on| search.brownbar = on)}
        </div>
    }
}

/// Two optional numeric bounds committed on change, used for the contract
/// price and estimated value filters.
#[component]
fn BoundsInputs(
    lower_placeholder: &'static str,
    upper_placeholder: &'static str,
    initial: Option<(f64, Option<f64>)>,
    #[prop(into)] on_commit: Callback<Option<(f64, Option<f64>)>>,
) -> impl IntoView {
    let lower = RwSignal::new(initial.map(|(lower, _)| lower.to_string()).unwrap_or_default());
    let upper = RwSignal::new(
        initial.and_then(|(_, upper)| upper).map(|upper| upper.to_string()).unwrap_or_default(),
    );

    let commit = move || {
        let bounds = match (lower.get_untracked().parse::<f64>(), upper.get_untracked().parse::<f64>()) {
            (Ok(lower), Ok(upper)) => Some((lower.min(upper), Some(lower.max(upper)))),
            (Ok(lower), Err(_)) => Some((lower, None)),
            // A single maximum matches the legacy single-value price
            // semantics only through both bounds, so keep it as a range
            // from zero.
            (Err(_), Ok(upper)) => Some((0.0, Some(upper))),
            (Err(_), Err(_)) => None,
        };
        on_commit.run(bounds);
    };

    view! {
        <div class="flex gap-2">
            <Input
                class="h-8 text-xs"
                placeholder=lower_placeholder
                bind_value=lower
                attr:inputmode="decimal"
                on:change=move |_| commit()
            />
            <Input
                class="h-8 text-xs"
                placeholder=upper_placeholder
                bind_value=upper
                attr:inputmode="decimal"
                on:change=move |_| commit()
            />
        </div>
    }
}

/// The meta group select over the source module's meta group.
#[component]
fn MetaGroupSelect(#[prop(into)] search: Signal<UiSearch>, go: Callback<UiSearch>) -> impl IntoView {
    /// Meta group ids with their URL slugs, like the legacy select.
    const META_GROUPS: [i64; 6] = [1, 2, 3, 4, 5, 6];

    let current = search.get_untracked().meta_group.unwrap_or_default();

    let selected_label = if current.is_empty() {
        "All meta groups".to_owned()
    } else {
        meta_group_label(&current).to_owned()
    };

    view! {
        <Select
            default_value=selected_label
            on_change=Callback::new(move |value: Option<String>| {
                let mut next = search.get_untracked();
                next.meta_group = value.and_then(|label| {
                    META_GROUPS
                        .into_iter()
                        .map(|id| meta_group_key(Some(id)))
                        .find(|key| meta_group_label(key) == label)
                        .map(str::to_owned)
                });
                go.run(next);
            })
        >
            <SelectTrigger class="w-full">
                <SelectValue placeholder="All meta groups"/>
            </SelectTrigger>
            <SelectContent>
                <SelectGroup aria_label="Meta groups">
                    <SelectOption value="All meta groups">"All meta groups"</SelectOption>
                    {META_GROUPS
                        .into_iter()
                        .map(|id| {
                            let key = meta_group_key(Some(id));
                            let label = meta_group_label(key);
                            view! {
                                <SelectOption value=label>{label}</SelectOption>
                            }
                        })
                        .collect_view()}
                </SelectGroup>
            </SelectContent>
        </Select>
    }
}

fn meta_group_label(key: &str) -> &'static str {
    match key {
        "t1" => "Tech I",
        "t2" => "Tech II",
        "storyline" => "Storyline",
        "faction" => "Faction",
        "officer" => "Officer",
        "deadspace" => "Deadspace",
        _ => "All meta groups",
    }
}
