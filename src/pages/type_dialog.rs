//! The category (type) picker, a one-to-one mirror of the legacy
//! `TypeDialog.vue` + `TypeCategory.vue`: a trigger button showing the
//! selected type and a full-screen dialog with the hardcoded catalog of
//! abyssal categories. Selecting a type navigates with only the legacy
//! `getTypeLink` subset of the current search carried over (contract type
//! and the boolean flags); clicking the active type deselects it.

use leptos::prelude::*;

use crate::modules::view::{UiSearch, build_query_path};

/// One catalog row: the icon asset stem (a type id or a named image from
/// `/img/icons`), the display name, and the size/variant links. Entries
/// without variants link their icon's type id directly.
struct Entry {
    icon: &'static str,
    name: &'static str,
    variants: &'static [(&'static str, i64)],
}

struct Section {
    title: &'static str,
    entries: &'static [Entry],
}

const fn entry(icon: &'static str, name: &'static str) -> Entry {
    Entry { icon, name, variants: &[] }
}

/// The legacy `TypeDialog.vue` catalog: three columns of sections, with the
/// exact EVE type ids the legacy links (single-variant entries use the icon
/// id as the type id, like `getTypeLink(icon_id)`).
const CATALOG: [&[Section]; 3] = [
    &[
        Section {
            title: "Electronic Warfare",
            entries: &[
                entry("47702", "Stasis Webifier"),
                entry("47732", "Warp Scrambler"),
                entry("47736", "Warp Disruptor"),
                entry("56303", "Heavy Warp Scrambler"),
                entry("56304", "Heavy Warp Disruptor"),
            ],
        },
        Section {
            title: "Weapon Upgrades",
            entries: &[
                entry("49722", "Magnetic Field Stabilizer"),
                entry("49726", "Heat Sink"),
                entry("49730", "Gyrostabilizer"),
                entry("49734", "Entropic Radiation Sink"),
                entry("49738", "Ballistic Control System"),
                entry("60482", "Drone Damage Amplifier"),
                entry("56313", "Siege Module"),
                entry("78621", "Vorton Tuning System"),
                entry("60483", "Fighter Support Unit"),
            ],
        },
        Section {
            title: "Mining Lasers",
            entries: &[
                entry("90460", "Mining Laser"),
                entry("90483", "Deep Core Mining Laser"),
                entry("90474", "Modulated Deep Core Miner"),
            ],
        },
        Section {
            title: "Strip Miners",
            entries: &[
                entry("90493", "Strip Miner"),
                entry("90498", "Deep Core Strip Miner"),
                entry("90467", "Modulated Strip Miner"),
                entry("90487", "Modulated Deep Core Strip Miner"),
            ],
        },
    ],
    &[
        Section {
            title: "Shield",
            entries: &[
                Entry {
                    icon: "47781",
                    name: "Shield Booster",
                    variants: &[
                        ("Small", 47781),
                        ("Medium", 47785),
                        ("Large", 47789),
                        ("X-Large", 47793),
                        ("Capital", 56309),
                    ],
                },
                Entry {
                    icon: "47836",
                    name: "Ancillary Shield Booster",
                    variants: &[
                        ("Medium", 47836),
                        ("Large", 47838),
                        ("X-Large", 47840),
                        ("Capital", 56310),
                    ],
                },
                Entry {
                    icon: "47800",
                    name: "Shield Extender",
                    variants: &[("Small", 47800), ("Medium", 47804), ("Large", 47808)],
                },
            ],
        },
        Section {
            title: "Armor",
            entries: &[
                Entry {
                    icon: "47769",
                    name: "Armor Repairer",
                    variants: &[
                        ("Small", 47769),
                        ("Medium", 47773),
                        ("Large", 47777),
                        ("Capital", 56307),
                    ],
                },
                Entry {
                    icon: "47842",
                    name: "Ancillary Armor Repairer",
                    variants: &[
                        ("Small", 47842),
                        ("Medium", 47844),
                        ("Large", 47846),
                        ("Capital", 56308),
                    ],
                },
                Entry {
                    icon: "47812",
                    name: "Armor Plates",
                    variants: &[("Small", 47812), ("Medium", 47817), ("Large", 47820)],
                },
            ],
        },
        Section {
            title: "Propulsion",
            entries: &[
                Entry {
                    icon: "47749",
                    name: "Afterburner",
                    variants: &[
                        ("1mn", 47749),
                        ("10mn", 47753),
                        ("100mn", 47757),
                        ("10000mn", 56305),
                    ],
                },
                Entry {
                    icon: "47408",
                    name: "Microwarpdrive",
                    variants: &[
                        ("5mn", 47740),
                        ("50mn", 47408),
                        ("500mn", 47745),
                        ("50000mn", 56306),
                    ],
                },
            ],
        },
        Section {
            title: "Ice Mining",
            entries: &[entry("90502", "Ice Mining Laser"), entry("90524", "Ice Harvester")],
        },
        Section {
            title: "Gas Harvesting",
            entries: &[
                entry("90529", "Gas Cloud Scoop"),
                entry("90593", "Gas Cloud Harvester"),
            ],
        },
    ],
    &[
        Section {
            title: "Engineering",
            entries: &[
                Entry {
                    icon: "47824",
                    name: "Energy Neutralizer",
                    variants: &[
                        ("Small", 47824),
                        ("Medium", 47828),
                        ("Heavy", 47832),
                        ("Capital", 56312),
                    ],
                },
                Entry {
                    icon: "48419",
                    name: "Energy Nosferatu",
                    variants: &[
                        ("Small", 48419),
                        ("Medium", 48423),
                        ("Heavy", 48427),
                        ("Capital", 56311),
                    ],
                },
                Entry {
                    icon: "48431",
                    name: "Cap Battery",
                    variants: &[("Small", 48431), ("Medium", 48435), ("Large", 48439)],
                },
            ],
        },
        Section {
            title: "Miscellaneous",
            entries: &[
                Entry {
                    icon: "52227",
                    name: "Damage Control",
                    variants: &[("Regular", 52227), ("Assault", 52230)],
                },
                Entry {
                    icon: "SmartbombEM",
                    name: "EMP Smartbombs",
                    variants: &[("Small", 84442), ("Medium", 84438), ("Large", 84434)],
                },
                Entry {
                    icon: "SmartbombKin",
                    name: "Graviton Smartbombs",
                    variants: &[("Small", 84444), ("Medium", 84440), ("Large", 84436)],
                },
                Entry {
                    icon: "SmartbombThermal",
                    name: "Plasma Smartbombs",
                    variants: &[("Small", 84443), ("Medium", 84439), ("Large", 84435)],
                },
                Entry {
                    icon: "SmartbombExplo",
                    name: "Proton Smartbombs",
                    variants: &[("Small", 84445), ("Medium", 84441), ("Large", 84437)],
                },
                Entry {
                    icon: "60479",
                    name: "Drones",
                    variants: &[
                        ("Light", 60478),
                        ("Medium", 60479),
                        ("Heavy", 60480),
                        ("Sentry", 60481),
                    ],
                },
            ],
        },
        Section {
            title: "Mining Drones",
            entries: &[
                entry("90614", "Mining Drone"),
                entry("90618", "Ice Harvesting Drone"),
                entry("90621", "'Excavator' Mining Drone"),
                entry("90622", "'Excavator' Ice Harvesting Drone"),
            ],
        },
    ],
];

fn icon_src(icon: &str) -> String {
    format!("/img/icons/{icon}.png")
}

/// The search carried over when switching types, the exact prop subset of
/// the legacy `TypeCategory.getTypeLink`: contract type and the boolean
/// flags survive, attributes, sort, meta and price bounds reset. Clicking
/// the already-selected type clears the type.
pub fn type_switch_search(current: &UiSearch, current_type_id: Option<i64>, target: i64) -> UiSearch {
    UiSearch {
        type_slug: (current_type_id != Some(target)).then(|| target.to_string()),
        contract_type: current.contract_type.clone(),
        only_contracts: current.only_contracts,
        no_multi_item_contracts: current.no_multi_item_contracts,
        goldbar: current.goldbar,
        brownbar: current.brownbar,
        diamondbar: current.diamondbar,
        ..UiSearch::default()
    }
}

#[cfg(test)]
mod tests {
    use super::type_switch_search;
    use crate::modules::view::{UiAttributeFilter, UiSearch, build_query_path};

    #[test]
    fn switching_types_keeps_only_the_legacy_flag_subset() {
        let current = UiSearch {
            type_slug: Some("47408".to_owned()),
            meta_group: Some("t2".to_owned()),
            attributes: vec![UiAttributeFilter {
                name: "speedfactor".to_owned(),
                lower: 500.0,
                upper: None,
            }],
            sort: Some(("price".to_owned(), true)),
            contract_type: Some("auction".to_owned()),
            price: Some((100.0, None)),
            goldbar: true,
            only_contracts: true,
            ..UiSearch::default()
        };

        // Switching to another type: flags survive, the rest resets.
        let switched = type_switch_search(&current, Some(47408), 47740);
        assert_eq!(
            build_query_path("modules", &switched),
            "/modules/type/47740/auction/contracts-only/goldbar",
        );

        // Clicking the active type deselects it.
        let cleared = type_switch_search(&current, Some(47408), 47408);
        assert_eq!(
            build_query_path("modules", &cleared),
            "/modules/auction/contracts-only/goldbar",
        );
    }
}

#[component]
fn TypeLink(
    prefix: &'static str,
    #[prop(into)] search: Signal<UiSearch>,
    current_type_id: Option<i64>,
    type_id: i64,
    open: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    // Reactive so the carried-over flags stay current even though the panel
    // (and this link) persists across filter navigations.
    let href = move || {
        build_query_path(
            prefix,
            &type_switch_search(&search.get(), current_type_id, type_id),
        )
    };
    let active = current_type_id == Some(type_id);

    view! {
        <a
            class="flex items-center gap-2 p-1 text-muted-foreground transition-colors duration-150 hover:text-white"
            href=href
            on:click=move |_| open.set(false)
        >
            <div class=if active {
                "grid size-5 place-items-center rounded border border-primary bg-primary text-white"
            } else {
                "grid size-5 place-items-center rounded border border-border text-white *:opacity-0"
            }>
                <span aria-hidden="true">{"\u{2713}"}</span>
            </div>
            {children()}
        </a>
    }
}

/// The trigger + dialog. `current_type_id`/`current_type_name` describe the
/// resolved selected type, when there is one.
#[component]
pub fn TypeDialog(
    prefix: &'static str,
    #[prop(into)] search: Signal<UiSearch>,
    #[prop(optional)] current_type_id: Option<i64>,
    #[prop(optional)] current_type_name: Option<String>,
) -> impl IntoView {
    let open = RwSignal::new(false);

    // The trigger label strips the mutation words, like the legacy dialog.
    let label = current_type_name
        .as_ref()
        .map(|name| name.replace("Abyssal", "").replace("Mutated", "").trim().to_owned());
    let trigger_icon = current_type_id.map(|type_id| {
        let src = format!("https://images.evetech.net/types/{type_id}/icon?size=64");
        view! { <img alt="" class="size-6 rounded-sm" src=src/> }
    });

    let render_catalog = move || CATALOG
        .iter()
        .map(|sections| {
            let sections = sections
                .iter()
                .map(|section| {
                    let entries = section
                        .entries
                        .iter()
                        .map(|entry| {
                            let icon = icon_src(entry.icon);

                            if entry.variants.is_empty() {
                                let type_id: i64 = entry
                                    .icon
                                    .parse()
                                    .expect("single-variant catalog icons are type ids");
                                view! {
                                    <TypeLink prefix search current_type_id type_id open>
                                        <img alt=entry.name class="h-6 w-6" src=icon/>
                                        <span class="text-sm">{entry.name}</span>
                                    </TypeLink>
                                }
                                .into_any()
                            } else {
                                let variants = entry
                                    .variants
                                    .iter()
                                    .map(|&(variant, type_id)| {
                                        view! {
                                            <TypeLink prefix search current_type_id type_id open>
                                                <small>{variant}</small>
                                            </TypeLink>
                                        }
                                    })
                                    .collect_view();

                                view! {
                                    <div class="flex items-center gap-2">
                                        <img alt=entry.name class="size-8 rounded-lg" src=icon/>
                                        <div>
                                            <h1 class="mb-1 text-sm font-medium">{entry.name}</h1>
                                            <div class="flex flex-wrap gap-2">{variants}</div>
                                        </div>
                                    </div>
                                }
                                .into_any()
                            }
                        })
                        .collect_view();

                    view! {
                        <div class="grid gap-1 p-4">
                            <h3 class="mb-2 text-lg text-primary">{section.title}</h3>
                            {entries}
                        </div>
                    }
                })
                .collect_view();

            view! { <div>{sections}</div> }
        })
        .collect_view();

    // Legacy quirk kept on purpose: the string replace leaves a double
    // space ("50MN  Microwarpdrive"); HTML rendering collapses it.
    view! {
        <button
            class="flex h-10 w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-border bg-card-2 px-3 py-2 text-start text-sm transition hover:brightness-125"
            on:click=move |_| open.set(true)
        >
            {trigger_icon}
            <span class="truncate">{label.unwrap_or_else(|| "All".to_owned())}</span>
            <span aria-hidden="true" class="ml-auto shrink-0 opacity-50">{"\u{25BE}"}</span>
        </button>
        {move || {
            open.get().then(|| {
                view! {
                    <div
                        class="fixed inset-0 z-50 overflow-y-auto bg-black/80 p-4"
                        on:click=move |_| open.set(false)
                    >
                        <div
                            class="mx-auto max-w-[1440px] rounded-lg border border-border bg-card"
                            on:click=|event| event.stop_propagation()
                        >
                            <div class="grid gap-[1px] md:grid-cols-2 xl:grid-cols-3">
                                {render_catalog()}
                            </div>
                        </div>
                    </div>
                }
            })
        }}
    }
}
